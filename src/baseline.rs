//! Known-good snapshots and diffing against them.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub created_unix: u64,
    pub root: String,
    /// Keys are lowercase, root-relative paths with `/` separators.
    pub files: BTreeMap<String, Entry>,
}

#[derive(Debug, Default, Serialize)]
pub struct Diff {
    pub modified: Vec<Modified>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Modified {
    pub path: String,
    pub old_sha256: String,
    pub new_sha256: String,
}

impl Diff {
    pub fn is_clean(&self) -> bool {
        self.modified.is_empty() && self.added.is_empty() && self.removed.is_empty()
    }
}

impl Baseline {
    pub fn new(root: &Path, files: BTreeMap<String, Entry>) -> Self {
        Baseline {
            version: 1,
            created_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            root: root.display().to_string(),
            files,
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json).with_context(|| format!("cannot write {}", path.display()))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read baseline {}", path.display()))?;
        serde_json::from_str(&text).context("baseline file is not valid JSON")
    }
}

/// Normalised key for a file: lowercase, relative to `root`, forward slashes.
pub fn rel_key(root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file);
    rel.to_string_lossy().replace('\\', "/").to_lowercase()
}

/// Compare a saved baseline against the current state of the same tree.
pub fn diff(old: &Baseline, current: &BTreeMap<String, Entry>) -> Diff {
    let mut d = Diff::default();
    for (path, entry) in current {
        match old.files.get(path) {
            None => d.added.push(path.clone()),
            Some(prev) if prev.sha256 != entry.sha256 => d.modified.push(Modified {
                path: path.clone(),
                old_sha256: prev.sha256.clone(),
                new_sha256: entry.sha256.clone(),
            }),
            Some(_) => {}
        }
    }
    for path in old.files.keys() {
        if !current.contains_key(path) {
            d.removed.push(path.clone());
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(h: &str) -> Entry {
        Entry {
            sha256: h.into(),
            size: 1,
        }
    }

    fn map(items: &[(&str, &str)]) -> BTreeMap<String, Entry> {
        items
            .iter()
            .map(|(p, h)| (p.to_string(), entry(h)))
            .collect()
    }

    #[test]
    fn identical_trees_are_clean() {
        let base = Baseline::new(Path::new("."), map(&[("a.dll", "1"), ("b.exe", "2")]));
        let d = diff(&base, &map(&[("a.dll", "1"), ("b.exe", "2")]));
        assert!(d.is_clean());
    }

    #[test]
    fn detects_modified_added_removed() {
        let base = Baseline::new(Path::new("."), map(&[("a.dll", "1"), ("gone.exe", "3")]));
        let d = diff(&base, &map(&[("a.dll", "9"), ("new.exe", "4")]));
        assert_eq!(d.modified.len(), 1);
        assert_eq!(d.modified[0].path, "a.dll");
        assert_eq!(d.modified[0].old_sha256, "1");
        assert_eq!(d.added, vec!["new.exe"]);
        assert_eq!(d.removed, vec!["gone.exe"]);
    }

    #[test]
    fn rel_key_is_lowercase_and_relative() {
        let key = rel_key(
            Path::new("C:\\Windows"),
            Path::new("C:\\Windows\\System32\\Kernel32.DLL"),
        );
        assert_eq!(key, "system32/kernel32.dll");
    }

    #[test]
    fn roundtrips_through_json() {
        let base = Baseline::new(Path::new("."), map(&[("a.dll", "1")]));
        let json = serde_json::to_string(&base).unwrap();
        let back: Baseline = serde_json::from_str(&json).unwrap();
        assert_eq!(back.files, base.files);
    }
}
