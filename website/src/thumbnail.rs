use std::path::Path;

use crate::errors::AppError;

/// Generate a JPEG thumbnail from an MP4 video file.
///
/// Tries `ffmpeg` first to extract the first frame as a real thumbnail.
/// Falls back to a pattern-based placeholder if ffmpeg is unavailable.
pub fn generate_thumbnail(
    video_path: &Path,
    thumb_path: &Path,
    max_w: u32,
) -> Result<(), AppError> {
    if !video_path.exists() {
        return Err(AppError::NotFound("Video file not found".into()));
    }

    // Try ffmpeg subprocess — extracts the real first frame
    let result = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &video_path.to_string_lossy(),
            "-vframes",
            "1",
            "-q:v",
            "2",
            "-vf",
            &format!("scale={}:-1", max_w),
            &thumb_path.to_string_lossy(),
        ])
        .output();

    match result {
        Ok(output) if output.status.success() => return Ok(()),
        _ => {
            // ffmpeg failed or not installed — fall through to placeholder
        }
    }

    generate_pattern_placeholder(thumb_path, (max_w, (max_w as f64 * 9.0 / 16.0) as u32))
}

fn generate_pattern_placeholder(
    thumb_path: &Path,
    dim: (u32, u32),
) -> Result<(), AppError> {
    if let Some(parent) = thumb_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let w = dim.0.max(1);
    let h = dim.1.max(1);
    let mut img = image::RgbImage::new(w, h);
    for y in 0..h {
        let t = y as f64 / h as f64;
        for x in 0..w {
            let cx = x as f64 / w as f64;
            let diag = ((cx + t) * 0.5).clamp(0.0, 1.0);
            let base = (70.0 + diag * 30.0) as u8;
            img.put_pixel(x, y, image::Rgb([base, base.saturating_sub(10), base.saturating_sub(20)]));
        }
    }

    let file_out = std::fs::File::create(thumb_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file_out, 80);
    encoder
        .encode(&img, w, h, image::ExtendedColorType::Rgb8)
        .map_err(|e| AppError::Internal(format!("JPEG encode failed: {e}")))?;

    Ok(())
}

/// Generate a pattern-based placeholder thumbnail (used externally for
/// cases where no video file is available).
#[allow(dead_code)]
pub fn generate_placeholder_thumb(thumb_path: &Path, dim: (u32, u32)) -> Result<(), AppError> {
    generate_pattern_placeholder(thumb_path, dim)
}
