//! The zip files packs come in (`.mcpack`, `.zip`): every file read out of one, and a folder's
//! files written into one for clients to download. Stored and deflated entries only; no zip64.

use std::io::{Read, Write};

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::{Compression, Crc};

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
const END_LEN: usize = 22;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;
const ENCRYPTED: u16 = 1;
/// Names are UTF-8.
const UTF8: u16 = 1 << 11;
/// 1980-01-01, the earliest date a zip holds: written files carry no time, so a folder zips the same each start.
const DOS_DATE: u16 = 0x21;

/// A file of a pack: its path inside the pack, with `/` between folders.
pub(crate) type Files = Vec<(String, Vec<u8>)>;

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    bytes.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| "cut short".to_owned())
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| "cut short".to_owned())
}

/// Every file in a zip, folders left out.
pub(crate) fn read(zip: &[u8]) -> Result<Files, String> {
    let lowest = zip.len().saturating_sub(END_LEN + usize::from(u16::MAX));
    let end = (lowest..=zip.len().saturating_sub(END_LEN)).rev().find(|&at| u32_at(zip, at) == Ok(END)).ok_or("not a zip file")?;
    let count = usize::from(u16_at(zip, end + 10)?);
    let mut at = u32_at(zip, end + 16)? as usize;
    let mut files = Vec::with_capacity(count);
    for _ in 0..count {
        if u32_at(zip, at)? != CENTRAL {
            return Err("a broken central directory".into());
        }
        let (flags, method) = (u16_at(zip, at + 8)?, u16_at(zip, at + 10)?);
        let (packed, size) = (u32_at(zip, at + 20)? as usize, u32_at(zip, at + 24)? as usize);
        let name_len = usize::from(u16_at(zip, at + 28)?);
        let skip = name_len + usize::from(u16_at(zip, at + 30)?) + usize::from(u16_at(zip, at + 32)?);
        let local = u32_at(zip, at + 42)? as usize;
        let name = String::from_utf8_lossy(zip.get(at + 46..at + 46 + name_len).ok_or("cut short")?).replace('\\', "/");
        at += 46 + skip;
        if name.ends_with('/') {
            continue;
        }
        if flags & ENCRYPTED != 0 {
            return Err(format!("{name} is password-protected"));
        }
        if u32_at(zip, local)? != LOCAL {
            return Err(format!("{name}: a broken local header"));
        }
        let start = local + 30 + usize::from(u16_at(zip, local + 26)?) + usize::from(u16_at(zip, local + 28)?);
        let raw = zip.get(start..start + packed).ok_or_else(|| format!("{name} is cut short"))?;
        let data = match method {
            STORED => raw.to_vec(),
            DEFLATED => {
                let mut data = Vec::with_capacity(size);
                DeflateDecoder::new(raw).read_to_end(&mut data).map_err(|e| format!("{name}: {e}"))?;
                data
            }
            other => return Err(format!("{name} is packed with method {other}, which sapling does not read")),
        };
        files.push((name, data));
    }
    Ok(files)
}

/// A zip of `files`, deflated as BDS's are: fewer chunks to send.
pub(crate) fn write(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    write_with(files, true)
}

fn write_with(files: &[(String, Vec<u8>)], deflate: bool) -> Vec<u8> {
    let mut zip = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let mut crc = Crc::new();
        crc.update(data);
        let packed = if deflate {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data).expect("writing to memory");
            encoder.finish().expect("writing to memory")
        } else {
            data.clone()
        };
        let method = if deflate { DEFLATED } else { STORED };
        let offset = zip.len() as u32;
        let header = |zip: &mut Vec<u8>, central: bool| {
            zip.extend_from_slice(&(if central { CENTRAL } else { LOCAL }).to_le_bytes());
            if central {
                zip.extend_from_slice(&20u16.to_le_bytes());
            }
            for half in [20, UTF8, method, 0, DOS_DATE] {
                zip.extend_from_slice(&half.to_le_bytes());
            }
            for word in [crc.sum(), packed.len() as u32, data.len() as u32] {
                zip.extend_from_slice(&word.to_le_bytes());
            }
            zip.extend_from_slice(&(name.len() as u16).to_le_bytes());
            zip.extend_from_slice(&0u16.to_le_bytes());
            if central {
                zip.extend_from_slice(&[0; 10]);
                zip.extend_from_slice(&offset.to_le_bytes());
            }
            zip.extend_from_slice(name.as_bytes());
        };
        header(&mut zip, false);
        zip.extend_from_slice(&packed);
        header(&mut central, true);
    }
    let (central_at, central_len) = (zip.len() as u32, central.len() as u32);
    zip.append(&mut central);
    zip.extend_from_slice(&END.to_le_bytes());
    zip.extend_from_slice(&[0; 4]);
    zip.extend_from_slice(&(files.len() as u16).to_le_bytes());
    zip.extend_from_slice(&(files.len() as u16).to_le_bytes());
    zip.extend_from_slice(&central_len.to_le_bytes());
    zip.extend_from_slice(&central_at.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_written_reads_back() {
        let files = vec![("manifest.json".to_owned(), b"{}".to_vec()), ("functions/a/b.mcfunction".to_owned(), b"say hi\n".to_vec()), ("empty".to_owned(), Vec::new())];
        assert_eq!(read(&write(&files)).unwrap(), files);
        assert!(read(b"not a zip at all, longer than an end record").is_err());
    }

    #[test]
    fn a_stored_entry_reads_and_a_written_one_is_packed() {
        let files = vec![("padding.bin".to_owned(), vec![b'p'; 100_000])];
        assert!(write(&files).len() < 1_000);
        assert_eq!(read(&write_with(&files, false)).unwrap(), files);
    }
}
