#![forbid(unsafe_code)]

mod cli;
mod hygiene;
mod inspector;
mod stripper;

use anyhow::{bail, Result};
use clap::Parser;
use owo_colors::OwoColorize;
use std::fs;
use std::path::{Path, PathBuf};
use stripper::StripMode;

fn main() {
    let args = cli::Cli::parse();

    if !args.quiet {
        cli::print_banner();
    }

    if let Err(err) = run(&args) {
        eprintln!("{} {:#}", "[ERROR]".bright_red().bold(), err);
        std::process::exit(1);
    }
}

fn run(args: &cli::Cli) -> Result<()> {
    let targets = collect_targets(&args.images)?;
    if targets.is_empty() {
        bail!("No valid image files found in the specified path(s).");
    }

    if args.output.is_some()
        && targets.len() > 1
        && !args.output.as_ref().unwrap().is_dir()
    {
        bail!("When processing multiple images, --output must point to an existing directory.");
    }

    let mut total_processed = 0usize;
    let mut total_tags_removed = 0usize;

    for path in &targets {
        hygiene::validate_input_file(path, args.max_size_mb)?;

        let raw = fs::read(path)?;
        let pre_report = inspector::inspect_bytes(&raw);

        if args.inspect {
            inspector::print_report(path, &pre_report);
            continue;
        }

        let findings_before = pre_report.total_findings();
        if !args.quiet {
            println!(
                "{} Target: {} ({} bytes | {} metadata items detected)",
                "[*]".bright_cyan().bold(),
                path.display().bold(),
                raw.len(),
                findings_before.to_string().bright_yellow()
            );
        }

        let mode = if args.lossless {
            StripMode::LosslessContainer
        } else {
            StripMode::PixelReencode {
                jpeg_quality: args.quality,
            }
        };

        let (sanitized_bytes, _format) =
            stripper::strip_image_bytes(&raw, mode, pre_report.orientation)?;

        if args.verify {
            let post_report = inspector::inspect_bytes(&sanitized_bytes);
            if !post_report.is_clean() {
                bail!(
                    "Verification failed for '{}': {} metadata artifact(s) still detected!",
                    path.display(),
                    post_report.total_findings()
                );
            }
        }

        let out_path = hygiene::resolve_output_path(
            path,
            args.in_place,
            args.output.as_deref(),
            args.randomize_name,
        )?;

        hygiene::write_sanitized_atomic(
            path,
            &out_path,
            &sanitized_bytes,
            args.in_place,
            args.zero_timestamps,
        )?;

        total_processed += 1;
        total_tags_removed += findings_before;

        if !args.quiet {
            println!(
                "  {} Stripped {} metadata items -> {} ({} bytes){}",
                "[✓]".bright_green().bold(),
                findings_before,
                out_path.display().bright_green(),
                sanitized_bytes.len(),
                if args.verify {
                    " [VERIFIED CLEAN]".bright_green().bold().to_string()
                } else {
                    String::new()
                }
            );
        }
    }

    if !args.inspect && !args.quiet {
        println!(
            "\n{} Sanitized {} image(s), scrubbed {} metadata item(s). Zero trace remaining.",
            "[+]".bright_green().bold(),
            total_processed,
            total_tags_removed
        );
    }

    Ok(())
}

fn collect_targets(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for input in inputs {
        if input.is_dir() {
            for entry in fs::read_dir(input)? {
                let entry = entry?;
                let p = entry.path();
                if p.is_file() && is_supported_extension(&p) {
                    files.push(p);
                }
            }
        } else {
            files.push(input.clone());
        }
    }
    Ok(files)
}

fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "bmp" | "tif" | "tiff"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};
    use img_parts::jpeg::{Jpeg, JpegSegment};
    use img_parts::png::{Png, PngChunk};
    use img_parts::{Bytes, ImageEXIF};
    use std::io::Cursor;

    fn create_sample_jpeg_with_exif() -> Vec<u8> {
        let mut img = RgbImage::new(16, 16);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([42, 180, 220]);
        }
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
            .unwrap();

        let mut jpeg = Jpeg::from_bytes(Bytes::from(buf)).unwrap();
        // Minimal valid Big-Endian TIFF/EXIF header with 1 IFD entry (Make = "SpyCam")
        let fake_exif: Vec<u8> = vec![
            0x4D, 0x4D, 0x00, 0x2A, // "MM" big-endian + 42 magic
            0x00, 0x00, 0x00, 0x08, // Offset to IFD0
            0x00, 0x01, // 1 directory entry
            0x01, 0x0F, // Tag 0x010F (Make)
            0x00, 0x02, // Type ASCII
            0x00, 0x00, 0x00, 0x04, // Count 4
            b'S', b'P', b'Y', 0x00, // Inline value "SPY\0"
            0x00, 0x00, 0x00, 0x00, // Next IFD = 0
        ];
        jpeg.set_exif(Some(Bytes::from(fake_exif)));
        // Add a JPEG COM (0xFE) comment segment
        jpeg.segments_mut().insert(
            1,
            JpegSegment::new_with_contents(0xFE, Bytes::from_static(b"SecretAuthorComment")),
        );

        let mut out = Vec::new();
        jpeg.encoder().write_to(&mut out).unwrap();
        out
    }

    fn create_sample_png_with_chunks() -> Vec<u8> {
        let mut img = RgbImage::new(16, 16);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([10, 200, 100]);
        }
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();

        let mut png = Png::from_bytes(Bytes::from(buf)).unwrap();
        let text_chunk = PngChunk::new(*b"tEXt", Bytes::from_static(b"Author\0zhs4tra_secret"));
        png.chunks_mut().insert(1, text_chunk);

        let mut out = Vec::new();
        png.encoder().write_to(&mut out).unwrap();
        out
    }

    #[test]
    fn test_jpeg_reencode_strips_all_metadata() {
        let dirty_jpeg = create_sample_jpeg_with_exif();
        let pre = inspector::inspect_bytes(&dirty_jpeg);
        assert!(!pre.is_clean(), "Expected dirty JPEG to contain metadata");
        assert!(pre.has_exif_chunk);

        let (clean, _) = stripper::strip_image_bytes(
            &dirty_jpeg,
            StripMode::PixelReencode { jpeg_quality: 90 },
            pre.orientation,
        )
        .unwrap();

        let post = inspector::inspect_bytes(&clean);
        assert!(post.is_clean(), "Expected re-encoded JPEG to have zero metadata");
    }

    #[test]
    fn test_jpeg_lossless_strips_all_metadata() {
        let dirty_jpeg = create_sample_jpeg_with_exif();
        let pre = inspector::inspect_bytes(&dirty_jpeg);
        assert!(!pre.is_clean());

        let (clean, _) =
            stripper::strip_image_bytes(&dirty_jpeg, StripMode::LosslessContainer, None).unwrap();

        let post = inspector::inspect_bytes(&clean);
        assert!(post.is_clean(), "Expected lossless JPEG to have zero metadata");
    }

    #[test]
    fn test_png_lossless_and_reencode_strip_chunks() {
        let dirty_png = create_sample_png_with_chunks();
        let pre = inspector::inspect_bytes(&dirty_png);
        assert!(!pre.is_clean());

        let (clean_lossless, _) =
            stripper::strip_image_bytes(&dirty_png, StripMode::LosslessContainer, None).unwrap();
        assert!(inspector::inspect_bytes(&clean_lossless).is_clean());

        let (clean_reencoded, _) = stripper::strip_image_bytes(
            &dirty_png,
            StripMode::PixelReencode { jpeg_quality: 92 },
            None,
        )
        .unwrap();
        assert!(inspector::inspect_bytes(&clean_reencoded).is_clean());
    }
}
