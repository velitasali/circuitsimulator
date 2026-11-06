//! Build script for `cs-engine`: packs `tests/fixtures/components` into a
//! Deflate-compressed ZIP archive (`$OUT_DIR/bundled_components.zip`) so clean
//! installations can seed their local `components` directory on first launch.

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::DeflateEncoder;

struct ZipEntry {
    rel_path: String,
    compression: u16,
    comp_size: u32,
    uncomp_size: u32,
    local_header_offset: u32,
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            println!("cargo:rerun-if-changed={}", path.display());
            collect_files(root, &path, out);
        } else if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((rel_str, path));
            }
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let comps_root = manifest_dir.join("tests/fixtures/components");
    println!("cargo:rerun-if-changed={}", comps_root.display());

    let mut files = Vec::new();
    collect_files(&comps_root, &comps_root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut zip_bytes: Vec<u8> = Vec::new();
    let mut entries: Vec<ZipEntry> = Vec::with_capacity(files.len());

    for (rel_path, full_path) in files {
        let raw = fs::read(&full_path).expect("failed to read component fixture file");
        let uncomp_size = raw.len() as u32;

        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(&raw)
            .expect("failed to deflate component fixture file");
        let compressed = encoder.finish().expect("failed to finish deflate stream");

        let (compression, payload) = if compressed.len() < raw.len() {
            (8u16, compressed)
        } else {
            (0u16, raw)
        };
        let comp_size = payload.len() as u32;
        let name_bytes = rel_path.as_bytes();
        let fn_len = name_bytes.len() as u16;
        let local_header_offset = zip_bytes.len() as u32;

        // Local file header (30 bytes + filename)
        zip_bytes.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]); // Signature
        zip_bytes.extend_from_slice(&20u16.to_le_bytes()); // Version needed
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Flags
        zip_bytes.extend_from_slice(&compression.to_le_bytes()); // Compression method
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Mod time
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Mod date
        zip_bytes.extend_from_slice(&0u32.to_le_bytes()); // CRC32 (not checked by extract_zip)
        zip_bytes.extend_from_slice(&comp_size.to_le_bytes());
        zip_bytes.extend_from_slice(&uncomp_size.to_le_bytes());
        zip_bytes.extend_from_slice(&fn_len.to_le_bytes());
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Extra length
        zip_bytes.extend_from_slice(name_bytes);
        zip_bytes.extend_from_slice(&payload);

        entries.push(ZipEntry {
            rel_path,
            compression,
            comp_size,
            uncomp_size,
            local_header_offset,
        });
    }

    let cd_offset = zip_bytes.len() as u32;
    for entry in &entries {
        let name_bytes = entry.rel_path.as_bytes();
        let fn_len = name_bytes.len() as u16;

        // Central directory file header (46 bytes + filename)
        zip_bytes.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]); // Signature
        zip_bytes.extend_from_slice(&20u16.to_le_bytes()); // Version made by
        zip_bytes.extend_from_slice(&20u16.to_le_bytes()); // Version needed
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Flags
        zip_bytes.extend_from_slice(&entry.compression.to_le_bytes());
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Mod time
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Mod date
        zip_bytes.extend_from_slice(&0u32.to_le_bytes()); // CRC32
        zip_bytes.extend_from_slice(&entry.comp_size.to_le_bytes());
        zip_bytes.extend_from_slice(&entry.uncomp_size.to_le_bytes());
        zip_bytes.extend_from_slice(&fn_len.to_le_bytes());
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Extra len
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Comment len
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Disk number
        zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Internal attrs
        zip_bytes.extend_from_slice(&0u32.to_le_bytes()); // External attrs
        zip_bytes.extend_from_slice(&entry.local_header_offset.to_le_bytes());
        zip_bytes.extend_from_slice(name_bytes);
    }

    let cd_size = (zip_bytes.len() as u32) - cd_offset;
    let num_entries = entries.len() as u16;

    // End of Central Directory record (22 bytes)
    zip_bytes.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]); // Signature
    zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Disk number
    zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // CD start disk
    zip_bytes.extend_from_slice(&num_entries.to_le_bytes());
    zip_bytes.extend_from_slice(&num_entries.to_le_bytes());
    zip_bytes.extend_from_slice(&cd_size.to_le_bytes());
    zip_bytes.extend_from_slice(&cd_offset.to_le_bytes());
    zip_bytes.extend_from_slice(&0u16.to_le_bytes()); // Comment len

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_zip = out_dir.join("bundled_components.zip");
    let mut out_file = File::create(&out_zip).expect("failed to create bundled_components.zip");
    out_file
        .write_all(&zip_bytes)
        .expect("failed to write bundled_components.zip");
}
