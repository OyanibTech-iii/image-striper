use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

/// Decodes only the raw pixel matrix, normalizes EXIF orientation in memory so visual rotation
/// is preserved without any EXIF tags, and encodes a brand-new clean container.
pub fn strip_by_reencoding(
    raw: &[u8],
    format: ImageFormat,
    orientation: Option<u32>,
    jpeg_quality: u8,
) -> Result<Vec<u8>> {
    let decoded = image::load_from_memory_with_format(raw, format)
        .context("Failed to decode image pixel buffer")?;

    let oriented = apply_exif_orientation(decoded, orientation.unwrap_or(1));

    let mut out = Vec::new();
    match format {
        ImageFormat::Jpeg => {
            let quality = jpeg_quality.clamp(1, 100);
            let mut encoder = JpegEncoder::new_with_quality(&mut out, quality);
            encoder
                .encode_image(&oriented)
                .context("Failed to encode sanitized JPEG buffer")?;
        }
        _ => {
            let mut cursor = Cursor::new(&mut out);
            oriented
                .write_to(&mut cursor, format)
                .context("Failed to encode sanitized image buffer")?;
        }
    }

    // Extra defense-in-depth: run lossless chunk scrubber over the freshly encoded output
    if let Some(double_cleaned) = super::lossless::try_strip_lossless(&out, format)? {
        return Ok(double_cleaned);
    }

    Ok(out)
}

/// Applies the standard EXIF Orientation (1..=8) transformation directly to the pixel buffer.
fn apply_exif_orientation(img: DynamicImage, orientation: u32) -> DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}
