//! Instant replay.
//!
//! GameHub keeps a rolling buffer of the last minutes by having ffmpeg write
//! short numbered segments into a scratch folder and letting the old ones be
//! overwritten. Pressing the save hotkey stitches the segments that cover the
//! requested window into one clip. Nothing has to be hooked into the game, and
//! nothing large is ever held in memory.
//!
//! Sound comes from two places, both optional and both on by default where
//! they exist: **system audio** through WASAPI loopback (see `audio.rs`), piped
//! into ffmpeg as raw PCM, and a **microphone** through DirectShow if one is
//! chosen. The two are mixed into one track.
//!
//! Honest limits, all reported to the user rather than discovered later:
//!
//! * **Exclusive fullscreen cannot be captured.** `gdigrab` reads the desktop
//!   composition, and a game that owns the display outright is not part of it.
//!   Borderless windowed works, and every modern game offers it.
//! * **It costs CPU** unless the graphics card's encoder is used, which it is
//!   whenever one works.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use serde::{Deserialize, Serialize};

use crate::audio::{PcmFormat, SystemAudio};

/// Each segment is short enough that the saved clip starts close to where the
/// user wanted, and long enough that ffmpeg is not constantly opening files.
pub const SEGMENT_SECONDS: u64 = 5;

/// The shortest and longest buffer the user can ask for. Ten minutes at 1080p
/// medium quality is about 450 MB of scratch, which the Replay page shows
/// before it is chosen.
pub const MIN_BUFFER_SECONDS: u64 = 30;
pub const MAX_BUFFER_SECONDS: u64 = 600;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReplaySettings {
    pub enabled: bool,
    /// How much history to keep, in seconds. 30–600.
    pub buffer_seconds: u64,
    pub fps: u32,
    /// "low", "medium" or "high" — maps to an encoder preset and bitrate.
    pub quality: String,
    pub monitor: usize,
    /// Record what comes out of the speakers, through WASAPI loopback. On by
    /// default: this is the thing that was missing for a whole release.
    pub system_audio: bool,
    /// A DirectShow microphone name, or empty for none. Mixed in with the
    /// system audio when both are on.
    pub audio_device: String,
    /// Where finished clips go. Empty means the default under app data.
    pub folder: String,
    /// Record at this height instead of the desktop's own, keeping the aspect
    /// ratio. 0 means native. Scaling down is the single biggest thing that
    /// stops recording from stealing frames from the game.
    pub scale_height: u32,
    /// How many seconds the hotkey saves. 30 by default; the buffer can be
    /// longer, and the user picks how much of it a press actually keeps.
    pub save_seconds: u64,
    /// "auto", "cpu", or a specific ffmpeg encoder name.
    pub encoder: String,
}

impl Default for ReplaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_seconds: 120,
            fps: 30,
            quality: "medium".into(),
            monitor: 0,
            system_audio: true,
            audio_device: String::new(),
            folder: String::new(),
            save_seconds: 30,
            scale_height: 1080,
            encoder: "auto".into(),
        }
    }
}

impl ReplaySettings {
    /// The buffer length the recorder will actually use.
    pub fn buffer(&self) -> u64 {
        self.buffer_seconds.clamp(MIN_BUFFER_SECONDS, MAX_BUFFER_SECONDS)
    }

    /// Whether a microphone is part of the recording.
    pub fn wants_microphone(&self) -> bool {
        let device = self.audio_device.trim();
        !device.is_empty() && device != "none"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: String,
    pub path: String,
    pub game_id: Option<String>,
    pub game_name: String,
    pub recorded_at: String,
    pub seconds: u64,
    pub size_bytes: u64,
    pub favorite: bool,
    /// Whether the clip has a sound track — false for clips from before
    /// system audio existed, or made with every audio source switched off.
    #[serde(default)]
    pub has_audio: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClipLibrary {
    pub clips: Vec<Clip>,
}

/// How many segment files the ring needs to hold the requested window, with one
/// spare so the segment currently being written is never the one we read.
pub fn segment_count(buffer_seconds: u64) -> u64 {
    (buffer_seconds.clamp(MIN_BUFFER_SECONDS, MAX_BUFFER_SECONDS) / SEGMENT_SECONDS) + 2
}

/// Video bitrate in megabits per second for a quality preset.
pub fn bitrate_mbit(quality: &str) -> u64 {
    match quality {
        "low" => 3,
        "high" => 12,
        _ => 6,
    }
}

/// Rough disk cost of the buffer, so the length slider can say what it costs.
pub fn buffer_estimate_mb(settings: &ReplaySettings) -> u64 {
    // Video plus ~0.2 Mbit of audio, plus the two spare segments.
    let seconds = (segment_count(settings.buffer_seconds)) * SEGMENT_SECONDS;
    seconds * (bitrate_mbit(&settings.quality) * 10 + 2) / 80
}

fn quality_args(quality: &str) -> (&'static str, &'static str) {
    match quality {
        "low" => ("ultrafast", "3M"),
        "high" => ("faster", "12M"),
        _ => ("veryfast", "6M"),
    }
}


/// Hardware encoders, best first.
///
/// These run on the graphics card's dedicated video engine, which is idle while
/// you play. Encoding there instead of on the CPU is the difference between
/// Replay costing a few percent and costing enough to be felt in a game.
const HARDWARE_ENCODERS: &[&str] = &["h264_nvenc", "h264_qsv", "h264_amf"];

/// Reads the encoder list out of `ffmpeg -encoders` output.
///
/// Pure so it can be tested against real ffmpeg output without running ffmpeg:
/// picking an encoder that is not compiled in produces a recorder that fails
/// the moment it starts, which the user would only discover as a missing clip.
pub fn parse_encoders(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            // " V....D h264_nvenc           NVIDIA NVENC H.264 encoder"
            let trimmed = line.trim_start();
            let flags = trimmed.split_whitespace().next()?;
            if !flags.starts_with('V') || flags.len() < 6 {
                return None;
            }
            trimmed.split_whitespace().nth(1).map(str::to_string)
        })
        .collect()
}

/// Chooses the encoder to record with, and the arguments that go with it.
///
/// `preference` is the user's setting: "auto" takes the best hardware encoder
/// present, "cpu" forces libx264, and anything else is used as given if ffmpeg
/// actually has it.
pub fn pick_encoder(preference: &str, available: &[String], preset: &str) -> (String, Vec<String>) {
    let has = |name: &str| available.iter().any(|e| e == name);

    let chosen = match preference {
        "cpu" => "libx264".to_string(),
        "auto" | "" => HARDWARE_ENCODERS
            .iter()
            .find(|name| has(name))
            .map(|name| name.to_string())
            .unwrap_or_else(|| "libx264".to_string()),
        named if has(named) => named.to_string(),
        // Asked for something this ffmpeg cannot do. Recording on the CPU is a
        // worse experience than recording on the GPU, and a far better one than
        // not recording at all.
        _ => "libx264".to_string(),
    };

    // Each encoder names its speed control differently; using the wrong one is
    // an immediate failure to start.
    let args = match chosen.as_str() {
        "h264_nvenc" => vec!["-preset".into(), "p1".into(), "-tune".into(), "ll".into()],
        "h264_qsv" => vec!["-preset".into(), "veryfast".into()],
        "h264_amf" => vec!["-quality".into(), "speed".into()],
        _ => vec!["-preset".into(), preset.to_string()],
    };

    (chosen, args)
}

