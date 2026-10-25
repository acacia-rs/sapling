//! A pack on disk, a folder or a zip (`.mcpack`, `.zip`), read whole: its manifest and its files.

use std::fs;
use std::path::{Path, PathBuf};

use super::archive::{self, Files};
use super::manifest::Manifest;
use super::PackError;

const MANIFEST: &str = "manifest.json";

pub(crate) struct Source {
    pub manifest: Manifest,
    /// Paths from the pack's root, the folder its manifest is in.
    pub files: Files,
    /// The zip as it was given, when its root is the pack's root.
    zip: Option<Vec<u8>>,
}

impl Source {
    /// The pack as one zip, as clients download it: the file it came in, or its files zipped.
    pub fn into_zip(self) -> Vec<u8> {
        self.zip.unwrap_or_else(|| archive::write(&self.files))
    }

    pub fn file(&self, path: &str) -> Option<&[u8]> {
        self.files.iter().find(|(name, _)| name == path).map(|(_, data)| data.as_slice())
    }
}

fn invalid(path: &Path, reason: impl Into<String>) -> PackError {
    PackError::Invalid { path: path.to_owned(), reason: reason.into() }
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> PackError + '_ {
    move |source| PackError::Io { path: path.to_owned(), source }
}

/// Whether `path` looks like a pack: a folder, or a file named `.mcpack` or `.zip`.
pub(crate) fn is_pack(path: &Path) -> bool {
    path.is_dir() || path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("mcpack") || e.eq_ignore_ascii_case("zip"))
}

/// The packs in `folder`, by file name. A folder with no manifest is skipped (a `.git`, a note).
pub(crate) fn scan(folder: &Path) -> Result<Vec<Source>, PackError> {
    let mut paths: Vec<PathBuf> = fs::read_dir(folder).map_err(io(folder))?.map(|entry| entry.map(|e| e.path())).collect::<Result<_, _>>().map_err(io(folder))?;
    paths.sort();
    let mut packs = Vec::new();
    for path in paths.into_iter().filter(|path| is_pack(path)) {
        if path.is_dir() && !path.join(MANIFEST).is_file() {
            continue;
        }
        packs.push(open(&path)?);
    }
    Ok(packs)
}

pub(crate) fn open(path: &Path) -> Result<Source, PackError> {
    let (files, zip) = if path.is_dir() {
        let mut files = Vec::new();
        walk(path, "", &mut files)?;
        (files, None)
    } else {
        let zip = fs::read(path).map_err(io(path))?;
        (archive::read(&zip).map_err(|reason| invalid(path, reason))?, Some(zip))
    };
    let root = files.iter().filter_map(|(name, _)| name.strip_suffix(MANIFEST).filter(|dir| dir.is_empty() || dir.ends_with('/'))).min_by_key(|dir| dir.len()).map(str::to_owned);
    let root = root.ok_or_else(|| invalid(path, "no manifest.json"))?;
    let files: Files = files.into_iter().filter_map(|(name, data)| Some((name.strip_prefix(root.as_str())?.to_owned(), data))).collect();
    let manifest = files.iter().find(|(name, _)| name == MANIFEST).map(|(_, data)| Manifest::parse(data)).expect("the root holds a manifest");
    let manifest = manifest.map_err(|reason| invalid(path, format!("manifest.json: {reason}")))?;
    Ok(Source { manifest, files, zip: zip.filter(|_| root.is_empty()) })
}

fn walk(dir: &Path, prefix: &str, files: &mut Files) -> Result<(), PackError> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir).map_err(io(dir))?.map(|entry| entry.map(|e| e.path())).collect::<Result<_, _>>().map_err(io(dir))?;
    entries.sort();
    for path in entries {
        let name = format!("{prefix}{}", path.file_name().unwrap_or_default().to_string_lossy());
        if path.is_dir() {
            walk(&path, &format!("{name}/"), files)?;
        } else {
            files.push((name, fs::read(&path).map_err(io(&path))?));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST_JSON: &[u8] = br#"{"header":{"name":"t","uuid":"5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001","version":[1,0,0]},"modules":[{"type":"data"}]}"#;

    #[test]
    fn a_zip_with_its_pack_in_a_folder_is_rooted_there() {
        let dir = std::env::temp_dir().join(format!("sapling-source-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let nested = archive::write(&[("pack/manifest.json".into(), MANIFEST_JSON.to_vec()), ("pack/functions/a.mcfunction".into(), b"say a".to_vec()), ("readme.txt".into(), Vec::new())]);
        fs::write(dir.join("nested.mcpack"), &nested).unwrap();
        fs::write(dir.join("notes.txt"), b"not a pack").unwrap();
        let packs = scan(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].file("functions/a.mcfunction"), Some(&b"say a"[..]));
        let zip = packs.into_iter().next().unwrap().into_zip();
        assert_ne!(zip, nested, "rezipped from its root");
        assert_eq!(archive::read(&zip).unwrap().len(), 2);
    }
}
