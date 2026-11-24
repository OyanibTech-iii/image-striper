pub mod lossless;
pub mod reencode;

use anyhow::Result;
use image::ImageFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripMode {
    /// Decodes pixels, applies EXIF orientation in memory, and encodes a brand-new container
    PixelReencode { jpeg_quality: u8 },
    /// Strips container metadata chunks (APP1..APP15, COM, PNG ancillary, WebP EXIF/XMP/ICC) without recompressing pixels
    LosslessContainer,
}

/// Sanitizes an image buffer using the requested stripping engine.
pub fn strip_image_bytes(
    raw: &[u8],
    mode: StripMode,
    orientation: Option<u32>,
) -> Result<(Vec<u8>, ImageFormat)> {
    let format = image::guess_format(raw)?;

    match mode {
        StripMode::LosslessContainer => {
            if let Some(cleaned) = lossless::try_strip_lossless(raw, format)? {
                return Ok((cleaned, format));
            }
            // Fall back to pixel re-encoding for formats without a lossless chunk container (e.g. BMP/TIFF)
            let cleaned = reencode::strip_by_reencoding(raw, format, orientation, 95)?;
            Ok((cleaned, format))
        }
        StripMode::PixelReencode { jpeg_quality } => {
            let cleaned = reencode::strip_by_reencoding(raw, format, orientation, jpeg_quality)?;
            Ok((cleaned, format))
        }
    }
}