/// Names that mean "whatever is coming out of the speakers", across the
/// languages and vendors Windows ships them under.
///
/// Windows has no loopback capture device of its own that ffmpeg can open, so
/// recording game sound depends on one the sound card exposes — usually
/// "Stereo Mix", disabled by default — or a virtual cable the user installed.
/// Matching on the name is how the right one gets chosen without asking
/// somebody to type it exactly.
const LOOPBACK_NAMES: &[&str] = &[
    "stereo mix",
    "stereomix",
    "stereo-mix",
    "what u hear",
    "what you hear",
    "wave out",
    "loopback",
    "cable output",     // VB-Audio Virtual Cable
    "voicemeeter out",
    "virtual-audio-capturer",
    "miks",             // Norwegian
    "stereomiks",
    "summe",            // German "Stereomix" variants
];

/// Reads device names out of `ffmpeg -list_devices true -f dshow -i dummy`.
///
/// ffmpeg writes the list to stderr and then exits with an error, which is
/// normal and not a failure. Lines look like:
///
/// ```text
/// [dshow @ 0000] "Stereo Mix (Realtek(R) Audio)" (audio)
/// [dshow @ 0000]   Alternative name "@device_cm_{...}"
/// ```
///
/// Pure, so the parsing can be tested without a Windows machine.
pub fn parse_audio_devices(output: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in output.lines() {
        // Only the friendly name, never the "Alternative name" line — that one
        // is a GUID nobody can read, and either works as an input.
        if line.contains("Alternative name") {
            continue;
        }
        if !line.contains("(audio)") {
            continue;
        }
        let Some(start) = line.find('"') else { continue };
        let rest = &line[start + 1..];
        let Some(end) = rest.find('"') else { continue };
        let name = rest[..end].trim();
        if !name.is_empty() && !found.iter().any(|f: &String| f == name) {
            found.push(name.to_string());
        }
    }
    found
}

/// The device most likely to capture what you hear, if any.
pub fn pick_loopback(devices: &[String]) -> Option<String> {
    devices
        .iter()
        .find(|name| {
            let lower = name.to_lowercase();
            LOOPBACK_NAMES.iter().any(|candidate| lower.contains(candidate))
        })
        .cloned()
}

/// The DirectShow audio devices this machine offers.
pub fn audio_devices(ffmpeg: &Path) -> Vec<String> {
    let mut command = Command::new(ffmpeg);
    command.args([
        "-hide_banner",
        "-list_devices",
        "true",
        "-f",
        "dshow",
        "-i",
        "dummy",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    match command.output() {
        // A non-zero exit is expected here: listing devices is not a real run.
        Ok(output) => parse_audio_devices(&String::from_utf8_lossy(&output.stderr)),
        Err(_) => Vec::new(),
    }
}

/// Encoders this ffmpeg was *built* with. Not the same as ones that work.
fn compiled_encoders(ffmpeg: &Path) -> Vec<String> {
    let mut command = Command::new(ffmpeg);
    command.args(["-hide_banner", "-encoders"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    match command.output() {
        Ok(output) => parse_encoders(&String::from_utf8_lossy(&output.stdout)),
        Err(_) => Vec::new(),
    }
}

/// Actually encodes one frame with `name` and reports whether it worked.
///
/// This exists because `ffmpeg -encoders` lists what the build supports, not
/// what this machine can run. A Windows ffmpeg build lists `h264_nvenc` on a
/// laptop with no NVIDIA card at all; choosing it from that list starts a
/// recorder that dies instantly with "Error while opening encoder", and the
/// only symptom the user ever sees is an empty buffer. Trying it for real is
/// the only honest test.
pub fn encoder_works(ffmpeg: &Path, name: &str) -> bool {
    let mut command = Command::new(ffmpeg);
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=c=black:s=320x240:r=10:d=0.2",
        "-c:v",
        name,
        "-f",
        "null",
        "-",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    match command.output() {
        // ffmpeg can exit 0 while having written nothing, so the stderr is
        // checked too rather than trusting the status alone.
        Ok(output) => {
            let complained = String::from_utf8_lossy(&output.stderr).to_lowercase();
            output.status.success()
                && !complained.contains("error while opening encoder")
                && !complained.contains("no packets")
                && !complained.contains("not permitted")
        }
        Err(_) => false,
    }
}

/// The encoders that both exist and run on this machine, best first.
///
/// libx264 is always included last: it is software, so if ffmpeg runs at all it
/// can encode, and having something to fall back to matters more than speed.
pub fn available_encoders(ffmpeg: &Path) -> Vec<String> {
    let compiled = compiled_encoders(ffmpeg);
    let mut working: Vec<String> = HARDWARE_ENCODERS
        .iter()
        .filter(|name| compiled.iter().any(|c| c == *name))
        .filter(|name| encoder_works(ffmpeg, name))
        .map(|name| name.to_string())
        .collect();
    working.push("libx264".into());
    working
}


/// Which sound sources a recording actually has, once the machine has been
/// asked. Decided before ffmpeg starts, and reported on the Replay page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioPlan {
    /// The loopback capture's format, when system audio is on and opened.
    pub system: Option<PcmFormat>,
    pub microphone: bool,
}

impl AudioPlan {
    pub const NONE: AudioPlan = AudioPlan { system: None, microphone: false };

    pub fn has_audio(&self) -> bool {
        self.system.is_some() || self.microphone
    }
}

/// Builds the ffmpeg command that fills the ring buffer.
///
/// Split out from running it so the argument list can be asserted on and run
/// for real in tests: a typo here is a recorder that silently produces nothing.
///
/// Inputs are numbered in a fixed order — 0 is always the desktop, 1 is the
/// system-audio pipe when present, and the microphone is whichever comes next —
/// and every output stream is mapped explicitly. Leaving ffmpeg to pick "the
/// best audio" from two inputs means it drops one of them without a word.
pub fn buffer_args(
    settings: &ReplaySettings,
    scratch: &Path,
    encoders: &[String],
    audio: AudioPlan,
) -> Vec<String> {
    let (preset, bitrate) = quality_args(&settings.quality);
    let (encoder, encoder_args) = pick_encoder(&settings.encoder, encoders, preset);
    let pattern = scratch.join("seg_%04d.mp4");

    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        // Desktop capture. gdigrab is available in every ffmpeg build for
        // Windows and needs no extra components installed.
        "-f".into(),
        "gdigrab".into(),
        "-framerate".into(),
        settings.fps.to_string(),
        "-i".into(),
        "desktop".into(),
    ];

    // Audio inputs, in a fixed order so the map arguments below are right.
    let mut audio_inputs: Vec<String> = Vec::new();
    if let Some(format) = audio.system {
        args.extend(format.ffmpeg_args());
        audio_inputs.push(format!("{}:a", audio_inputs.len() + 1));
    }
    if audio.microphone {
        args.extend([
            "-thread_queue_size".into(),
            "1024".into(),
            "-f".into(),
            "dshow".into(),
            "-i".into(),
            format!("audio={}", settings.audio_device.trim()),
        ]);
        audio_inputs.push(format!("{}:a", audio_inputs.len() + 1));
    }

    // Scale before encoding, not after: the point is to give the encoder fewer
    // pixels. -2 keeps the aspect ratio and lands on an even width, which H.264
    // requires. The backslash before the comma is required, not cosmetic: a
    // bare comma separates filters, so `min(1080,ih)` parses as the end of
    // `scale` followed by a filter called `ih)`.
    let scale = (settings.scale_height > 0).then(|| format!("scale=-2:min({}\\,ih)", settings.scale_height));

    match audio_inputs.len() {
        0 => {
            if let Some(scale) = &scale {
                args.extend(["-vf".into(), scale.clone()]);
            }
            args.extend(["-map".into(), "0:v".into()]);
        }
        1 => {
            if let Some(scale) = &scale {
                args.extend(["-vf".into(), scale.clone()]);
            }
            args.extend(["-map".into(), "0:v".into(), "-map".into(), audio_inputs[0].clone()]);
        }
        _ => {
            // Two sound sources become one track. -vf cannot be combined with a
            // complex graph on the same output, so the scale goes in here too.
            let video = match &scale {
                Some(scale) => format!("[0:v]{scale}[v];"),
                None => "[0:v]null[v];".to_string(),
            };
            let mix = format!(
                "{video}[{}][{}]amix=inputs=2:duration=first:dropout_transition=0[a]",
                audio_inputs[0], audio_inputs[1]
            );
            args.extend([
                "-filter_complex".into(),
                mix,
                "-map".into(),
                "[v]".into(),
                "-map".into(),
                "[a]".into(),
            ]);
        }
    }

    if audio.has_audio() {
        args.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "160k".into()]);
    }

    args.extend(["-c:v".into(), encoder]);
    args.extend(encoder_args);
    args.extend([
        "-b:v".into(),
        bitrate.into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        // A keyframe at every segment boundary, so a clip can start at one.
        "-g".into(),
        (settings.fps as u64 * SEGMENT_SECONDS).to_string(),
        "-f".into(),
        "segment".into(),
        "-segment_time".into(),
        SEGMENT_SECONDS.to_string(),
        "-segment_wrap".into(),
        segment_count(settings.buffer_seconds).to_string(),
        "-reset_timestamps".into(),
        "1".into(),
        "-y".into(),
        pattern.to_string_lossy().into_owned(),
    ]);

    args
}


