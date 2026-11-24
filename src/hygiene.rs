use anyhow::{bail, Context, Result};
use rand::RngCore;
use std::fs::{self, FileTimes, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Checks that the file exists, is a regular file, and does not exceed `max_size_mb`.
pub fn validate_input_file(path: &Path, max_size_mb: u64) -> Result<u64> {
    let meta = fs::symlink_metadata(path)
        .with_context(|| format!("Cannot access file: {}", path.display()))?;

    if meta.file_type().is_symlink() {
        bail!(
            "Security policy: refusing to follow symbolic link '{}'",
            path.display()
        );
    }
    if !meta.is_file() {
        bail!("Path is not a regular file: {}", path.display());
    }

    let max_bytes = max_size_mb.saturating_mul(1024 * 1024);
    let len = meta.len();
    if len == 0 {
        bail!("File is empty (0 bytes): {}", path.display());
    }
    if len > max_bytes {
        bail!(
            "File '{}' ({:.2} MB) exceeds maximum allowed size of {} MB (use --max-size-mb to override)",
            path.display(),
            len as f64 / (1024.0 * 1024.0),
            max_size_mb
        );
    }
    Ok(len)
}

/// Computes the destination file path based on CLI flags (`--in-place`, `--output`, `--randomize-name`).
pub fn resolve_output_path(
    input: &Path,
    in_place: bool,
    custom_output: Option<&Path>,
    randomize_name: bool,
) -> Result<PathBuf> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("img");

    if let Some(out) = custom_output {
        if out.is_dir() {
            let file_name = if randomize_name {
                format!("{}.{}", random_hex_name(), ext)
            } else {
                let stem = input
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("image");
                format!("{}_clean.{}", stem, ext)
            };
            return Ok(out.join(file_name));
        }
        return Ok(out.to_path_buf());
    }

    let parent = input.parent().unwrap_or_else(|| Path::new("."));

    if randomize_name {
        return Ok(parent.join(format!("{}.{}", random_hex_name(), ext)));
    }

    if in_place {
        return Ok(input.to_path_buf());
    }

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let clean_stem = stem.strip_suffix("_clean").unwrap_or(stem);
    Ok(parent.join(format!("{}_clean.{}", clean_stem, ext)))
}

/// Atomically writes sanitized bytes via a temporary sibling file and optionally scrubs the original file.
pub fn write_sanitized_atomic(
    original_path: &Path,
    target_path: &Path,
    sanitized_bytes: &[u8],
    in_place: bool,
    zero_timestamps: bool,
) -> Result<()> {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_name = format!(".zhs4tra_tmp_{}", random_hex_name());
    let tmp_path = parent.join(tmp_name);

    // 1. Write sanitized bytes to temporary file and flush to disk
    {
        let mut tmp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .with_context(|| format!("Failed to create temporary file: {}", tmp_path.display()))?;
        tmp_file.write_all(sanitized_bytes)?;
        tmp_file.sync_all()?;
    }

    // 2. If overwriting in-place (or replacing original after rename), zero-fill the original bytes first
    if in_place && original_path.exists() {
        let _ = zero_fill_file(original_path);
        if original_path != target_path {
            let _ = fs::remove_file(original_path);
        }
    }

    // 3. Atomic rename of temporary file to target destination
    if let Err(e) = fs::rename(&tmp_path, target_path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e).with_context(|| {
            format!(
                "Failed to move sanitized file to destination: {}",
                target_path.display()
            )
        });
    }

    // 4. Normalize filesystem timestamps if requested
    if zero_timestamps {
        scrub_timestamps(target_path)?;
    }

    Ok(())
}

/// Overwrites an existing file's data blocks with zeros before replacement to hinder simple carving.
fn zero_fill_file(path: &Path) -> Result<()> {
    let len = fs::metadata(path)?.len();
    if len == 0 {
        return Ok(());
    }
    let mut file = OpenOptions::new().write(true).open(path)?;
    let chunk = [0u8; 8192];
    let mut remaining = len;
    while remaining > 0 {
        let to_write = (remaining as usize).min(chunk.len());
        file.write_all(&chunk[..to_write])?;
        remaining -= to_write as u64;
    }
    file.sync_all()?;
    Ok(())
}

/// Resets file modification and access timestamps to Unix Epoch (1970-01-01 00:00:00 UTC).
pub fn scrub_timestamps(path: &Path) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("Failed to open {} for timestamp scrubbing", path.display()))?;
    let times = FileTimes::new()
        .set_accessed(UNIX_EPOCH)
        .set_modified(UNIX_EPOCH);
    file.set_times(times)
        .with_context(|| format!("Failed to scrub timestamps on {}", path.display()))
}

pub fn random_hex_name() -> String {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
