//! Rules that turn (path, hash, signature) into a verdict.
//!
//! These are indicators, not proof: a clean result does not mean a file is safe,
//! and a finding means "look at this", not "this is malware".

use crate::signature::{SigInfo, SigStatus};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Level {
    Ok,
    Info,
    Warn,
    Danger,
}

/// Processes attackers like to imitate, because users and tools ignore them.
const CORE_PROCESSES: &[&str] = &[
    "svchost.exe",
    "lsass.exe",
    "csrss.exe",
    "services.exe",
    "winlogon.exe",
    "wininit.exe",
    "smss.exe",
    "explorer.exe",
    "taskhostw.exe",
    "spoolsv.exe",
    "dllhost.exe",
    "conhost.exe",
    "rundll32.exe",
];

const DECOY_EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "xls", "xlsx", "txt", "jpg", "jpeg", "png", "zip", "mp3", "mp4",
];

const USER_WRITABLE_MARKERS: &[&str] = &[
    "\\appdata\\",
    "\\temp\\",
    "\\downloads\\",
    "\\users\\public\\",
    "\\programdata\\",
];

/// Lowercase, backslash-separated path without the `\\?\` prefix.
pub fn normalize(path: &Path) -> String {
    let s = path.to_string_lossy().to_lowercase().replace('/', "\\");
    s.strip_prefix("\\\\?\\").map(str::to_string).unwrap_or(s)
}

/// True for the real Windows system directories.
pub fn is_system_location(path: &Path) -> bool {
    let p = normalize(path);
    let parent = p.rsplit_once('\\').map(|(dir, _)| dir).unwrap_or("");
    // Must be `<drive>:\windows...` so `C:\Users\x\Windows` does not count.
    let Some(rest) = parent.get(1..).and_then(|s| s.strip_prefix(':')) else {
        return false;
    };
    matches!(
        rest,
        "\\windows" | "\\windows\\system32" | "\\windows\\syswow64"
    ) || rest.starts_with("\\windows\\system32\\")
        || rest.starts_with("\\windows\\syswow64\\")
        || rest.starts_with("\\windows\\winsxs\\")
}

fn is_user_writable(path: &Path) -> bool {
    let p = normalize(path);
    USER_WRITABLE_MARKERS.iter().any(|m| p.contains(m))
}

/// Hardware and driver vendors whose valid signatures in system folders are normal.
const TRUSTED_VENDORS: &[&str] = &[
    "nvidia",
    "advanced micro devices",
    "intel",
    "realtek",
    "qualcomm",
    "broadcom",
    "mediatek",
    "marvell",
    "logitech",
    "dolby",
    "synaptics",
    "elan microelectronics",
    "asmedia",
    "lunarg",
    "vmware",
    "oracle",
    "razer",
    "corsair",
    "steelseries",
    "samsung electronics",
    "western digital",
    "seagate",
    "dell",
    "hp inc",
    "lenovo",
    "asustek",
    "micro-star",
    "gigabyte",
];

fn is_trusted_vendor(signer: &Option<String>) -> bool {
    signer
        .as_deref()
        .map(str::to_lowercase)
        .is_some_and(|s| TRUSTED_VENDORS.iter().any(|v| s.contains(v)))
}