/// The segments that cover the last `seconds`, oldest first.
///
/// Selection is by modification time rather than by file name, because the ring
/// wraps: `seg_0003.mp4` may well be newer than `seg_0011.mp4`. The segment
/// still being written is skipped — it has no moov atom yet and would break the
/// concat.
pub fn segments_for(scratch: &Path, seconds: u64, now: std::time::SystemTime) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(scratch) else {
        return Vec::new();
    };

    let mut segments: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("mp4"))
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| (t, e.path())))
        .collect();

    segments.sort_by_key(|(time, _)| *time);

    // The newest file is the one ffmpeg currently has open.
    segments.pop();

    let wanted = (seconds / SEGMENT_SECONDS).max(1) as usize;
    let start = segments.len().saturating_sub(wanted);
    segments[start..]
        .iter()
        .filter(|(time, _)| {
            now.duration_since(*time)
                .map(|age| age.as_secs() <= seconds + SEGMENT_SECONDS * 3)
                .unwrap_or(true)
        })
        .map(|(_, path)| path.clone())
        .collect()
}

/// A clip file name that sorts by time and is legal on Windows.
pub fn clip_name(game_name: &str, recorded_at: &str) -> String {
    gamehub_detect::media::file_name(game_name, recorded_at).replace(".png", ".mp4")
}

/// The concat list ffmpeg reads. Paths are quoted the way the demuxer expects.
pub fn concat_list(segments: &[PathBuf]) -> String {
    segments
        .iter()
        .map(|p| format!("file '{}'", p.to_string_lossy().replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Locates ffmpeg: the copy shipped beside GameHub first, then the one on PATH.
pub fn find_ffmpeg(resource_dir: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    // The bundle copies `resources/*` verbatim, so the binary lands one folder
    // down; the flat path is checked too in case that ever changes.
    for candidate in [resource_dir.join("resources").join(name), resource_dir.join(name)] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).map(|dir| dir.join(name)).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .find(|candidate| candidate.is_file())
}


/// What the recorder is doing right now, for the Replay page.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AudioReport {
    /// True when the system-audio loopback is feeding the recording.
    pub system_audio: bool,
    /// The output device being listened to, when it is.
    pub system_device: Option<String>,
    pub microphone: bool,
    /// Why system audio is off although it was asked for.
    pub note: Option<String>,
}

/// The running buffer.
pub struct Recorder {
    child: Option<Child>,
    pub scratch: PathBuf,
    /// Whatever ffmpeg last wrote to stderr, so a failure can be explained
    /// rather than merely noticed.
    complaints: Arc<Mutex<String>>,
    system_audio: Option<SystemAudio>,
    pump: Option<std::thread::JoinHandle<()>>,
    /// What sound the recording has, decided when it started.
    pub audio: AudioReport,
    /// When the buffer started filling, so the page can say how much is in it.
    started_at: Option<std::time::Instant>,
}

impl Recorder {
    pub fn new(scratch: PathBuf) -> Self {
        Self {
            child: None,
            scratch,
            complaints: Arc::new(Mutex::new(String::new())),
            system_audio: None,
            pump: None,
            audio: AudioReport::default(),
            started_at: None,
        }
    }

    pub fn is_running(&mut self) -> bool {
        match self.child.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// Seconds of recording that exist right now, capped at the buffer.
    pub fn seconds_buffered(&self, settings: &ReplaySettings) -> u64 {
        self.started_at
            .map(|t| t.elapsed().as_secs().min(settings.buffer()))
            .unwrap_or(0)
    }

    pub fn start(&mut self, ffmpeg: &Path, settings: &ReplaySettings) -> Result<(), String> {
        self.stop();
        std::fs::create_dir_all(&self.scratch)
            .map_err(|e| crate::msg::code("buffer_folder", &[&e.to_string()]))?;
        // Old segments from a previous session would otherwise be stitched into
        // the first clip of this one.
        clear_scratch(&self.scratch);

        // Asked once per start rather than per frame: the answer cannot change
        // while ffmpeg is running.
        let encoders = available_encoders(ffmpeg);

        // System audio is opened *before* ffmpeg, because ffmpeg has to be told
        // the exact sample rate and channel count of what it will be fed.
        let mut report = AudioReport { microphone: settings.wants_microphone(), ..Default::default() };
        let system = if settings.system_audio {
            match SystemAudio::open() {
                Ok(audio) => {
                    report.system_audio = true;
                    report.system_device = Some(audio.device_name.clone());
                    Some(audio)
                }
                Err(why) => {
                    // Recording without sound beats not recording — but the
                    // page says so, in the words the backend gave.
                    report.note = Some(why);
                    None
                }
            }
        } else {
            None
        };
        let plan = AudioPlan {
            system: system.as_ref().map(|a| a.format),
            microphone: settings.wants_microphone(),
        };

        let mut command = Command::new(ffmpeg);
        command.args(buffer_args(settings, &self.scratch, &encoders, plan));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW, so no console flashes up behind the game, and
            // BELOW_NORMAL_PRIORITY_CLASS, so when the game and the recorder
            // both want the CPU, Windows gives it to the game.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
            command.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
        }

        let mut child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| crate::msg::code("ffmpeg_start", &[&e.to_string()]))?;

        // Whatever ffmpeg complains about is kept, so a recorder that dies can
        // say why. Without this the only symptom is an empty buffer minutes
        // later, which describes the consequence and hides the cause.
        let complaints = Arc::new(Mutex::new(String::new()));
        if let Some(stderr) = child.stderr.take() {
            let sink = complaints.clone();
            std::thread::spawn(move || {
                use std::io::{BufRead, BufReader};
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    let mut held = sink.lock();
                    if held.len() < 2000 {
                        held.push_str(&line);
                        held.push('\n');
                    }
                }
            });
        }
        self.complaints = complaints;

        // The pipe is handed to the audio pump, which keeps it fed from here
        // on. Without system audio the pipe is simply closed.
        let stdin = child.stdin.take();
        match (&system, stdin) {
            (Some(audio), Some(stdin)) => self.pump = Some(audio.pump_into(stdin)),
            (_, stdin) => drop(stdin),
        }

        // ffmpeg fails immediately when it fails at all — a bad encoder, a
        // filter that will not parse, a capture method that is unavailable. A
        // moment's wait here turns a silent nothing into a message the user
        // gets the instant they switch Replay on.
        std::thread::sleep(Duration::from_millis(1200));
        if let Ok(Some(_)) = child.try_wait() {
            let why = self.complaints.lock().trim().to_string();
            let last = why.lines().last().unwrap_or("").trim().to_string();
            self.system_audio = None;
            return Err(if last.is_empty() {
                crate::msg::plain("replay_start_silent")
            } else {
                crate::msg::code("replay_start_failed", &[&last])
            });
        }

        self.system_audio = system;
        self.audio = report;
        self.child = Some(child);
        self.started_at = Some(std::time::Instant::now());
        Ok(())
    }

    /// What ffmpeg has complained about, if anything — plus a dead audio
    /// capture, which is the other thing that can go quietly wrong.
    pub fn last_complaint(&self) -> String {
        if let Some(why) = self.system_audio.as_ref().and_then(|a| a.failure()) {
            return why;
        }
        self.complaints.lock().trim().to_string()
    }

    pub fn stop(&mut self) {
        if let Some(mut audio) = self.system_audio.take() {
            audio.stop();
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(pump) = self.pump.take() {
            let _ = pump.join();
        }
        self.audio = AudioReport::default();
        self.started_at = None;
        clear_scratch(&self.scratch);
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop();
    }
}


