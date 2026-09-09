//! System audio for Replay — the sound that comes out of the speakers.
//!
//! Windows has no capture device ffmpeg can open for "what you hear". The old
//! approach — ask the user to enable "Stereo Mix" — worked on a minority of
//! sound cards and produced silent clips on the rest. This module records the
//! default output device through WASAPI loopback instead, which every Windows
//! machine since Vista supports, and pipes the raw PCM into ffmpeg's stdin as a
//! second input. No driver, no virtual cable, nothing to enable.
//!
//! The pump is clock-driven on purpose. WASAPI loopback delivers no packets at
//! all while nothing is playing, and a pipe that goes quiet stalls ffmpeg's
//! muxer — the video keeps arriving and has nowhere to go. So the writer thread
//! works out how many samples *should* have been written by now, sends what the
//! callback delivered, and pads the difference with silence. That keeps the
//! audio track exactly as long as the video track, whatever the game does.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// The shape of the PCM the pump produces, which ffmpeg has to be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl PcmFormat {
    /// The ffmpeg input arguments for a raw stream in this format.
    pub fn ffmpeg_args(&self) -> Vec<String> {
        vec![
            "-thread_queue_size".into(),
            "2048".into(),
            "-f".into(),
            "s16le".into(),
            "-ar".into(),
            self.sample_rate.to_string(),
            "-ac".into(),
            self.channels.to_string(),
            "-i".into(),
            "pipe:0".into(),
        ]
    }
}

/// How many interleaved samples the pump should have written after `elapsed`.
///
/// Pure, because the whole point of the pump is this arithmetic and it must be
/// right: too few and the track drifts behind the video, too many and it runs
/// ahead.
pub fn samples_due(format: PcmFormat, elapsed: Duration) -> u64 {
    let seconds = elapsed.as_secs_f64();
    (seconds * format.sample_rate as f64).floor() as u64 * format.channels as u64
}

/// Decides what to write on one tick: real samples, padded with silence up to
/// what the clock says is due.
///
/// `queued` is what the callback has delivered so far; `written` is what has
/// already gone down the pipe; `due` is what should have gone by now. Returns
/// the samples to write, and drops any backlog beyond `max_backlog` so a burst
/// (Windows waking up, a stall) cannot push the track permanently late.
pub fn drain(queued: &mut VecDeque<i16>, written: u64, due: u64, max_backlog: usize) -> Vec<i16> {
    let needed = due.saturating_sub(written) as usize;
    if needed == 0 {
        // Ahead of the clock: keep a little in hand, but never a lot.
        if queued.len() > max_backlog {
            let excess = queued.len() - max_backlog;
            queued.drain(..excess);
        }
        return Vec::new();
    }
    let mut out: Vec<i16> = queued.drain(..needed.min(queued.len())).collect();
    if out.len() < needed {
        out.resize(needed, 0);
    }
    out
}

