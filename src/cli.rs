use clap::Parser;
use owo_colors::OwoColorize;
use std::path::PathBuf;

pub const ASCII_BANNER: &str = r#"
  ███████╗██╗  ██╗███████╗██╗  ██╗████████╗██████╗  █████╗ 
  ╚══███╔╝██║  ██║██╔════╝██║  ██║╚══██╔══╝██╔══██╗██╔══██╗
    ███╔╝ ███████║███████╗███████║   ██║   ██████╔╝███████║
   ███╔╝  ██╔══██║╚════██║╚════██║   ██║   ██╔══██╗██╔══██║
  ███████╗██║  ██║███████║     ██║   ██║   ██║  ██║██║  ██║
  ╚══════╝╚═╝  ╚═╝╚══════╝     ╚═╝   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝
"#;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "zhs4tra",
    version = "0.1.0",
    before_help = concat!(
        "  ███████╗██╗  ██╗███████╗██╗  ██╗████████╗██████╗  █████╗ \n",
        "  ╚══███╔╝██║  ██║██╔════╝██║  ██║╚══██╔══╝██╔══██╗██╔══██╗\n",
        "    ███╔╝ ███████║███████╗███████║   ██║   ██████╔╝███████║\n",
        "   ███╔╝  ██╔══██║╚════██║╚════██║   ██║   ██╔══██╗██╔══██║\n",
        "  ███████╗██║  ██║███████║     ██║   ██║   ██║  ██║██║  ██║\n",
        "  ╚══════╝╚═╝  ╚═╝╚══════╝     ╚═╝   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝\n",
        "  [ zhs4tra :: Zero-Trace Image Metadata Stripper & Anonymizer ]"
    ),
    about = "Zero-Trace Image Metadata Stripper & Anonymizer for Linux & Windows",
    long_about = "zhs4tra is a lightweight cybersecurity & anonymity CLI tool that completely strips identifying metadata (EXIF, GPS, XMP, IPTC, ICC profiles, comments, thumbnails) from images."
)]
pub struct Cli {
    /// Target image file(s) or directory to sanitize (e.g., zhs4tra <IMAGE>)
    #[arg(required = true, value_name = "IMAGE")]
    pub images: Vec<PathBuf>,

    /// Overwrite original file(s) in-place with zero-fill forensic scrubbing
    #[arg(short = 'i', long = "in-place")]
    pub in_place: bool,

    /// Custom output file or directory (defaults to <name>_clean.<ext> if not --in-place)
    #[arg(short = 'o', long = "output", value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Lossless container chunk stripping without pixel re-encoding (JPEG, PNG, WebP)
    #[arg(short = 'l', long = "lossless")]
    pub lossless: bool,

    /// Anonymize output filename using a cryptographically random 16-byte hex string
    #[arg(short = 'r', long = "randomize-name")]
    pub randomize_name: bool,

    /// Scrub filesystem timestamps (reset mtime/atime to Unix epoch 1970-01-01 00:00:00 UTC)
    #[arg(short = 't', long = "zero-timestamps")]
    pub zero_timestamps: bool,

    /// Inspect only: list detected metadata segments and EXIF tags without modifying the file
    #[arg(short = 'c', long = "inspect")]
    pub inspect: bool,

    /// Re-scan the sanitized output image to verify zero metadata remains
    #[arg(short = 'v', long = "verify", default_value_t = true)]
    pub verify: bool,

    /// JPEG encoding quality (1-100) when using default pixel re-encoding mode
    #[arg(long = "quality", default_value_t = 92)]
    pub quality: u8,

    /// Maximum allowed input file size in megabytes (DoS / decompression bomb protection)
    #[arg(long = "max-size-mb", default_value_t = 100)]
    pub max_size_mb: u64,

    /// Suppress the ASCII banner and informational logs
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
}

pub fn print_banner() {
    println!("{}", ASCII_BANNER.bright_cyan().bold());
    println!(
        "  {} {}",
        "[ zhs4tra :: Zero-Trace Image Metadata Stripper & Anonymizer ]".bright_green().bold(),
        "v0.1.0".bright_black()
    );
    println!(
        "  {}\n",
        "  OPSEC Mode: Zero Telemetry | Pure Rust | Cross-Platform (Linux/Windows)".bright_black()
    );
}