fn clear_scratch(scratch: &Path) {
    if let Ok(entries) = std::fs::read_dir(scratch) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|x| x.to_str()) == Some("mp4") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// The combined length of the segments, in seconds.
///
/// Measured one file at a time because ffprobe reports a duration of zero for a
/// concat list. Returns None if anything cannot be measured, and the caller
/// then saves the whole buffer rather than failing — a clip a few seconds
/// longer than asked for is a far better outcome than an error.
fn total_duration(ffmpeg: &Path, segments: &[PathBuf]) -> Option<f64> {
    let probe = ffprobe_beside(ffmpeg)?;
    let mut total = 0.0;
    for segment in segments {
        let mut command = Command::new(&probe);
        command.args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            &segment.to_string_lossy(),
        ]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let output = command.output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        total += text.trim().parse::<f64>().ok()?;
    }
    (total > 0.0).then_some(total)
}

/// ffprobe ships alongside ffmpeg, so it is looked for in the same folder.
fn ffprobe_beside(ffmpeg: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };
    let candidate = ffmpeg.parent()?.join(name);
    candidate.is_file().then_some(candidate)
}

/// Stitches the last `seconds` into one file.
pub fn save_clip(
    ffmpeg: &Path,
    scratch: &Path,
    destination: &Path,
    seconds: u64,
) -> Result<u64, String> {
    let segments = segments_for(scratch, seconds, std::time::SystemTime::now());
    if segments.is_empty() {
        return Err(crate::msg::plain("buffer_empty"));
    }

    let list_path = scratch.join("concat.txt");
    std::fs::write(&list_path, concat_list(&segments))
        .map_err(|e| crate::msg::code("clip_list", &[&e.to_string()]))?;

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| crate::msg::code("clip_folder", &[&e.to_string()]))?;
    }

    // How far into the stitched stream to start, so the clip is the last
    // `seconds` rather than everything the buffer happened to be holding.
    //
    // This used to be `-sseof -N` placed after `-i`, which is two mistakes at
    // once: an input option written in the output position, which ffmpeg
    // rejects outright with "Error opening output files: Invalid argument" —
    // and even in the right position, the concat demuxer ignores -sseof and
    // returns the whole buffer. Seeking with -ss before -i does work, but needs
    // a real duration, which is why the segments are measured first.
    let total = total_duration(ffmpeg, &segments);
    let offset = total.map(|t| (t - seconds as f64).max(0.0));

    let mut command = Command::new(ffmpeg);
    command.args(["-hide_banner", "-loglevel", "error"]);
    if let Some(offset) = offset {
        if offset > 0.5 {
            command.args(["-ss", &format!("{offset:.3}")]);
        }
    }
    command.args([
        "-f",
        "concat",
        "-safe",
        "0",
        "-i",
        &list_path.to_string_lossy(),
        "-c",
        "copy",
        "-y",
        &destination.to_string_lossy(),
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let output = command
        .output()
        .map_err(|e| crate::msg::code("ffmpeg_start", &[&e.to_string()]))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(crate::msg::code("clip_save", &[detail.lines().last().unwrap_or("?")]));
    }

    std::fs::metadata(destination)
        .map(|m| m.len())
        .map_err(|e| crate::msg::code("clip_not_written", &[&e.to_string()]))
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn ffmpeg_is_found_in_the_bundle_before_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        let nested = tmp.path().join("resources");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join(name), b"binary").unwrap();
        assert_eq!(find_ffmpeg(tmp.path()), Some(nested.join(name)));
    }

    #[test]
    fn a_missing_ffmpeg_is_reported_rather_than_assumed() {
        let tmp = tempfile::tempdir().unwrap();
        // Nothing bundled; whether PATH has one is the machine's business, but
        // the bundled lookup must not invent a path that does not exist.
        if let Some(found) = find_ffmpeg(tmp.path()) {
            assert!(found.is_file(), "only a real file is ever returned");
        }
    }

    #[test]
    fn the_ring_holds_the_requested_window_plus_a_spare() {
        assert_eq!(segment_count(30), 8, "30s of 5s segments, plus two spare");
        assert_eq!(segment_count(90), 20);
        // Out-of-range values are clamped rather than producing a silly ring.
        assert_eq!(segment_count(1), segment_count(MIN_BUFFER_SECONDS));
        assert_eq!(segment_count(99_999), segment_count(MAX_BUFFER_SECONDS));
    }

    #[test]
    fn the_buffer_command_captures_the_desktop_into_a_wrapping_segment_ring() {
        let settings = ReplaySettings { buffer_seconds: 60, fps: 60, ..Default::default() };
        let args = buffer_args(&settings, Path::new("/scratch"), &[], AudioPlan::NONE);
        let joined = args.join(" ");
        assert!(joined.contains("-f gdigrab"));
        assert!(joined.contains("-framerate 60"));
        assert!(joined.contains("-f segment"));
        assert!(joined.contains("-segment_wrap 14"));
        assert!(joined.contains("seg_%04d.mp4"));
        assert!(!joined.contains("dshow"), "no audio device means no audio input");
    }

    #[test]
    fn quality_changes_the_preset_and_bitrate() {
        let low = buffer_args(&ReplaySettings { quality: "low".into(), ..Default::default() }, Path::new("/s"), &[], AudioPlan::NONE).join(" ");
        let high = buffer_args(&ReplaySettings { quality: "high".into(), ..Default::default() }, Path::new("/s"), &[], AudioPlan::NONE).join(" ");
        assert!(low.contains("ultrafast") && low.contains("3M"));
        assert!(high.contains("faster") && high.contains("12M"));
    }

    /// Writes fake segments with controlled modification times.
    fn segment(dir: &Path, name: &str, age: Duration) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"segment").unwrap();
        let when = SystemTime::now() - age;
        filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(when)).unwrap();
        path
    }

    #[test]
    fn segments_are_chosen_by_age_because_the_ring_wraps() {
        let tmp = tempfile::tempdir().unwrap();
        // seg_0003 is the newest despite its low number — the ring has wrapped.
        segment(tmp.path(), "seg_0009.mp4", Duration::from_secs(30));
        segment(tmp.path(), "seg_0010.mp4", Duration::from_secs(25));
        segment(tmp.path(), "seg_0011.mp4", Duration::from_secs(20));
        segment(tmp.path(), "seg_0001.mp4", Duration::from_secs(15));
        segment(tmp.path(), "seg_0002.mp4", Duration::from_secs(10));
        segment(tmp.path(), "seg_0003.mp4", Duration::from_secs(2));

        let chosen = segments_for(tmp.path(), 30, SystemTime::now());
        let names: Vec<String> = chosen
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();

        assert!(!names.contains(&"seg_0003.mp4".to_string()), "the segment being written is skipped");
        // Thirty seconds wants six segments; only five are complete, so all
        // five are taken — a short buffer gives a short clip, not an error.
        assert_eq!(names.len(), 5);
        assert_eq!(names.first().unwrap(), "seg_0009.mp4", "oldest first, in age order not name order");
        assert_eq!(names.last().unwrap(), "seg_0002.mp4");
    }

    #[test]
    fn a_shorter_window_takes_fewer_segments() {
        let tmp = tempfile::tempdir().unwrap();
        for (index, age) in [30, 25, 20, 15, 10, 2].iter().enumerate() {
            segment(tmp.path(), &format!("seg_{index:04}.mp4"), Duration::from_secs(*age));
        }
        assert_eq!(segments_for(tmp.path(), 15, SystemTime::now()).len(), 3);
        assert_eq!(segments_for(tmp.path(), 30, SystemTime::now()).len(), 5);
    }

    #[test]
    fn an_empty_buffer_selects_nothing_rather_than_erroring() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(segments_for(tmp.path(), 60, SystemTime::now()).is_empty());
    }

    #[test]
    fn the_concat_list_quotes_paths_ffmpeg_would_otherwise_split() {
        let list = concat_list(&[PathBuf::from("/tmp/it's here/seg_0001.mp4")]);
        assert!(list.starts_with("file '"));
        assert!(list.contains("'\\''"), "an apostrophe is escaped for the concat demuxer");
    }

    #[test]
    fn clip_names_are_mp4_and_safe_on_windows() {
        let name = clip_name("Grand Theft Auto V: Enhanced?", "2026-08-30T16:42:00Z");
        assert!(name.ends_with(".mp4"));
        for bad in [':', '?', '/', '\\'] {
            assert!(!name.contains(bad));
        }
    }
}

