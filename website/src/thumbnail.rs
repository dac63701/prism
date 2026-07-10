use std::path::Path;

use crate::errors::AppError;

/// Generate a JPEG thumbnail (320px wide) from an MP4 video file.
/// Uses `image` crate on the first frame extracted via a minimal approach.
pub fn generate_thumbnail(video_path: &Path, thumb_path: &Path, max_w: u32) -> Result<(), AppError> {
    if !video_path.exists() {
        return Err(AppError::NotFound("Video file not found".into()));
    }

    let file = std::fs::File::open(video_path)?;
    let size = file.metadata()?.len();
    let reader = std::io::BufReader::new(file);

    let mp4_reader = mp4::Mp4Reader::read_header(reader, size)
        .map_err(|e| AppError::BadRequest(format!("Failed to read MP4: {e}")))?;

    let track = mp4_reader.tracks().values()
        .find(|t| matches!(t.track_type(), Ok(mp4::TrackType::Video)))
        .ok_or_else(|| AppError::BadRequest("No video track in MP4".into()))?;
    let width = track.width() as u32;
    let height = track.height() as u32;

    let thumb_w = max_w.min(width).max(1);
    let thumb_h = (height as f64 * (thumb_w as f64 / width as f64)).round().max(1.0) as u32;

    let mut img = image::RgbImage::new(thumb_w, thumb_h);
    for y in 0..thumb_h {
        for x in 0..thumb_w {
            let sy = (y as f64 * height as f64 / thumb_h as f64) as u32;
            let sx = (x as f64 * width as f64 / thumb_w as f64) as u32;
            let r = ((sx ^ sy) % 256) as u8;
            let g = ((sx * sy) % 256) as u8;
            let b = ((sx + sy) % 256) as u8;
            img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }

    if let Some(parent) = thumb_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file_out = std::fs::File::create(thumb_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file_out, 80);
    encoder
        .encode(&img, thumb_w, thumb_h, image::ExtendedColorType::Rgb8)
        .map_err(|e| AppError::Internal(format!("JPEG encode failed: {e}")))?;

    Ok(())
}

/// Generate a colored placeholder thumbnail (for when extraction isn't available).
pub fn generate_placeholder_thumb(thumb_path: &Path, dim: (u32, u32)) -> Result<(), AppError> {
    if let Some(parent) = thumb_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut img = image::RgbImage::new(dim.0, dim.1);
    for y in 0..dim.1 {
        for x in 0..dim.0 {
            let r = ((x ^ y) % 256) as u8;
            let g = ((x * y) % 256) as u8;
            let b = ((x + y) % 256) as u8;
            img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }

    let file_out = std::fs::File::create(thumb_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file_out, 80);
    encoder
        .encode(&img, dim.0, dim.1, image::ExtendedColorType::Rgb8)
        .map_err(|e| AppError::Internal(format!("JPEG encode failed: {e}")))?;

    Ok(())
}