fn is_microsoft(signer: &Option<String>) -> bool {
    signer
        .as_deref()
        .map(|s| s.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

/// Extensions a user would double-click to run (a DLL named `x.pdf.dll` is not a lure).
const LAUNCHABLE_EXTENSIONS: &[&str] = &["exe", "scr", "com", "pif", "cpl"];

/// `invoice.pdf.exe` style names.
fn has_decoy_double_extension(path: &Path) -> bool {
    let launchable = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            LAUNCHABLE_EXTENSIONS
                .iter()
                .any(|x| x.eq_ignore_ascii_case(e))
        })
        .unwrap_or(false);
    if !launchable {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    match stem.rsplit_once('.') {
        Some((_, inner)) => DECOY_EXTENSIONS
            .iter()
            .any(|e| e.eq_ignore_ascii_case(inner)),
        None => false,
    }
}

pub fn evaluate(
    path: &Path,
    sha256: &str,
    sig: &SigInfo,
    blocklist: &HashSet<String>,
) -> (Level, Vec<String>) {
    let mut level = Level::Ok;
    let mut findings = Vec::new();
    let mut flag = |l: Level, msg: String| {
        level = level.max(l);
        findings.push(msg);
    };

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    let in_system = is_system_location(path);

    if blocklist.contains(&sha256.to_lowercase()) {
        flag(Level::Danger, "hash is on the blocklist".into());
    }

    match sig.status {
        SigStatus::Invalid => {
            let detail = sig.detail.clone().unwrap_or_default();
            let lvl = if detail.contains("modified") {
                Level::Danger
            } else {
                Level::Warn
            };
            flag(lvl, format!("signature check failed: {detail}"));
        }
        SigStatus::Unsigned if in_system => flag(
            Level::Warn,
            "unsigned binary inside a Windows system directory".into(),
        ),
        SigStatus::Unsigned if is_user_writable(path) => flag(
            Level::Warn,
            "unsigned binary in a user-writable location".into(),
        ),
        SigStatus::Unsigned => flag(Level::Info, "unsigned".into()),
        SigStatus::Error => flag(
            Level::Info,
            format!(
                "signature could not be checked: {}",
                sig.detail.clone().unwrap_or_default()
            ),
        ),
        SigStatus::Valid => {
            if in_system && !is_microsoft(&sig.signer) && !is_trusted_vendor(&sig.signer) {
                flag(
                    Level::Info,
                    format!(
                        "signed by a non-Microsoft publisher ({}) in a system directory",
                        sig.signer.clone().unwrap_or_else(|| "unknown".into())
                    ),
                );
            }
        }
    }

    if CORE_PROCESSES.contains(&name.as_str()) {
        if !in_system {
            flag(
                Level::Danger,
                format!("'{name}' is a core Windows process name but lives outside the system directories"),
            );
        } else if sig.status == SigStatus::Valid && !is_microsoft(&sig.signer) {
            flag(
                Level::Danger,
                format!("'{name}' in a system directory is not signed by Microsoft"),
            );
        }
    }

    if has_decoy_double_extension(path) {
        flag(
            Level::Warn,
            "double extension imitating a document or media file".into(),
        );
    }

    (level, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(status: SigStatus, signer: Option<&str>, detail: Option<&str>) -> SigInfo {
        SigInfo {
            status,
            signer: signer.map(String::from),
            via_catalog: false,
            detail: detail.map(String::from),
        }
    }

    fn eval(path: &str, s: SigInfo) -> (Level, Vec<String>) {
        evaluate(Path::new(path), "abc", &s, &HashSet::new())
    }

    #[test]
    fn signed_microsoft_system_file_is_ok() {
        let (l, f) = eval(
            "C:\\Windows\\System32\\kernel32.dll",
            sig(SigStatus::Valid, Some("Microsoft Windows"), None),
        );
        assert_eq!(l, Level::Ok);
        assert!(f.is_empty());
    }

    #[test]
    fn trusted_vendor_driver_in_system32_is_ok() {
        let (l, f) = eval(
            "C:\\Windows\\System32\\nvapi64.dll",
            sig(SigStatus::Valid, Some("NVIDIA Corporation"), None),
        );
        assert_eq!(l, Level::Ok);
        assert!(f.is_empty());
    }

    #[test]
    fn unknown_vendor_in_system32_is_info() {
        let (l, _) = eval(
            "C:\\Windows\\System32\\thing.dll",
            sig(SigStatus::Valid, Some("Some Small Vendor Ltd"), None),
        );
        assert_eq!(l, Level::Info);
    }

    #[test]
    fn trusted_vendor_cannot_sign_svchost() {
        let (l, _) = eval(
            "C:\\Windows\\System32\\svchost.exe",
            sig(SigStatus::Valid, Some("NVIDIA Corporation"), None),
        );
        assert_eq!(l, Level::Danger);
    }

    #[test]
    fn svchost_outside_system_is_danger() {
        let (l, _) = eval(
            "C:\\Users\\bob\\AppData\\Local\\Temp\\svchost.exe",
            sig(SigStatus::Unsigned, None, None),
        );
        assert_eq!(l, Level::Danger);
    }

    #[test]
    fn svchost_with_foreign_signer_in_system32_is_danger() {
        let (l, _) = eval(
            "C:\\Windows\\System32\\svchost.exe",
            sig(SigStatus::Valid, Some("Evil Corp"), None),
        );
        assert_eq!(l, Level::Danger);
    }

    #[test]
    fn modified_after_signing_is_danger() {
        let (l, _) = eval(
            "C:\\Windows\\System32\\notepad.exe",
            sig(
                SigStatus::Invalid,
                Some("Microsoft Windows"),
                Some("file digest does not match the signature (modified after signing)"),
            ),
        );
        assert_eq!(l, Level::Danger);
    }

    #[test]
    fn unsigned_in_system32_is_warn() {
        let (l, _) = eval(
            "C:\\Windows\\System32\\helper.dll",
            sig(SigStatus::Unsigned, None, None),
        );
        assert_eq!(l, Level::Warn);
    }

    #[test]
    fn unsigned_elsewhere_is_info() {
        let (l, _) = eval(
            "D:\\tools\\mytool.exe",
            sig(SigStatus::Unsigned, None, None),
        );
        assert_eq!(l, Level::Info);
    }

    #[test]
    fn double_extension_warns() {
        let (l, _) = eval(
            "D:\\tools\\invoice.pdf.exe",
            sig(SigStatus::Valid, Some("Acme"), None),
        );
        assert_eq!(l, Level::Warn);
    }

    #[test]
    fn pdf_named_dll_is_not_a_decoy() {
        let (l, _) = eval(
            "C:\\Windows\\System32\\Windows.Data.Pdf.dll",
            sig(SigStatus::Valid, Some("Microsoft Windows"), None),
        );
        assert_eq!(l, Level::Ok);
    }

    #[test]
    fn blocklisted_hash_is_danger() {
        let mut bl = HashSet::new();
        bl.insert("abc".to_string());
        let (l, _) = evaluate(
            Path::new("D:\\x.exe"),
            "ABC",
            &sig(SigStatus::Valid, Some("Acme"), None),
            &bl,
        );
        assert_eq!(l, Level::Danger);
    }

    #[test]
    fn system_location_detection() {
        assert!(is_system_location(Path::new(
            "C:\\Windows\\System32\\a.dll"
        )));
        assert!(is_system_location(Path::new("C:\\Windows\\explorer.exe")));
        assert!(!is_system_location(Path::new(
            "C:\\Users\\a\\Windows\\a.dll"
        )));
        assert!(!is_system_location(Path::new("C:\\Temp\\System32\\a.dll")));
    }
}