#[cfg(test)]
mod encoder_tests {
    use super::*;

    /// Real `ffmpeg -encoders` output, trimmed. Parsing this wrong means either
    /// missing the graphics card's encoder or picking something that does not
    /// exist, and both only show up as a broken recorder on the user's machine.
    const SAMPLE: &str = "\
Encoders:
 V..... = Video
 ------
 V....D libx264              libx264 H.264 / AVC
 V....D h264_nvenc           NVIDIA NVENC H.264 encoder
 V....D h264_qsv             H.264 QSV encoder
 A....D aac                  AAC encoder
 S....D srt                  SubRip subtitle
";

    #[test]
    fn only_video_encoders_are_picked_up() {
        let found = parse_encoders(SAMPLE);
        assert!(found.contains(&"libx264".to_string()));
        assert!(found.contains(&"h264_nvenc".to_string()));
        assert!(found.contains(&"h264_qsv".to_string()));
        assert!(!found.contains(&"aac".to_string()), "audio encoders are not video encoders");
        assert!(!found.contains(&"srt".to_string()));
    }

    #[test]
    fn auto_prefers_the_graphics_card() {
        // The whole reason Replay stops making the machine feel slow.
        let (chosen, _) = pick_encoder("auto", &parse_encoders(SAMPLE), "veryfast");
        assert_eq!(chosen, "h264_nvenc");
    }

    #[test]
    fn auto_falls_back_to_the_cpu_when_there_is_no_hardware_encoder() {
        let available = vec!["libx264".to_string()];
        let (chosen, args) = pick_encoder("auto", &available, "veryfast");
        assert_eq!(chosen, "libx264");
        assert!(args.contains(&"veryfast".to_string()));
    }

    #[test]
    fn auto_with_nothing_detected_still_produces_a_working_command() {
        // available_encoders() returns empty when ffmpeg cannot be run at all.
        let (chosen, _) = pick_encoder("auto", &[], "veryfast");
        assert_eq!(chosen, "libx264");
    }

    #[test]
    fn the_cpu_can_be_forced() {
        let (chosen, _) = pick_encoder("cpu", &parse_encoders(SAMPLE), "veryfast");
        assert_eq!(chosen, "libx264");
    }

    #[test]
    fn a_named_encoder_is_used_when_present() {
        let (chosen, args) = pick_encoder("h264_qsv", &parse_encoders(SAMPLE), "veryfast");
        assert_eq!(chosen, "h264_qsv");
        assert!(args.contains(&"-preset".to_string()));
    }

    #[test]
    fn asking_for_an_encoder_ffmpeg_lacks_falls_back_rather_than_failing() {
        // Recording on the CPU beats not recording.
        let (chosen, _) = pick_encoder("h264_amf", &parse_encoders(SAMPLE), "veryfast");
        assert_eq!(chosen, "libx264");
    }

    #[test]
    fn each_encoder_gets_the_speed_flag_it_actually_understands() {
        // -preset p1 on libx264 or -preset veryfast on AMF is an immediate
        // failure to start, with nothing recorded.
        let nvenc = pick_encoder("h264_nvenc", &["h264_nvenc".into()], "veryfast").1.join(" ");
        assert!(nvenc.contains("p1"), "nvenc uses p1..p7, got {nvenc}");

        let amf = pick_encoder("h264_amf", &["h264_amf".into()], "veryfast").1.join(" ");
        assert!(amf.contains("-quality"), "amf uses -quality, got {amf}");

        let cpu = pick_encoder("cpu", &[], "veryfast").1.join(" ");
        assert!(cpu.contains("-preset veryfast"), "got {cpu}");
    }

    #[test]
    fn the_scale_filter_escapes_the_comma() {
        // Unescaped, `min(1080,ih)` reads as the end of the scale filter
        // followed by a filter named `ih)`. ffmpeg exits with "Filter not
        // found" and records nothing — which is exactly what shipped.
        let settings = ReplaySettings { scale_height: 1080, ..Default::default() };
        let joined = buffer_args(&settings, Path::new("/s"), &[], AudioPlan::NONE).join(" ");
        assert!(
            joined.contains(r"min(1080\,ih)"),
            "the comma must be escaped, got: {joined}",
        );
    }

    #[test]
    fn the_hotkey_saves_thirty_seconds_by_default_not_the_whole_buffer() {
        let settings = ReplaySettings::default();
        assert_eq!(settings.save_seconds, 30);
        assert!(settings.buffer_seconds > settings.save_seconds);
    }

    #[test]
    fn scaling_is_applied_before_the_encoder() {
        let settings = ReplaySettings { scale_height: 1080, ..Default::default() };
        let args = buffer_args(&settings, Path::new("/s"), &[], AudioPlan::NONE);
        let joined = args.join(" ");
        let filter = joined.find("-vf").expect("a scale filter");
        let encoder = joined.find("-c:v").expect("an encoder");
        assert!(filter < encoder, "the encoder must receive the smaller frame");
        assert!(joined.contains("scale=-2:min(1080"));
    }

