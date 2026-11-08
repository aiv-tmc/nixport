//! On-disk layout of a `.nixport` archive.
//!
//! ```text
//! MAGIC (8 bytes)
//! manifest length (u64 LE) | manifest bytes
//! icon length     (u64 LE) | raw icon bytes (may be empty)
//! config length   (u64 LE) | zstd-compressed tar of the settings
//! zstd-compressed `nix-store --export` stream, up to the end of the file
//! ```
//!
//! The manifest and the icon sit in front of the bulky payload, so a file
//! manager can read the name and icon without touching the rest.

use crate::error::{fail, Context, Result};
use crate::meta::Meta;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Stdio;

const MAGIC: [u8; 8] = *b"NIXPORT\x01";

/// Upper bound for the manifest and the icon; protects against corrupt headers.
const MAX_BLOCK: u64 = 64 << 20;

/// Parsed archive header together with the offsets of the payload sections.
pub struct Layout {
    pub meta: Meta,
    pub icon: Vec<u8>,
    pub config_off: u64,
    pub config_len: u64,
    pub store_off: u64,
}

fn read_u64(file: &mut File) -> Result<u64> {
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf).context("the archive is truncated")?;
    Ok(u64::from_le_bytes(buf))
}

/// Reads a length-prefixed block.
fn read_block(file: &mut File) -> Result<Vec<u8>> {
    let len = read_u64(file)?;
    if len > MAX_BLOCK {
        return fail("the archive header is corrupt");
    }
    let mut buf = vec![0u8; len as usize];
    file.read_exact(&mut buf).context("the archive is truncated")?;
    Ok(buf)
}

fn write_block(file: &mut File, data: &[u8]) -> Result<()> {
    file.write_all(&(data.len() as u64).to_le_bytes())?;
    file.write_all(data)?;
    Ok(())
}

/// Reads the header of the archive at `path`.
pub fn read(path: &Path) -> Result<Layout> {
    let mut file = File::open(path).context(&format!("cannot open {}", path.display()))?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic).context("the archive is truncated")?;
    if magic != MAGIC {
        return fail(format!("{} is not a Nixport archive", path.display()));
    }
    let meta = Meta::decode(&read_block(&mut file)?)?;
    let icon = read_block(&mut file)?;
    let config_len = read_u64(&mut file)?;
    let config_off = file.stream_position()?;
    let Some(store_off) = config_off.checked_add(config_len) else {
        return fail("the archive header is corrupt");
    };
    Ok(Layout {
        meta,
        icon,
        config_off,
        config_len,
        store_off,
    })
}

/// Writes an archive front to back. The two payload sections are produced by
/// child processes that write straight into the file.
pub struct Writer {
    file: File,
}

impl Writer {
    /// Creates the file and writes the manifest and the icon.
    pub fn create(path: &Path, meta: &Meta, icon: &[u8]) -> Result<Writer> {
        let mut file = File::create(path).context(&format!("cannot create {}", path.display()))?;
        file.write_all(&MAGIC)?;
        write_block(&mut file, &meta.encode())?;
        write_block(&mut file, icon)?;
        Ok(Writer { file })
    }

    /// A handle for a child process to write the next section into. The handle
    /// shares the file offset with the writer, so output lands in order.
    pub fn sink(&self) -> Result<Stdio> {
        let handle = self.file.try_clone().context("cannot duplicate the archive handle")?;
        Ok(Stdio::from(handle))
    }

    /// Reserves the length field of the settings section and returns its position.
    pub fn begin_config(&mut self) -> Result<u64> {
        let pos = self.file.stream_position()?;
        self.file.write_all(&0u64.to_le_bytes())?;
        Ok(pos)
    }

    /// Fills in the settings length and moves back to the end of the file.
    pub fn end_config(&mut self, pos: u64) -> Result<()> {
        let end = self.file.stream_position()?;
        let len = end - (pos + 8);
        self.file.seek(SeekFrom::Start(pos))?;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.seek(SeekFrom::Start(end))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::tests::sample;

    fn temp_file(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nixport-{tag}-{}.nixport", std::process::id()))
    }

    #[test]
    fn header_round_trips_and_sections_are_located() {
        let path = temp_file("header");
        let meta = sample();
        let mut writer = Writer::create(&path, &meta, b"icon-bytes").unwrap();
        let pos = writer.begin_config().unwrap();
        writer.end_config(pos).unwrap();

        let layout = read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(layout.meta.id, meta.id);
        assert_eq!(layout.icon, b"icon-bytes");
        assert_eq!(layout.config_len, 0);
        assert_eq!(layout.store_off, layout.config_off);
    }

    #[test]
    fn foreign_files_are_rejected() {
        let path = temp_file("foreign");
        std::fs::write(&path, b"this is not an archive").unwrap();
        let result = read(&path);
        std::fs::remove_file(&path).unwrap();
        assert!(result.is_err());
    }
}
