//! Performance monitoring.
//!
//! Everything here comes from `sysinfo`, which reads what the OS already
//! publishes. That covers CPU, memory, disk and network honestly.
//!
//! What it does **not** cover, and what GameHub therefore refuses to display:
//!
//! * **Frame rate.** Real per-game FPS means intercepting the game's calls to
//!   Direct3D or Vulkan — what PresentMon and overlays like RTSS do. Nothing in
//!   a process list can produce it, and a number invented from CPU load would
//!   be a lie on screen.
//! * **GPU load and temperatures.** These need vendor libraries (NVML for
//!   NVIDIA, ADL for AMD) or WMI queries that are frequently blocked. Rather
//!   than report a wrong number, the fields are `None` and the UI says the
//!   value is unavailable.

use serde::Serialize;
use sysinfo::{Disks, Networks, ProcessRefreshKind, RefreshKind, System};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameProcess {
    pub name: String,
    /// Percent of one core times the number of cores, as Windows reports it.
    pub cpu_percent: f32,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSample {
    pub cpu_percent: f32,
    pub cpu_name: String,
    pub cpu_cores: usize,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
    pub network_down_bytes: u64,
    pub network_up_bytes: u64,
    pub disks: Vec<DiskUsage>,
    /// The processes belonging to the game that is running, if one is.
    pub game_processes: Vec<GameProcess>,
    /// Always `None`. Present so the UI can say why rather than show nothing.
    pub fps: Option<f32>,
    pub gpu_percent: Option<f32>,
    pub cpu_temperature_c: Option<f32>,
    /// Explains the three fields above, shown once in the UI.
    pub unavailable_note: String,
    pub sampled_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    pub name: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

pub const UNAVAILABLE_NOTE: &str =
    "Frame rate, GPU load and temperatures need vendor drivers or a graphics-API hook that GameHub does \
     not install, so they are not shown rather than guessed at.";

pub struct PerformanceMonitor {
    system: System,
    networks: Networks,
    disks: Disks,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::new()
                    .with_cpu(sysinfo::CpuRefreshKind::everything())
                    .with_memory(sysinfo::MemoryRefreshKind::everything())
                    .with_processes(ProcessRefreshKind::new().with_cpu().with_memory().with_exe(
                        sysinfo::UpdateKind::Always,
                    )),
            ),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
        }
    }

    /// One reading. `game_dirs` are the install folders of whatever is running,
    /// so a game's own processes can be told from everything else.
    pub fn sample(&mut self, game_dirs: &[String]) -> PerformanceSample {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.system
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        self.networks.refresh();
        self.disks.refresh();

        let cpu_percent = self.system.global_cpu_usage();
        let cpu_name = self
            .system
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "CPU".into());

        let mut disk_read = 0u64;
        let mut disk_written = 0u64;
        let mut game_processes: Vec<GameProcess> = Vec::new();

        for process in self.system.processes().values() {
            let usage = process.disk_usage();
            disk_read += usage.read_bytes;
            disk_written += usage.written_bytes;

            if game_dirs.is_empty() {
                continue;
            }
            let Some(exe) = process.exe() else { continue };
            if game_dirs
                .iter()
                .any(|dir| gamehub_detect::safepath::is_within(std::path::Path::new(dir), exe))
            {
                game_processes.push(GameProcess {
                    name: process.name().to_string_lossy().into_owned(),
                    cpu_percent: process.cpu_usage(),
                    memory_bytes: process.memory(),
                });
            }
        }
        game_processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
        game_processes.truncate(6);

        let (down, up) = self
            .networks
            .iter()
            .fold((0u64, 0u64), |(d, u), (_, data)| (d + data.received(), u + data.transmitted()));

        let disks = self
            .disks
            .iter()
            .map(|disk| DiskUsage {
                name: disk.mount_point().to_string_lossy().into_owned(),
                used_bytes: disk.total_space().saturating_sub(disk.available_space()),
                total_bytes: disk.total_space(),
            })
            .collect();

        PerformanceSample {
            cpu_percent,
            cpu_name,
            cpu_cores: self.system.cpus().len(),
            memory_used_bytes: self.system.used_memory(),
            memory_total_bytes: self.system.total_memory(),
            swap_used_bytes: self.system.used_swap(),
            disk_read_bytes: disk_read,
            disk_written_bytes: disk_written,
            network_down_bytes: down,
            network_up_bytes: up,
            disks,
            game_processes,
            fps: None,
            gpu_percent: None,
            cpu_temperature_c: None,
            unavailable_note: UNAVAILABLE_NOTE.to_string(),
            sampled_at: gamehub_detect::now_iso8601(),
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sample_reports_real_cpu_and_memory() {
        let mut monitor = PerformanceMonitor::new();
        let sample = monitor.sample(&[]);
        assert!(sample.memory_total_bytes > 0, "the machine has memory");
        assert!(sample.cpu_cores > 0);
        assert!((0.0..=100.0 * sample.cpu_cores as f32).contains(&sample.cpu_percent));
    }

    #[test]
    fn the_values_that_cannot_be_measured_are_none_and_explained() {
        let mut monitor = PerformanceMonitor::new();
        let sample = monitor.sample(&[]);
        assert!(sample.fps.is_none(), "a made-up frame rate is worse than none");
        assert!(sample.gpu_percent.is_none());
        assert!(sample.cpu_temperature_c.is_none());
        assert!(sample.unavailable_note.contains("not shown rather than guessed"));
    }

    #[test]
    fn no_game_folders_means_no_game_processes() {
        let mut monitor = PerformanceMonitor::new();
        assert!(monitor.sample(&[]).game_processes.is_empty());
    }
}
