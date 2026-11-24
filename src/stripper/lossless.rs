use anyhow::{Context, Result};
use image::ImageFormat;
use img_parts::jpeg::Jpeg;
use img_parts::png::Png;
use img_parts::webp::WebP;
use img_parts::{Bytes, ImageEXIF, ImageICC};

/// Performs container-level lossless metadata stripping for JPEG, PNG, and WebP.
/// Returns `Ok(Some(clean_bytes))` if lossless stripping is supported for the format,
/// or `Ok(None)` if pixel re-encoding is required.
pub fn try_strip_lossless(raw: &[u8], format: ImageFormat) -> Result<Option<Vec<u8>>> {
    let bytes = Bytes::copy_from_slice(raw);

    match format {
        ImageFormat::Jpeg => {
            let mut jpeg = Jpeg::from_bytes(bytes)
                .context("Failed to parse JPEG container for lossless stripping")?;
            jpeg.set_exif(None);
            jpeg.set_icc_profile(None);

            // Retain only essential JPEG structure segments; drop APP1..APP15 (0xE1..=0xEF) and COM (0xFE)
            jpeg.segments_mut().retain(|seg| {
                let m = seg.marker();
                !((0xE1..=0xEF).contains(&m) || m == 0xFE)
            });

            let mut out = Vec::with_capacity(raw.len());
            jpeg.encoder().write_to(&mut out)?;
            Ok(Some(out))
        }
        ImageFormat::Png => {
            let mut png = Png::from_bytes(bytes)
                .context("Failed to parse PNG container for lossless stripping")?;
            png.set_exif(None);
            png.set_icc_profile(None);

            // Keep only critical rendering chunks: IHDR, PLTE, IDAT, IEND, tRNS
            png.chunks_mut().retain(|chunk| {
                let kind_bytes = chunk.kind();
                let kind = std::str::from_utf8(&kind_bytes).unwrap_or("");
                matches!(kind, "IHDR" | "PLTE" | "IDAT" | "IEND" | "tRNS")
            });

            let mut out = Vec::with_capacity(raw.len());
            png.encoder().write_to(&mut out)?;
            Ok(Some(out))
        }
        ImageFormat::WebP => {
            let mut webp = WebP::from_bytes(bytes)
                .context("Failed to parse WebP container for lossless stripping")?;
            webp.set_exif(None);
            webp.set_icc_profile(None);

            // Also drop XMP chunks
            webp.chunks_mut().retain(|chunk| {
                let id_bytes = chunk.id();
                let id = std::str::from_utf8(&id_bytes).unwrap_or("");
                !matches!(id, "EXIF" | "XMP " | "ICCP")
            });

            let mut out = Vec::with_capacity(raw.len());
            webp.encoder().write_to(&mut out)?;
            Ok(Some(out))
        }
        _ => Ok(None),
    }
}