/// Interleaved i16 samples as little-endian bytes.
pub fn to_bytes(samples: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// A running loopback capture.
///
/// Dropping it stops the pump; the stream itself lives on the pump's thread,
/// since audio streams are not sendable between threads on every backend.
pub struct SystemAudio {
    pub format: PcmFormat,
    stop: Arc<AtomicBool>,
    queue: Arc<Mutex<VecDeque<i16>>>,
    /// The device the sound is taken from, for the status card.
    pub device_name: String,
    /// Set by the stream thread if the capture dies mid-recording.
    failure: Arc<Mutex<Option<String>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SystemAudio {
    /// Opens the default output device for loopback capture.
    ///
    /// Returns the capture and its format before any ffmpeg is started, so the
    /// command line can be built to match. Fails plainly when the machine has
    /// no output device or the backend refuses loopback — the caller records
    /// without system sound and says so, rather than recording nothing.
    pub fn open() -> Result<Self, String> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "Ingen lydenhet er valgt som standard utgang i Windows.".to_string())?;
        let device_name = device.name().unwrap_or_else(|_| "Standard lydenhet".into());

        // On WASAPI an output device opened for input *is* loopback capture.
        let supported = device
            .default_input_config()
            .map_err(|e| format!("Lydenheten «{device_name}» kan ikke tas opp fra: {e}"))?;
        let format = PcmFormat {
            sample_rate: supported.sample_rate().0,
            channels: supported.channels(),
        };
        let sample_format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let queue: Arc<Mutex<VecDeque<i16>>> = Arc::new(Mutex::new(VecDeque::with_capacity(
            format.sample_rate as usize * format.channels as usize,
        )));
        let stop = Arc::new(AtomicBool::new(false));
        let failure: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        // The stream is built on its own thread and kept alive there until
        // stop is raised. Errors from building it are handed back over a
        // channel so `open` can fail synchronously.
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        let thread = {
            let queue = queue.clone();
            let stop = stop.clone();
            let failure = failure.clone();
            std::thread::Builder::new()
                .name("replay-audio".into())
                .spawn(move || {
                    let sink = queue.clone();
                    let error_sink = failure.clone();
                    let on_error = move |error: cpal::StreamError| {
                        *error_sink.lock() = Some(format!("Lydopptaket stoppet: {error}"));
                    };
                    let built = match sample_format {
                        cpal::SampleFormat::F32 => device.build_input_stream(
                            &config,
                            move |data: &[f32], _| push(&sink, data.iter().map(|s| f32_to_i16(*s))),
                            on_error,
                            None,
                        ),
                        cpal::SampleFormat::I16 => device.build_input_stream(
                            &config,
                            move |data: &[i16], _| push(&sink, data.iter().copied()),
                            on_error,
                            None,
                        ),
                        cpal::SampleFormat::U16 => device.build_input_stream(
                            &config,
                            move |data: &[u16], _| push(&sink, data.iter().map(|s| (*s as i32 - 32768) as i16)),
                            on_error,
                            None,
                        ),
                        cpal::SampleFormat::I32 => device.build_input_stream(
                            &config,
                            move |data: &[i32], _| push(&sink, data.iter().map(|s| (*s >> 16) as i16)),
                            on_error,
                            None,
                        ),
                        other => Err(cpal::BuildStreamError::BackendSpecific {
                            err: cpal::BackendSpecificError {
                                description: format!("sample format {other:?} is not supported"),
                            },
                        }),
                    };
                    let stream = match built {
                        Ok(stream) => stream,
                        Err(error) => {
                            let _ = ready_tx.send(Err(format!("Lydopptaket kunne ikke startes: {error}")));
                            return;
                        }
                    };
                    use cpal::traits::StreamTrait;
                    if let Err(error) = stream.play() {
                        let _ = ready_tx.send(Err(format!("Lydopptaket kunne ikke startes: {error}")));
                        return;
                    }
                    let _ = ready_tx.send(Ok(()));
                    while !stop.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    drop(stream);
                })
                .map_err(|e| format!("Lydtråden kunne ikke startes: {e}"))?
        };

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                stop.store(true, Ordering::Relaxed);
                let _ = thread.join();
                return Err(error);
            }
            Err(_) => {
                stop.store(true, Ordering::Relaxed);
                return Err("Lydenheten svarte ikke.".into());
            }
        }

        Ok(Self { format, stop, queue, device_name, failure, thread: Some(thread) })
    }

    /// Feeds ffmpeg until it closes the pipe or the capture is stopped.
    ///
    /// Runs on its own thread so the recorder never blocks on audio. Writes
    /// every 20 ms; the pad-with-silence rule keeps the stream continuous even
    /// while the game is quiet.
    pub fn pump_into(&self, mut stdin: std::process::ChildStdin) -> std::thread::JoinHandle<()> {
        let queue = self.queue.clone();
        let stop = self.stop.clone();
        let format = self.format;
        // Up to 200 ms held back before it is dropped as late.
        let max_backlog = (format.sample_rate as usize / 5) * format.channels as usize;

        std::thread::Builder::new()
            .name("replay-audio-pump".into())
            .spawn(move || {
                let started = Instant::now();
                let mut written: u64 = 0;
                // The first tick lets a little audio accumulate, so the clock
                // is not permanently 20 ms ahead of the callback.
                std::thread::sleep(Duration::from_millis(60));
                while !stop.load(Ordering::Relaxed) {
                    let due = samples_due(format, started.elapsed());
                    let chunk = {
                        let mut queued = queue.lock();
                        drain(&mut queued, written, due, max_backlog)
                    };
                    if !chunk.is_empty() {
                        if stdin.write_all(&to_bytes(&chunk)).is_err() {
                            // ffmpeg went away; nothing more to do here.
                            break;
                        }
                        written += chunk.len() as u64;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let _ = stdin.flush();
            })
            .expect("the audio pump thread starts")
    }

    /// Why the capture died, if it did.
    pub fn failure(&self) -> Option<String> {
        self.failure.lock().clone()
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for SystemAudio {
    fn drop(&mut self) {
        self.stop();
    }
}

fn push(sink: &Arc<Mutex<VecDeque<i16>>>, samples: impl Iterator<Item = i16>) {
    let mut queue = sink.lock();
    queue.extend(samples);
    // A callback that keeps delivering while the pump is stalled must not grow
    // without bound: two seconds is more than any stall the pump survives.
    const CAP: usize = 48_000 * 2 * 2;
    if queue.len() > CAP {
        let excess = queue.len() - CAP;
        queue.drain(..excess);
    }
}

fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEREO_48K: PcmFormat = PcmFormat { sample_rate: 48_000, channels: 2 };

    #[test]
    fn the_clock_says_how_many_samples_are_due() {
        assert_eq!(samples_due(STEREO_48K, Duration::from_secs(1)), 96_000);
        assert_eq!(samples_due(STEREO_48K, Duration::from_millis(500)), 48_000);
        assert_eq!(samples_due(STEREO_48K, Duration::ZERO), 0);
        // Always a whole number of frames, never half a stereo pair.
        assert_eq!(samples_due(STEREO_48K, Duration::from_nanos(10_417)) % 2, 0);
    }

    #[test]
    fn silence_fills_in_when_the_game_is_quiet() {
        // The reason the pump is clock-driven: nothing delivered, but the track
        // must keep pace with the video anyway.
        let mut queued = VecDeque::new();
        let out = drain(&mut queued, 0, 960, 9_600);
        assert_eq!(out.len(), 960);
        assert!(out.iter().all(|s| *s == 0));
    }

    #[test]
    fn real_samples_are_used_before_any_silence() {
        let mut queued: VecDeque<i16> = (1..=500).collect();
        let out = drain(&mut queued, 0, 960, 9_600);
        assert_eq!(out.len(), 960);
        assert_eq!(out[0], 1);
        assert_eq!(out[499], 500);
        assert_eq!(out[500], 0, "the shortfall is padded, not skipped");
        assert!(queued.is_empty());
    }

    #[test]
    fn nothing_is_written_ahead_of_the_clock() {
        let mut queued: VecDeque<i16> = (1..=500).collect();
        let out = drain(&mut queued, 960, 960, 9_600);
        assert!(out.is_empty());
        assert_eq!(queued.len(), 500, "held for the next tick");
    }

    #[test]
    fn a_backlog_is_trimmed_so_the_track_cannot_run_late() {
        let mut queued: VecDeque<i16> = (0..20_000).collect();
        let _ = drain(&mut queued, 960, 960, 9_600);
        assert_eq!(queued.len(), 9_600);
        // The oldest samples are the ones dropped.
        assert_eq!(*queued.front().unwrap(), 20_000 - 9_600);
    }

    #[test]
    fn bytes_are_little_endian_pcm() {
        assert_eq!(to_bytes(&[1, -1]), vec![1, 0, 0xff, 0xff]);
    }

    #[test]
    fn float_samples_are_scaled_and_clamped() {
        assert_eq!(f32_to_i16(0.0), 0);
        assert_eq!(f32_to_i16(1.0), i16::MAX);
        assert_eq!(f32_to_i16(4.0), i16::MAX, "clipping must not wrap around");
        assert_eq!(f32_to_i16(-1.0), -i16::MAX);
    }

    #[test]
    fn ffmpeg_is_told_the_exact_format() {
        let args = STEREO_48K.ffmpeg_args().join(" ");
        assert!(args.contains("-f s16le"));
        assert!(args.contains("-ar 48000"));
        assert!(args.contains("-ac 2"));
        assert!(args.ends_with("-i pipe:0"));
    }

    #[test]
    fn opening_without_a_device_is_an_error_not_a_panic() {
        // On a machine with no sound (CI, this container) `open` must come back
        // with a message the UI can show. On a machine with one it must come
        // back with a format ffmpeg can be given.
        match SystemAudio::open() {
            Ok(mut audio) => {
                assert!(audio.format.sample_rate > 0);
                assert!(audio.format.channels > 0);
                audio.stop();
            }
            Err(message) => assert!(!message.is_empty()),
        }
    }
}
