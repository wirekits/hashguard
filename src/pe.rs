//! Detect Portable Executable files (`.exe`, `.dll`, `.sys` ...).

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const PE_EXTENSIONS: &[&str] = &["exe", "dll", "sys", "ocx", "scr", "cpl", "drv", "efi"];

pub fn has_pe_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| PE_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

/// Check the DOS `MZ` header and the `PE\0\0` signature it points to.
pub fn looks_like_pe(buf: &[u8]) -> bool {
    if buf.len() < 0x40 || &buf[..2] != b"MZ" {
        return false;
    }
    let e_lfanew = u32::from_le_bytes([buf[0x3C], buf[0x3D], buf[0x3E], buf[0x3F]]) as usize;
    buf.get(e_lfanew..e_lfanew + 4) == Some(b"PE\0\0".as_slice())
}

/// True when the file has a PE extension and a valid PE header.
pub fn is_pe_file(path: &Path) -> bool {
    if !has_pe_extension(path) {
        return false;
    }
    is_pe_content(path)
}

/// Header check only, regardless of extension (a renamed executable still counts).
pub fn is_pe_content(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let mut buf = Vec::with_capacity(4096);
    if file.take(4096).read_to_end(&mut buf).is_err() {
        return false;
    }
    looks_like_pe(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_pe() -> Vec<u8> {
        let mut b = vec![0u8; 0x100];
        b[0] = b'M';
        b[1] = b'Z';
        b[0x3C] = 0x80;
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b
    }

    #[test]
    fn accepts_valid_header() {
        assert!(looks_like_pe(&fake_pe()));
    }

    #[test]
    fn rejects_missing_pe_signature() {
        let mut b = fake_pe();
        b[0x80] = b'X';
        assert!(!looks_like_pe(&b));
    }

    #[test]
    fn rejects_text_and_short_files() {
        assert!(!looks_like_pe(b"hello world"));
        assert!(!looks_like_pe(b"MZ"));
    }

    #[test]
    fn rejects_out_of_range_offset() {
        let mut b = fake_pe();
        b[0x3C] = 0xFF;
        b[0x3D] = 0xFF;
        assert!(!looks_like_pe(&b));
    }

    #[test]
    fn extension_match_is_case_insensitive() {
        assert!(has_pe_extension(Path::new("C:\\a\\KERNEL32.DLL")));
        assert!(!has_pe_extension(Path::new("notes.txt")));
    }
}