    #[test]
    fn native_resolution_adds_no_filter() {
        let settings = ReplaySettings { scale_height: 0, ..Default::default() };
        let joined = buffer_args(&settings, Path::new("/s"), &[], AudioPlan::NONE).join(" ");
        assert!(!joined.contains("-vf"), "got {joined}");
    }

    #[test]
    fn scaling_never_enlarges_a_smaller_desktop() {
        // min(h,ih) rather than h: a 720p desktop must not be upscaled to 1080p,
        // which would cost more to encode than recording it natively.
        let settings = ReplaySettings { scale_height: 1080, ..Default::default() };
        let joined = buffer_args(&settings, Path::new("/s"), &[], AudioPlan::NONE).join(" ");
        assert!(joined.contains(r"min(1080\,ih)"));
    }

    #[test]
    fn the_default_buffer_is_two_minutes() {
        assert_eq!(ReplaySettings::default().buffer_seconds, 120);
    }

    #[test]
    fn the_ring_holds_the_whole_window_plus_a_spare() {
        // Everything older than the window is overwritten in place and never
        // reaches the disk as a file the user keeps.
        let count = segment_count(120);
        assert!(count * SEGMENT_SECONDS >= 120, "the ring must cover the window");
        assert_eq!(count, 120 / SEGMENT_SECONDS + 2);
    }

    #[test]
    fn replay_is_off_until_it_is_switched_on() {
        assert!(!ReplaySettings::default().enabled);
    }
}

/// Tests that run real ffmpeg.
///
/// The saving bug that shipped — an input option written in the output position
/// — could not have survived a test that actually ran the command, so these
/// build genuine segment files and stitch them. They skip themselves when
/// ffmpeg is not installed, so they never turn into a failure on a machine that
/// simply does not have it.
#[cfg(test)]
mod ffmpeg_tests {
    use super::*;

    fn ffmpeg_on_path() -> Option<PathBuf> {
        let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        std::env::var_os("PATH")?
            .to_string_lossy()
            .split(if cfg!(windows) { ';' } else { ':' })
            .map(|dir| Path::new(dir).join(name))
            .find(|candidate| candidate.is_file())
    }

