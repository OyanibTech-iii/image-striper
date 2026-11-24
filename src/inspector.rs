use img_parts::jpeg::Jpeg;
use img_parts::png::Png;
use img_parts::webp::WebP;
use img_parts::{Bytes, ImageEXIF, ImageICC};
use owo_colors::OwoColorize;
use std::io::Cursor;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct MetadataReport {
    pub exif_fields: Vec<(String, String, bool)>, // (Tag, Value, is_high_risk)
    pub has_exif_chunk: bool,
    pub has_icc_profile: bool,
    pub ancillary_chunks: Vec<String>,
    pub orientation: Option<u32>,
}

impl MetadataReport {
    pub fn is_clean(&self) -> bool {
        self.exif_fields.is_empty()
            && !self.has_exif_chunk
            && !self.has_icc_profile
            && self.ancillary_chunks.is_empty()
    }

    pub fn total_findings(&self) -> usize {
        self.exif_fields.len()
            + usize::from(self.has_exif_chunk)
            + usize::from(self.has_icc_profile)
            + self.ancillary_chunks.len()
    }
}

/// Scans an image byte buffer for EXIF tags, GPS data, ICC profiles, comments, and container metadata chunks.
pub fn inspect_bytes(raw: &[u8]) -> MetadataReport {
    let mut report = MetadataReport::default();

    // 1. Parse EXIF table using kamadak-exif
    let mut cursor = Cursor::new(raw);
    if let Ok(exif_reader) = exif::Reader::new().read_from_container(&mut cursor) {
        for field in exif_reader.fields() {
            let tag_name = format!("{}", field.tag);
            let value_str = field.display_value().with_unit(&exif_reader).to_string();

            if field.tag == exif::Tag::Orientation {
                if let Some(v) = field.value.get_uint(0) {
                    report.orientation = Some(v);
                }
            }

            let is_high_risk = is_high_risk_tag(field.tag, &tag_name);
            report.exif_fields.push((tag_name, value_str, is_high_risk));
        }
    }

    // 2. Inspect container-level chunks (JPEG APP/COM, PNG ancillary chunks, WebP EXIF/XMP/ICC)
    let bytes = Bytes::copy_from_slice(raw);
    if let Ok(jpeg) = Jpeg::from_bytes(bytes.clone()) {
        report.has_exif_chunk = jpeg.exif().is_some();
        report.has_icc_profile = jpeg.icc_profile().is_some();
        for seg in jpeg.segments() {
            let marker = seg.marker();
            // APP1..APP15 (0xE1..=0xEF) or COM (0xFE)
            if (0xE1..=0xEF).contains(&marker) || marker == 0xFE {
                let label = match marker {
                    0xE1 => "JPEG APP1 (EXIF/XMP)",
                    0xE2 => "JPEG APP2 (ICC Profile/FPXR)",
                    0xEC => "JPEG APP12 (Picture Info/Ducky)",
                    0xED => "JPEG APP13 (IPTC/Photoshop)",
                    0xEE => "JPEG APP14 (Adobe)",
                    0xFE => "JPEG COM (Comment)",
                    m => {
                        report
                            .ancillary_chunks
                            .push(format!("JPEG Marker 0x{:02X} ({} bytes)", m, seg.contents().len()));
                        continue;
                    }
                };
                report
                    .ancillary_chunks
                    .push(format!("{} ({} bytes)", label, seg.contents().len()));
            }
        }
    } else if let Ok(png) = Png::from_bytes(bytes.clone()) {
        report.has_exif_chunk = png.exif().is_some();
        report.has_icc_profile = png.icc_profile().is_some();
        for chunk in png.chunks() {
            let kind_bytes = chunk.kind();
            let kind = std::str::from_utf8(&kind_bytes).unwrap_or("????");
            // Critical PNG chunks: IHDR, PLTE, IDAT, IEND, plus transparency tRNS
            if !matches!(kind, "IHDR" | "PLTE" | "IDAT" | "IEND" | "tRNS") {
                report
                    .ancillary_chunks
                    .push(format!("PNG Chunk '{}' ({} bytes)", kind, chunk.contents().len()));
            }
        }
    } else if let Ok(webp) = WebP::from_bytes(bytes) {
        report.has_exif_chunk = webp.exif().is_some();
        report.has_icc_profile = webp.icc_profile().is_some();
        for chunk in webp.chunks() {
            let id_bytes = chunk.id();
            let id = std::str::from_utf8(&id_bytes).unwrap_or("????");
            if matches!(id, "EXIF" | "XMP " | "ICCP") {
                report
                    .ancillary_chunks
                    .push(format!("WebP Chunk '{}' ({} bytes)", id, chunk.content().len()));
            }
        }
    }

    report
}

pub fn print_report(path: &Path, report: &MetadataReport) {
    println!(
        "{} Inspecting: {}",
        "[*]".bright_blue().bold(),
        path.display().bold()
    );

    if report.is_clean() {
        println!(
            "  {} No EXIF tags, ICC profiles, or ancillary metadata chunks found.",
            "[✓]".bright_green().bold()
        );
        return;
    }

    if !report.ancillary_chunks.is_empty() {
        println!(
            "  {} Container Metadata Segments ({}):",
            "[!]".bright_yellow().bold(),
            report.ancillary_chunks.len()
        );
        for chunk in &report.ancillary_chunks {
            println!("      - {}", chunk.yellow());
        }
    }

    if !report.exif_fields.is_empty() {
        println!(
            "  {} Decoded EXIF Fields ({}):",
            "[!]".bright_red().bold(),
            report.exif_fields.len()
        );
        for (tag, val, high_risk) in &report.exif_fields {
            if *high_risk {
                println!(
                    "      {} {:<26} : {}",
                    "[HIGH-RISK]".bright_red().bold(),
                    tag.bright_red(),
                    val.bright_white()
                );
            } else {
                println!("      {:<38} : {}", tag, val.dimmed());
            }
        }
    }
}

fn is_high_risk_tag(tag: exif::Tag, name: &str) -> bool {
    tag.0 == exif::Context::Gps
        || name.contains("GPS")
        || name.contains("Make")
        || name.contains("Model")
        || name.contains("Serial")
        || name.contains("Owner")
        || name.contains("Artist")
        || name.contains("Author")
        || name.contains("Copyright")
        || name.contains("Software")
        || name.contains("DateTime")
        || name.contains("HostComputer")
        || name.contains("MakerNote")
        || name.contains("UserComment")
        || name.contains("UniqueID")
}
