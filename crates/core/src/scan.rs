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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

/// Hooks for long-running scans: cooperative cancellation and progress reporting.
pub struct Control<'a> {
    pub cancel: &'a AtomicBool,
    /// Called as `(done, total)` after every file, from worker threads.
    pub on_progress: &'a (dyn Fn(usize, usize) + Sync),
}

pub struct Outcome {
    pub reports: Vec<FileReport>,
    pub failures: Failures,
    /// True when the scan was stopped early; `reports` then holds a partial result.
    pub cancelled: bool,
}

/// Scan every PE file under `root` in parallel. Unreadable files are collected
/// separately so one locked file never aborts a run.
pub fn scan_with(root: &Path, opts: &ScanOptions, ctl: &Control) -> Result<Outcome> {
    let files = collect_pe_files(root)?;
    let total = files.len();
    let done = AtomicUsize::new(0);
    (ctl.on_progress)(0, total);

    let results: Vec<Option<std::result::Result<FileReport, (String, String)>>> = files
        .par_iter()
        .map(|p| {
            if ctl.cancel.load(Ordering::Relaxed) {
                return None;
            }
            let r = scan_file(p, opts).map_err(|e| (p.display().to_string(), e.to_string()));
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            (ctl.on_progress)(n, total);
            Some(r)
        })
        .collect();

    let mut reports = Vec::new();
    let mut failures = Vec::new();
    for r in results.into_iter().flatten() {
        match r {
            Ok(rep) => reports.push(rep),
            Err(e) => failures.push(e),
        }
    }
    reports.sort_by(|a, b| b.level.cmp(&a.level).then_with(|| a.path.cmp(&b.path)));
    Ok(Outcome {
        reports,
        failures,
        cancelled: ctl.cancel.load(Ordering::Relaxed),
    })
}

/// Convenience wrapper without progress or cancellation.
pub fn scan(root: &Path, opts: &ScanOptions) -> Result<(Vec<FileReport>, Failures)> {
    let cancel = AtomicBool::new(false);
    let out = scan_with(
        root,
        opts,
        &Control {
            cancel: &cancel,
            on_progress: &|_, _| {},
        },
    )?;
    Ok((out.reports, out.failures))
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

    fn temp_tree(name: &str, files: usize) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hashguard-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut pe = vec![0u8; 0x100];
        pe[0] = b'M';
        pe[1] = b'Z';
        pe[0x3C] = 0x80;
        pe[0x80..0x84].copy_from_slice(b"PE\0\0");
        for i in 0..files {
            std::fs::write(dir.join(format!("f{i}.exe")), &pe).unwrap();
        }
        dir
    }

    #[test]
    fn scan_reports_progress_for_every_file() {
        let dir = temp_tree("progress", 3);
        let cancel = AtomicBool::new(false);
        let calls = AtomicUsize::new(0);
        let out = scan_with(
            &dir,
            &ScanOptions::default(),
            &Control {
                cancel: &cancel,
                on_progress: &|_, total| {
                    assert_eq!(total, 3);
                    calls.fetch_add(1, Ordering::Relaxed);
                },
            },
        )
        .unwrap();
        assert_eq!(out.reports.len(), 3);
        assert!(!out.cancelled);
        assert_eq!(calls.load(Ordering::Relaxed), 4); // initial 0/3 plus one per file
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn cancelled_scan_stops_and_says_so() {
        let dir = temp_tree("cancel", 3);
        let cancel = AtomicBool::new(true);
        let out = scan_with(
            &dir,
            &ScanOptions::default(),
            &Control {
                cancel: &cancel,
                on_progress: &|_, _| {},
            },
        )
        .unwrap();
        assert!(out.cancelled);
        assert!(out.reports.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_path_is_an_error() {
        assert!(collect_pe_files(Path::new("Z:\\definitely\\not\\here")).is_err());
    }
}
