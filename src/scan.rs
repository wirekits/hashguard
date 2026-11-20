//! Directory walking and parallel scanning.

use crate::baseline::{rel_key, Entry};
use crate::hashing::sha256_file;
use crate::heuristics::{evaluate, Level};
use crate::pe::{is_pe_content, is_pe_file};
use crate::signature::{self, SigInfo};
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
pub struct FileReport {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub signature: SigInfo,
    pub level: Level,
    pub findings: Vec<String>,
}

/// `(path, error message)` pairs for files that could not be read.
pub type Failures = Vec<(String, String)>;

#[derive(Debug, Default)]
pub struct ScanOptions {
    pub blocklist: HashSet<String>,
}

/// Load a blocklist: one SHA-256 per line, `#` comments allowed.
pub fn load_blocklist(path: &Path) -> Result<HashSet<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read blocklist {}", path.display()))?;
    Ok(parse_blocklist(&text))
}

pub fn parse_blocklist(text: &str) -> HashSet<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim().to_lowercase())
        .filter(|l| l.len() == 64 && l.chars().all(|c| c.is_ascii_hexdigit()))
        .collect()
}

/// All PE files under `root` (or `root` itself when it is a file).
pub fn collect_pe_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        bail!("path does not exist: {}", root.display());
    }
    if root.is_file() {
        // An explicitly named file is checked by content, whatever it is called.
        return Ok(if is_pe_content(root) {
            vec![root.to_path_buf()]
        } else {
            vec![]
        });
    }
    Ok(WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| is_pe_file(p))
        .collect())
}

pub fn scan_file(path: &Path, opts: &ScanOptions) -> Result<FileReport> {
    let size = std::fs::metadata(path)?.len();
    let sha256 = sha256_file(path)?;
    let sig = signature::verify(path);
    let (level, findings) = evaluate(path, &sha256, &sig, &opts.blocklist);
    Ok(FileReport {
        path: path.display().to_string(),
        size,
        sha256,
        signature: sig,
        level,
        findings,
    })
}

/// Scan every PE file under `root` in parallel. Unreadable files are returned
/// separately so one locked file never aborts a run.
pub fn scan(root: &Path, opts: &ScanOptions) -> Result<(Vec<FileReport>, Failures)> {
    let files = collect_pe_files(root)?;
    let results: Vec<_> = files
        .par_iter()
        .map(|p| scan_file(p, opts).map_err(|e| (p.display().to_string(), e.to_string())))
        .collect();

    let mut reports = Vec::new();
    let mut errors = Vec::new();
    for r in results {
        match r {
            Ok(rep) => reports.push(rep),
            Err(e) => errors.push(e),
        }
    }
    reports.sort_by(|a, b| b.level.cmp(&a.level).then_with(|| a.path.cmp(&b.path)));
    Ok((reports, errors))
}

/// Hash every PE file under `root` for baseline creation / verification.
pub fn hash_tree(root: &Path) -> Result<(BTreeMap<String, Entry>, Failures)> {
    let files = collect_pe_files(root)?;
    let results: Vec<_> = files
        .par_iter()
        .map(|p| {
            let hashed = (|| -> Result<Entry> {
                Ok(Entry {
                    sha256: sha256_file(p)?,
                    size: std::fs::metadata(p)?.len(),
                })
            })();
            (p, hashed)
        })
        .collect();

    let mut map = BTreeMap::new();
    let mut errors = Vec::new();
    for (p, r) in results {
        match r {
            Ok(entry) => {
                map.insert(rel_key(root, p), entry);
            }
            Err(e) => errors.push((p.display().to_string(), e.to_string())),
        }
    }
    Ok((map, errors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocklist_parsing_ignores_junk_and_comments() {
        let h = "A".repeat(64);
        let text = format!(
            "# header\n{h}  # known bad\nnot-a-hash\n\n{}\n",
            "b".repeat(63)
        );
        let set = parse_blocklist(&text);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&"a".repeat(64)));
    }

    #[test]
    fn missing_path_is_an_error() {
        assert!(collect_pe_files(Path::new("Z:\\definitely\\not\\here")).is_err());
    }
}
