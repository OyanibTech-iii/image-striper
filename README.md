# `zhs4tra` — Zero-Trace Image Metadata Stripper & Anonymizer

```text
  ███████╗██╗  ██╗███████╗██╗  ██╗████████╗██████╗  █████╗ 
  ╚══███╔╝██║  ██║██╔════╝██║  ██║╚══██╔══╝██╔══██╗██╔══██╗
    ███╔╝ ███████║███████╗███████║   ██║   ██████╔╝███████║
   ███╔╝  ██╔══██║╚════██║╚════██║   ██║   ██╔══██╗██╔══██║
  ███████╗██║  ██║███████║     ██║   ██║   ██║  ██║██║  ██║
  ╚══════╝╚═╝  ╚═╝╚══════╝     ╚═╝   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝
  [ zhs4tra :: Zero-Trace Image Metadata Stripper & Anonymizer ]
```

`zhs4tra` is a lightweight, zero-telemetry cybersecurity and OPSEC CLI tool written in 100% safe Rust (`#![forbid(unsafe_code)]`) for **Linux** and **Windows**. It completely strips identifying metadata (EXIF, GPS coordinates, camera serial numbers, XMP, IPTC, ICC color profiles, embedded thumbnails, and comments) from images.

---

## Features & OPSEC Design

- **Defense-in-Depth Dual Stripping Engine**:
  - **Default Mode (Pixel-Level Re-encoding)**: Decodes raw pixels, applies EXIF orientation in memory so visual rotation is preserved without retaining the orientation tag, and encodes a brand-new container guaranteed to carry zero leftover metadata or trailing bytes.
  - **Lossless Container Mode (`-l / --lossless`)**: Parses JPEG (`APP1`–`APP15`, `COM`), PNG (`eXIf`, `tEXt`, `zTXt`, `iTXt`, `tIME`, `iCCP`, `pHYs`), and WebP (`EXIF`, `XMP `, `ICCP`) chunks directly and drops all non-essential segments without recompressing pixel data.
- **Automated Post-Sanitization Verification**: Automatically re-scans the output buffer to guarantee zero EXIF tags, ICC profiles, or ancillary chunks remain before committing to disk.
- **Forensic & Filesystem Hygiene**:
  - **Atomic Write + Zero-Fill Scrub (`-i / --in-place`)**: Writes sanitized bytes to a randomized temporary sibling file and overwrites the original file's data blocks with zeros before unlinking/renaming.
  - **Timestamp Scrubbing (`-t / --zero-timestamps`)**: Resets `mtime` and `atime` to Unix Epoch (`1970-01-01 00:00:00 UTC`).
  - **Filename Anonymization (`-r / --randomize-name`)**: Replaces camera-specific filenames (`IMG_20261124_142201.JPG`, `DSC_0042.JPG`) with cryptographically random 24-character hex filenames.
  - **Symlink & DoS Protection**: Refuses to follow symbolic links and enforces a configurable maximum file size limit (`--max-size-mb`, default `100 MB`).

---

## Build & Installation (Linux & Windows)

### Linux (Debian / Ubuntu / Kali / Arch / Fedora)
```bash
# 1. Install Rust toolchain if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Build stripped, LTO-optimized release binary
cargo build --release

# 3. Install to /usr/local/bin
sudo install -m 0755 target/release/zhs4tra /usr/local/bin/zhs4tra
```

### Static Musl Build for Portable Linux Deployment
```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

### Windows
```powershell
cargo build --release
.\target\release\zhs4tra.exe <IMAGE>
```

---

## Usage Examples

```bash
# Basic metadata strip (outputs <IMAGE>_clean.<ext> and verifies zero metadata remains)
zhs4tra photo.jpg

# Strip metadata in-place (zero-fills original file before replacing)
zhs4tra -i photo.jpg

# Maximum OPSEC mode: in-place strip + randomized filename + zeroed Unix epoch timestamps
zhs4tra -i -r -t photo.jpg

# Lossless container chunk stripping (no pixel recompression)
zhs4tra --lossless photo.jpg

# Inspect an image for high-risk metadata (GPS, Make/Model, Serial, Software) without modifying
zhs4tra --inspect photo.jpg

# Batch sanitize all images in a folder quietly
zhs4tra -q -i ./evidence_photos/
```
# image-striper
