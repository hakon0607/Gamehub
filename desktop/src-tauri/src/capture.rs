//! Taking a screenshot.
//!
//! `xcap` grabs the monitor's framebuffer, which is what every screenshot tool
//! does and needs no special privileges. Two things it cannot do, and neither
//! is papered over: a game running in exclusive fullscreen may hand back a
//! black frame, and a window with display-capture protection is excluded by
//! Windows itself. Both are reported to the user rather than saved as a black
//! PNG.

use std::path::{Path, PathBuf};

use gamehub_detect::media;

/// Which monitor to grab. Index 0 is the primary.
pub fn monitor_names() -> Vec<String> {
    xcap::Monitor::all()
        .map(|monitors| {
            monitors
                .iter()
                .enumerate()
                .map(|(index, monitor)| {
                    let name = monitor.name();
                    let label = if name.trim().is_empty() {
                        format!("Display {}", index + 1)
                    } else {
                        name.to_string()
                    };
                    format!("{label} ({}×{})", monitor.width(), monitor.height())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Captures a monitor and writes it as a PNG under `root`, in the folder for
/// `game_name`. Returns the path and its size.
pub fn capture(
    root: &Path,
    game_name: &str,
    taken_at: &str,
    monitor_index: usize,
) -> Result<(PathBuf, u64), String> {
    let monitors = xcap::Monitor::all().map_err(|e| crate::msg::code("display_read", &[&e.to_string()]))?;
    if monitors.is_empty() {
        return Err(crate::msg::plain("no_display"));
    }
    let monitor = monitors
        .get(monitor_index)
        .or_else(|| monitors.first())
        .ok_or_else(|| crate::msg::plain("no_display"))?;

    let image = monitor
        .capture_image()
        .map_err(|e| crate::msg::code("capture_failed", &[&e.to_string()]))?;

    if is_blank(&image) {
        return Err(crate::msg::plain("blank_frame"));
    }

    let folder = media::folder_for(root, game_name);
    std::fs::create_dir_all(&folder).map_err(|e| crate::msg::code("shot_folder", &[&e.to_string()]))?;

    let name = media::file_name(game_name, taken_at);
    let path = gamehub_detect::safepath::join_within(&folder, &name).map_err(|e| e.to_string())?;
    image
        .save(&path)
        .map_err(|e| crate::msg::code("shot_write", &[&e.to_string()]))?;

    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    Ok((path, size))
}

/// A frame where every sampled pixel is identical is almost certainly a failed
/// capture rather than a picture. Sampling beats scanning every pixel of a 4K
/// frame on the UI's timescale.
fn is_blank(image: &image::RgbaImage) -> bool {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return true;
    }
    let first = image.get_pixel(0, 0);
    let step_x = (width / 32).max(1);
    let step_y = (height / 32).max(1);
    for y in (0..height).step_by(step_y as usize) {
        for x in (0..width).step_by(step_x as usize) {
            if image.get_pixel(x, y) != first {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uniform_frame_is_treated_as_a_failed_capture() {
        let blank = image::RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 255]));
        assert!(is_blank(&blank));
    }

    #[test]
    fn a_frame_with_content_is_not_blank() {
        let mut picture = image::RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 255]));
        picture.put_pixel(40, 40, image::Rgba([255, 255, 255, 255]));
        assert!(!is_blank(&picture));
    }

    #[test]
    fn an_empty_image_is_blank_rather_than_a_panic() {
        assert!(is_blank(&image::RgbaImage::new(0, 0)));
    }
}