    /// Writes `count` segments of `seconds` each, oldest first.
    fn make_segments(ffmpeg: &Path, dir: &Path, count: usize, seconds: u32) -> bool {
        for index in 1..=count {
            let path = dir.join(format!("seg_{index:04}.mp4"));
            let status = Command::new(ffmpeg)
                .args([
                    "-hide_banner", "-loglevel", "error",
                    "-f", "lavfi",
                    "-i", &format!("testsrc=size=320x180:rate=15:duration={seconds}"),
                    "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p",
                    "-y", &path.to_string_lossy(),
                ])
                .status();
            if !matches!(status, Ok(s) if s.success()) {
                return false;
            }
            // Distinct modification times, since segments are chosen by age.
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        true
    }

    fn duration_of(ffmpeg: &Path, file: &Path) -> f64 {
        let probe = ffmpeg.parent().unwrap().join("ffprobe");
        let output = Command::new(probe)
            .args([
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "csv=p=0",
                &file.to_string_lossy(),
            ])
            .output()
            .expect("ffprobe");
        String::from_utf8_lossy(&output.stdout).trim().parse().unwrap_or(0.0)
    }

    #[test]
    fn a_clip_is_actually_saved_and_is_the_length_that_was_asked_for() {
        let Some(ffmpeg) = ffmpeg_on_path() else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };

        let scratch = std::env::temp_dir().join(format!("gamehub-replay-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).unwrap();

        // Six five-second segments: thirty seconds in the buffer.
        if !make_segments(&ffmpeg, &scratch, 6, 5) {
            eprintln!("skipped: this ffmpeg cannot encode H.264");
            return;
        }

        let destination = scratch.join("clip.mp4");
        let size = save_clip(&ffmpeg, &scratch, &destination, 20)
            .expect("saving must succeed — this is the bug that shipped");

        assert!(size > 0, "the clip must have contents");
        assert!(destination.is_file());

        let seconds = duration_of(&ffmpeg, &destination);
        assert!(
            (seconds - 20.0).abs() < 3.0,
            "asked for 20s, got {seconds:.1}s — the seek is not being applied",
        );

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn asking_for_more_than_the_buffer_holds_returns_everything_it_has() {
        let Some(ffmpeg) = ffmpeg_on_path() else { return };

        let scratch = std::env::temp_dir().join(format!("gamehub-replay-all-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).unwrap();

        if !make_segments(&ffmpeg, &scratch, 3, 5) {
            return;
        }

        let destination = scratch.join("clip.mp4");
        save_clip(&ffmpeg, &scratch, &destination, 600).expect("must not fail");

        let seconds = duration_of(&ffmpeg, &destination);
        assert!(seconds > 5.0, "should return the whole buffer, got {seconds:.1}s");

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn an_empty_buffer_explains_itself_instead_of_failing_obscurely() {
        let Some(ffmpeg) = ffmpeg_on_path() else { return };

        let scratch = std::env::temp_dir().join(format!("gamehub-replay-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).unwrap();

        let error = save_clip(&ffmpeg, &scratch, &scratch.join("c.mp4"), 30).unwrap_err();
        assert_eq!(error, "@buffer_empty", "a message code the interface translates");

        let _ = std::fs::remove_dir_all(&scratch);
    }
}

/// The recording command, run for real.
///
/// Two bugs shipped in one release because the argument list was only ever
/// asserted on as a string: an encoder chosen from `ffmpeg -encoders` that the
/// machine could not actually run, and a filter whose comma was not escaped.
/// Both produced the same symptom — an empty buffer — and neither could have
/// survived a test that ran the command. These do.
#[cfg(test)]
pub(crate) mod recording_tests {
    use super::*;

    fn ffmpeg_on_path() -> Option<PathBuf> {
        let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        std::env::var_os("PATH")?
            .to_string_lossy()
            .split(if cfg!(windows) { ';' } else { ':' })
            .map(|dir| Path::new(dir).join(name))
            .find(|candidate| candidate.is_file())
    }

    /// The real argument list, with a synthetic source standing in for gdigrab,
    /// which needs a Windows desktop.
    fn record_with(settings: &ReplaySettings, source: &str, scratch: &Path, ffmpeg: &Path) -> (bool, String) {
        record_with_audio(settings, source, scratch, ffmpeg, AudioPlan::NONE)
    }

    /// Like `record_with`, with a synthetic system-audio feed down the real
    /// pipe — the path a Windows machine takes when Replay has sound.
    pub(super) fn record_with_audio(
        settings: &ReplaySettings,
        source: &str,
        scratch: &Path,
        ffmpeg: &Path,
        plan: AudioPlan,
    ) -> (bool, String) {
        let mut args = buffer_args(settings, scratch, &available_encoders(ffmpeg), plan);

        // Swap the desktop capture for a test pattern, leaving every other
        // argument — filter, encoder, segment muxer — exactly as it ships.
        //
        // -framerate goes too: it belongs to gdigrab, and lavfi refuses it. The
        // rate is carried on the test source instead, so the substitution
        // changes only how frames arrive, never how they are processed.
        let desktop = args.iter().position(|a| a == "desktop").expect("a desktop input");
        args[desktop] = format!("testsrc=size={source}:rate=30");
        let gdigrab = args.iter().position(|a| a == "gdigrab").expect("gdigrab");
        args[gdigrab] = "lavfi".into();
        if let Some(framerate) = args.iter().position(|a| a == "-framerate") {
            args.drain(framerate..framerate + 2);
        }

        let mut child = Command::new(ffmpeg)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("ffmpeg starts");

        // Feed the pipe the way the audio pump does: clock-driven silence with
        // a tone in it, so the muxer has a real second stream to interleave.
        let mut stdin = child.stdin.take();
        let feeder = plan.system.map(|format| {
            let mut stdin = stdin.take().expect("a pipe");
            std::thread::spawn(move || {
                use std::io::Write;
                let started = std::time::Instant::now();
                let mut written: u64 = 0;
                let mut queued = std::collections::VecDeque::new();
                while started.elapsed() < Duration::from_secs(40) {
                    let due = crate::audio::samples_due(format, started.elapsed());
                    // A 440 Hz-ish tone rather than pure silence, so a wrong
                    // format would be audible as well as measurable.
                    while queued.len() < (due.saturating_sub(written)) as usize {
                        let n = written + queued.len() as u64;
                        queued.push_back(((n as f64 / 50.0).sin() * 8000.0) as i16);
                    }
                    let chunk = crate::audio::drain(&mut queued, written, due, 96_000);
                    if !chunk.is_empty() {
                        if stdin.write_all(&crate::audio::to_bytes(&chunk)).is_err() {
                            break;
                        }
                        written += chunk.len() as u64;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            })
        });
        drop(stdin);

        // Poll for the first finished segment rather than sleeping a fixed
        // time. A fixed wait is a race: under load — several of these tests at
        // once — ffmpeg may not have closed a segment yet, and the test fails
        // for a reason that has nothing to do with the code under test.
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while std::time::Instant::now() < deadline {
            let finished = std::fs::read_dir(scratch)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("mp4"))
                        // The segment being written is not finished yet; a
                        // second one having appeared proves the first is done.
                        .count()
                })
                .unwrap_or(0);
            if finished >= 2 {
                break;
            }
            // Nothing will ever appear if ffmpeg has already died.
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let _ = child.kill();
        let output = child.wait_with_output().expect("wait");
        let complaints = String::from_utf8_lossy(&output.stderr).to_string();
        if let Some(feeder) = feeder {
            let _ = feeder.join();
        }

        let segments = std::fs::read_dir(scratch)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("mp4"))
                    .count()
            })
            .unwrap_or(0);

        (segments > 0, complaints)
    }

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gamehub-rec-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_recording_command_actually_produces_segments() {
        let Some(ffmpeg) = ffmpeg_on_path() else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let scratch = scratch_dir("default");
        let (recorded, complaints) = record_with(&ReplaySettings::default(), "1920x1080", &scratch, &ffmpeg);
        assert!(recorded, "nothing was recorded. ffmpeg said:\n{complaints}");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn recording_works_on_a_desktop_smaller_than_the_scale_target() {
        let Some(ffmpeg) = ffmpeg_on_path() else { return };
        let scratch = scratch_dir("small");
        let settings = ReplaySettings { scale_height: 1080, ..Default::default() };
        let (recorded, complaints) = record_with(&settings, "1280x720", &scratch, &ffmpeg);
        assert!(recorded, "a 720p desktop must still record. ffmpeg said:\n{complaints}");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn recording_works_at_native_resolution_too() {
        let Some(ffmpeg) = ffmpeg_on_path() else { return };
        let scratch = scratch_dir("native");
        let settings = ReplaySettings { scale_height: 0, ..Default::default() };
        let (recorded, complaints) = record_with(&settings, "1280x720", &scratch, &ffmpeg);
        assert!(recorded, "no filter at all must also work. ffmpeg said:\n{complaints}");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn only_encoders_that_genuinely_run_are_offered() {
        let Some(ffmpeg) = ffmpeg_on_path() else { return };

        // The heart of the bug: this machine's ffmpeg lists h264_nvenc, and it
        // does not work here. Anything available_encoders() returns must
        // survive an actual encode.
        for encoder in available_encoders(&ffmpeg) {
            assert!(
                encoder_works(&ffmpeg, &encoder),
                "{encoder} was offered but cannot encode a single frame",
            );
        }
    }

    #[test]
    fn there_is_always_something_to_fall_back_to() {
        let Some(ffmpeg) = ffmpeg_on_path() else { return };
        let available = available_encoders(&ffmpeg);
        assert!(
            available.iter().any(|e| e == "libx264"),
            "libx264 must always be offered, got {available:?}",
        );
    }

    #[test]
    fn record_then_save_end_to_end() {
        let Some(ffmpeg) = ffmpeg_on_path() else { return };
        let scratch = scratch_dir("e2e");

        let settings = ReplaySettings::default();
        let (recorded, complaints) = record_with(&settings, "1280x720", &scratch, &ffmpeg);
        assert!(recorded, "nothing recorded: {complaints}");

        // What pressing the hotkey does, on a buffer that has only just started
        // filling — the case the user asked about explicitly.
        let destination = scratch.join("clip.mp4");
        let size = save_clip(&ffmpeg, &scratch, &destination, settings.save_seconds)
            .expect("a short buffer must still save, just shorter");
        assert!(size > 0);
        assert!(destination.is_file());

        let _ = std::fs::remove_dir_all(&scratch);
    }
}


#[cfg(test)]
mod audio_tests {
    use super::*;

    const STEREO: PcmFormat = PcmFormat { sample_rate: 48_000, channels: 2 };

    /// Real `ffmpeg -list_devices` output from a Windows machine, trimmed.
    const LISTING: &str = r#"
[dshow @ 0000021] "Integrated Camera" (video)
[dshow @ 0000021]   Alternative name "@device_pnp_\\?\usb#vid_04f2"
[dshow @ 0000021] "Microphone (Realtek(R) Audio)" (audio)
[dshow @ 0000021]   Alternative name "@device_cm_{33D9A762}\wave_{B1F4}"
[dshow @ 0000021] "Stereo Mix (Realtek(R) Audio)" (audio)
[dshow @ 0000021]   Alternative name "@device_cm_{33D9A762}\wave_{9C22}"
"#;

    #[test]
    fn audio_devices_are_read_out_of_the_listing() {
        let found = parse_audio_devices(LISTING);
        assert_eq!(found, vec!["Microphone (Realtek(R) Audio)", "Stereo Mix (Realtek(R) Audio)"]);
        assert!(!found.iter().any(|d| d.contains("Camera")));
        assert!(!found.iter().any(|d| d.starts_with("@device")));
    }

    #[test]
    fn loopback_devices_are_still_recognised_for_the_microphone_list() {
        // They are not needed for system audio any more, but a user who picks
        // one as the "microphone" should be told what it is.
        let found = parse_audio_devices(LISTING);
        assert_eq!(pick_loopback(&found).as_deref(), Some("Stereo Mix (Realtek(R) Audio)"));
        assert_eq!(pick_loopback(&[]), None);
    }

    #[test]
    fn system_audio_is_on_by_default() {
        // The bug that shipped: clips had no sound. The default must have it.
        let settings = ReplaySettings::default();
        assert!(settings.system_audio);
        assert!(!settings.wants_microphone());
    }

    #[test]
    fn no_audio_means_no_audio_arguments_and_only_video_mapped() {
        let joined = buffer_args(&ReplaySettings::default(), Path::new("/s"), &[], AudioPlan::NONE).join(" ");
        assert!(!joined.contains("pipe:0"));
        assert!(!joined.contains("dshow"));
        assert!(!joined.contains("-c:a"));
        assert!(joined.contains("-map 0:v"));
    }

    #[test]
    fn system_audio_arrives_on_the_pipe_as_input_one() {
        let plan = AudioPlan { system: Some(STEREO), microphone: false };
        let joined = buffer_args(&ReplaySettings::default(), Path::new("/s"), &[], plan).join(" ");
        assert!(joined.contains("-f s16le -ar 48000 -ac 2 -i pipe:0"), "got {joined}");
        assert!(joined.contains("-map 0:v -map 1:a"), "both streams must be mapped explicitly: {joined}");
        assert!(joined.contains("-c:a aac"));
        // The pipe input must be declared *before* the output options.
        assert!(joined.find("pipe:0").unwrap() < joined.find("-c:v").unwrap());
    }

    #[test]
    fn the_pipe_format_follows_the_device_not_a_constant() {
        let plan = AudioPlan { system: Some(PcmFormat { sample_rate: 44_100, channels: 1 }), microphone: false };
        let joined = buffer_args(&ReplaySettings::default(), Path::new("/s"), &[], plan).join(" ");
        assert!(joined.contains("-ar 44100 -ac 1"), "got {joined}");
    }

    #[test]
    fn a_microphone_alone_is_a_dshow_input_mapped_as_the_only_audio() {
        let settings = ReplaySettings { audio_device: "Microphone (Realtek(R) Audio)".into(), ..Default::default() };
        let plan = AudioPlan { system: None, microphone: true };
        let joined = buffer_args(&settings, Path::new("/s"), &[], plan).join(" ");
        assert!(joined.contains("-f dshow -i audio=Microphone (Realtek(R) Audio)"), "got {joined}");
        assert!(joined.contains("-map 0:v -map 1:a"));
        assert!(!joined.contains("amix"));
    }

    #[test]
    fn system_audio_and_microphone_are_mixed_into_one_track() {
        let settings = ReplaySettings { audio_device: "Mic".into(), ..Default::default() };
        let plan = AudioPlan { system: Some(STEREO), microphone: true };
        let joined = buffer_args(&settings, Path::new("/s"), &[], plan).join(" ");
        assert!(joined.contains("amix=inputs=2"), "got {joined}");
        assert!(joined.contains("[1:a][2:a]amix"), "pipe is input 1, microphone input 2: {joined}");
        assert!(joined.contains("-map [v] -map [a]"));
        // -vf and -filter_complex on the same output is an ffmpeg error, so
        // the scale must have moved into the graph.
        assert!(!joined.contains("-vf"), "got {joined}");
        assert!(joined.contains("[0:v]scale=-2:min(1080\\,ih)[v]"), "got {joined}");
    }

    #[test]
    fn choosing_none_as_the_microphone_means_none() {
        let settings = ReplaySettings { audio_device: "none".into(), ..Default::default() };
        assert!(!settings.wants_microphone());
    }

    #[test]
    fn the_buffer_can_be_anything_from_thirty_seconds_to_ten_minutes() {
        assert_eq!(MIN_BUFFER_SECONDS, 30);
        assert_eq!(MAX_BUFFER_SECONDS, 600);
        assert_eq!(ReplaySettings { buffer_seconds: 5, ..Default::default() }.buffer(), 30);
        assert_eq!(ReplaySettings { buffer_seconds: 5_000, ..Default::default() }.buffer(), 600);
        assert_eq!(ReplaySettings { buffer_seconds: 300, ..Default::default() }.buffer(), 300);
        assert_eq!(segment_count(600), 122);
    }

    #[test]
    fn the_disk_estimate_grows_with_the_buffer_and_the_quality() {
        let two_minutes = buffer_estimate_mb(&ReplaySettings::default());
        let ten_minutes = buffer_estimate_mb(&ReplaySettings { buffer_seconds: 600, ..Default::default() });
        let ten_high = buffer_estimate_mb(&ReplaySettings { buffer_seconds: 600, quality: "high".into(), ..Default::default() });
        assert!(two_minutes > 50 && two_minutes < 200, "got {two_minutes} MB");
        assert!(ten_minutes > two_minutes * 4);
        assert!(ten_high > ten_minutes);
    }

    #[test]
    fn old_settings_files_without_the_new_field_still_load() {
        // Settings written by the previous version have no `systemAudio`. They
        // must load with it on, not fail to parse and wipe the user's choices.
        let old = r#"{"enabled":true,"bufferSeconds":120,"fps":30,"quality":"medium","monitor":0,"audioDevice":"","folder":"","scaleHeight":1080,"saveSeconds":30,"encoder":"auto"}"#;
        let parsed: ReplaySettings = serde_json::from_str(old).unwrap();
        assert!(parsed.system_audio);
        assert!(parsed.enabled);
    }

    #[test]
    fn old_clips_without_the_audio_flag_still_load() {
        let old = r#"{"id":"x","path":"/c.mp4","gameId":null,"gameName":"Desktop","recordedAt":"2026-01-01T00:00:00Z","seconds":30,"sizeBytes":1,"favorite":false}"#;
        let parsed: Clip = serde_json::from_str(old).unwrap();
        assert!(!parsed.has_audio);
    }
}

/// System audio down the real pipe, with real ffmpeg.
#[cfg(test)]
mod piped_audio_tests {
    use super::*;

    fn ffmpeg_on_path() -> Option<PathBuf> {
        let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        std::env::var_os("PATH")?
            .to_string_lossy()
            .split(if cfg!(windows) { ';' } else { ':' })
            .map(|dir| Path::new(dir).join(name))
            .find(|candidate| candidate.is_file())
    }

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gamehub-audio-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn streams_of(ffmpeg: &Path, file: &Path) -> String {
        let probe = ffmpeg.parent().unwrap().join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
        let output = Command::new(probe)
            .args(["-v", "error", "-show_entries", "stream=codec_type", "-of", "csv=p=0", &file.to_string_lossy()])
            .output()
            .expect("ffprobe");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[test]
    fn a_recording_with_system_audio_has_a_sound_track_in_every_segment() {
        let Some(ffmpeg) = ffmpeg_on_path() else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let scratch = scratch_dir("piped");
        let plan = AudioPlan { system: Some(PcmFormat { sample_rate: 48_000, channels: 2 }), microphone: false };
        let (recorded, complaints) =
            super::recording_tests::record_with_audio(&ReplaySettings::default(), "1280x720", &scratch, &ffmpeg, plan);
        assert!(recorded, "nothing recorded with the pipe attached. ffmpeg said:\n{complaints}");

        // Every finished segment must carry both streams, or the clip built
        // from them has no sound — the very bug this replaces.
        let mut checked = 0;
        for entry in std::fs::read_dir(&scratch).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("mp4") {
                continue;
            }
            let streams = streams_of(&ffmpeg, &path);
            if streams.trim().is_empty() {
                // The segment ffmpeg was writing when it was killed.
                continue;
            }
            assert!(streams.contains("video"), "{}: {streams}", path.display());
            assert!(streams.contains("audio"), "{} has no audio: {streams}", path.display());
            checked += 1;
        }
        assert!(checked > 0, "no finished segment to check");

        // And the stitched clip keeps the audio too.
        let destination = scratch.join("clip.mp4");
        save_clip(&ffmpeg, &scratch, &destination, 10).expect("save");
        let streams = streams_of(&ffmpeg, &destination);
        assert!(streams.contains("audio"), "the saved clip lost its sound: {streams}");

        let _ = std::fs::remove_dir_all(&scratch);
    }
}
