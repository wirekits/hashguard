//! Authenticode signature verification.
//!
//! On Windows this calls `WinVerifyTrust`. Most system binaries are not signed
//! individually; their hashes live in signed catalog (`.cat`) files, so when the
//! embedded check finds no signature we look the file up in the catalog database.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SigStatus {
    /// Signature chains to a trusted root and the file digest matches.
    Valid,
    /// No embedded signature and no catalog entry.
    Unsigned,
    /// A signature exists but fails verification (tampered, untrusted, expired...).
    Invalid,
    /// Verification could not be performed.
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigInfo {
    pub status: SigStatus,
    /// Display name of the signing certificate, when available.
    pub signer: Option<String>,
    /// True when trust came from a Windows catalog rather than an embedded signature.
    pub via_catalog: bool,
    /// Human readable explanation for non-valid results.
    pub detail: Option<String>,
}

impl SigInfo {
    fn error(msg: impl Into<String>) -> Self {
        SigInfo {
            status: SigStatus::Error,
            signer: None,
            via_catalog: false,
            detail: Some(msg.into()),
        }
    }
}

/// Verify the Authenticode signature of `path`.
pub fn verify(path: &Path) -> SigInfo {
    imp::verify(path)
}

/// Translate a `WinVerifyTrust` result code into a status plus explanation.
pub fn classify(code: i32) -> (SigStatus, Option<String>) {
    match code as u32 {
        0 => (SigStatus::Valid, None),
        0x800B0100 => (SigStatus::Unsigned, Some("no signature present".into())),
        0x80096010 => (
            SigStatus::Invalid,
            Some("file digest does not match the signature (modified after signing)".into()),
        ),
        0x800B0101 => (SigStatus::Invalid, Some("signing certificate expired".into())),
        0x800B0109 => (
            SigStatus::Invalid,
            Some("certificate chain ends in an untrusted root".into()),
        ),
        0x800B010A => (
            SigStatus::Invalid,
            Some(
                "certificate chain could not be built (unknown or missing root; signer name is unverified)"
                    .into(),
            ),
        ),
        0x800B0111 => (
            SigStatus::Invalid,
            Some("explicitly distrusted publisher".into()),
        ),
        0x800B010C => (SigStatus::Invalid, Some("certificate revoked".into())),
        0x800B0004 => (
            SigStatus::Invalid,
            Some("subject is not trusted for the requested action".into()),
        ),
        other => (
            SigStatus::Invalid,
            Some(format!("trust verification failed (0x{other:08X})")),
        ),
    }
}

/// Result codes that mean "no embedded signature, try the catalog instead".
pub fn should_try_catalog(code: i32) -> bool {
    matches!(
        code as u32,
        0x800B0100 // TRUST_E_NOSIGNATURE
        | 0x800B0003 // TRUST_E_SUBJECT_FORM_UNKNOWN
        | 0x800B0001 // TRUST_E_PROVIDER_UNKNOWN
    )
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::c_void;
    use std::fs::File;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::ptr::{null, null_mut};
    use windows_sys::core::GUID;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Security::Cryptography::Catalog::*;
    use windows_sys::Win32::Security::Cryptography::*;
    use windows_sys::Win32::Security::WinTrust::*;

    /// WINTRUST_ACTION_GENERIC_VERIFY_V2
    const ACTION_GENERIC_VERIFY_V2: GUID = GUID {
        data1: 0x00AAC56B,
        data2: 0xCD44,
        data3: 0x11d0,
        data4: [0x8c, 0xc2, 0x00, 0xc0, 0x4f, 0xc2, 0x95, 0xee],
    };

    // Not exposed by windows-sys; declared by hand (both live in wintrust.dll).
    #[link(name = "wintrust")]
    extern "system" {
        fn WTHelperProvDataFromStateData(state: *mut c_void) -> *mut c_void;
        fn WTHelperGetProvSignerFromChain(
            prov: *mut c_void,
            idx_signer: u32,
            counter_signer: i32,
            idx_counter_signer: u32,
        ) -> *mut ProvSigner;
    }

    // Leading fields of CRYPT_PROVIDER_SGNR / CRYPT_PROVIDER_CERT (wintrust.h);
    // only what is needed to reach the signer's certificate context.
    #[repr(C)]
    struct ProvSigner {
        cb_struct: u32,
        verify_as_of: [u32; 2], // FILETIME
        cs_cert_chain: u32,
        pas_cert_chain: *const ProvCert,
    }

    #[repr(C)]
    struct ProvCert {
        cb_struct: u32,
        cert: *const CERT_CONTEXT,
    }

    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn wide_str(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn verify(path: &Path) -> SigInfo {
        let wpath = wide(path.as_os_str());

        // 1. Embedded signature.
        let (code, signer) = unsafe { check_file(&wpath) };
        if code == 0 {
            return SigInfo {
                status: SigStatus::Valid,
                signer,
                via_catalog: false,
                detail: None,
            };
        }
        if !should_try_catalog(code) {
            // A file listed in a signed catalog is trusted even if its embedded chain fails.
            if let Ok(Some((0, catalog_signer))) = unsafe { check_catalog(path, &wpath) } {
                return SigInfo {
                    status: SigStatus::Valid,
                    signer: catalog_signer,
                    via_catalog: true,
                    detail: None,
                };
            }
            let (status, detail) = classify(code);
            return SigInfo {
                status,
                signer,
                via_catalog: false,
                detail,
            };
        }

        // 2. Catalog signature.
        match unsafe { check_catalog(path, &wpath) } {
            Ok(Some((code, signer))) => {
                let (status, detail) = classify(code);
                SigInfo {
                    status,
                    signer,
                    via_catalog: true,
                    detail,
                }
            }
            Ok(None) => {
                let (status, detail) = classify(code);
                SigInfo {
                    status,
                    signer: None,
                    via_catalog: false,
                    detail,
                }
            }
            Err(e) => SigInfo::error(e),
        }
    }

    unsafe fn check_file(wpath: &[u16]) -> (i32, Option<String>) {
        let mut file_info: WINTRUST_FILE_INFO = std::mem::zeroed();
        file_info.cbStruct = std::mem::size_of::<WINTRUST_FILE_INFO>() as u32;
        file_info.pcwszFilePath = wpath.as_ptr();

        let mut data: WINTRUST_DATA = std::mem::zeroed();
        data.cbStruct = std::mem::size_of::<WINTRUST_DATA>() as u32;
        data.dwUIChoice = WTD_UI_NONE;
        data.fdwRevocationChecks = WTD_REVOKE_NONE;
        data.dwUnionChoice = WTD_CHOICE_FILE;
        data.Anonymous.pFile = &mut file_info;
        data.dwProvFlags = WTD_CACHE_ONLY_URL_RETRIEVAL;
        run_trust(&mut data)
    }

    unsafe fn check_catalog(
        path: &Path,
        wpath: &[u16],
    ) -> Result<Option<(i32, Option<String>)>, String> {
        let file = File::open(path).map_err(|e| format!("cannot open file: {e}"))?;
        let handle = file.as_raw_handle();

        let mut admin: isize = 0;
        if CryptCATAdminAcquireContext(&mut admin, null(), 0) == 0 {
            return Err("CryptCATAdminAcquireContext failed".into());
        }

        let result = (|| {
            let mut hash_len: u32 = 0;
            CryptCATAdminCalcHashFromFileHandle(handle as _, &mut hash_len, null_mut(), 0);
            if hash_len == 0 {
                return Ok(None);
            }
            let mut hash = vec![0u8; hash_len as usize];
            if CryptCATAdminCalcHashFromFileHandle(handle as _, &mut hash_len, hash.as_mut_ptr(), 0)
                == 0
            {
                return Ok(None);
            }

            let cat =
                CryptCATAdminEnumCatalogFromHash(admin, hash.as_ptr(), hash_len, 0, null_mut());
            if cat == 0 {
                return Ok(None);
            }

            let mut info: CATALOG_INFO = std::mem::zeroed();
            info.cbStruct = std::mem::size_of::<CATALOG_INFO>() as u32;
            if CryptCATCatalogInfoFromContext(cat, &mut info, 0) == 0 {
                CryptCATAdminReleaseCatalogContext(admin, cat, 0);
                return Err("CryptCATCatalogInfoFromContext failed".to_string());
            }

            // The member tag is the uppercase hex of the file's catalog hash.
            let tag: String = hash.iter().map(|b| format!("{b:02X}")).collect();
            let wtag = wide_str(&tag);

            let mut cat_info: WINTRUST_CATALOG_INFO = std::mem::zeroed();
            cat_info.cbStruct = std::mem::size_of::<WINTRUST_CATALOG_INFO>() as u32;
            cat_info.pcwszCatalogFilePath = info.wszCatalogFile.as_ptr();
            cat_info.pcwszMemberTag = wtag.as_ptr();
            cat_info.pcwszMemberFilePath = wpath.as_ptr();
            cat_info.pbCalculatedFileHash = hash.as_mut_ptr();
            cat_info.cbCalculatedFileHash = hash_len;
            cat_info.hCatAdmin = admin;

            let mut data: WINTRUST_DATA = std::mem::zeroed();
            data.cbStruct = std::mem::size_of::<WINTRUST_DATA>() as u32;
            data.dwUIChoice = WTD_UI_NONE;
            data.fdwRevocationChecks = WTD_REVOKE_NONE;
            data.dwUnionChoice = WTD_CHOICE_CATALOG;
            data.Anonymous.pCatalog = &mut cat_info;
            data.dwProvFlags = WTD_CACHE_ONLY_URL_RETRIEVAL;
            let out = run_trust(&mut data);

            CryptCATAdminReleaseCatalogContext(admin, cat, 0);
            Ok(Some(out))
        })();

        CryptCATAdminReleaseContext(admin, 0);
        result
    }

    /// Run the verify/close cycle, extracting the signer name while state is open.
    unsafe fn run_trust(data: &mut WINTRUST_DATA) -> (i32, Option<String>) {
        let mut action = ACTION_GENERIC_VERIFY_V2;
        let hwnd = INVALID_HANDLE_VALUE as _;

        data.dwStateAction = WTD_STATEACTION_VERIFY;
        let code = WinVerifyTrust(hwnd, &mut action, data as *mut _ as *mut c_void);
        let signer = signer_name(data.hWVTStateData);

        data.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(hwnd, &mut action, data as *mut _ as *mut c_void);
        (code, signer)
    }

    unsafe fn signer_name(state: *mut c_void) -> Option<String> {
        if state.is_null() {
            return None;
        }
        let prov = WTHelperProvDataFromStateData(state);
        if prov.is_null() {
            return None;
        }
        let sgnr = WTHelperGetProvSignerFromChain(prov, 0, 0, 0);
        if sgnr.is_null() {
            return None;
        }
        let chain = (*sgnr).pas_cert_chain;
        if chain.is_null() || (*sgnr).cs_cert_chain == 0 {
            return None;
        }
        let cert = (*chain).cert;
        if cert.is_null() {
            return None;
        }
        let mut buf = [0u16; 256];
        let n = CertGetNameStringW(
            cert,
            CERT_NAME_SIMPLE_DISPLAY_TYPE,
            0,
            null(),
            buf.as_mut_ptr(),
            buf.len() as u32,
        );
        if n <= 1 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..(n as usize - 1)]))
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn verify(_path: &Path) -> SigInfo {
        SigInfo::error("signature verification is only supported on Windows")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_success() {
        assert_eq!(classify(0).0, SigStatus::Valid);
    }

    #[test]
    fn classify_unsigned() {
        assert_eq!(classify(0x800B0100u32 as i32).0, SigStatus::Unsigned);
    }

    #[test]
    fn classify_tampered() {
        let (status, detail) = classify(0x80096010u32 as i32);
        assert_eq!(status, SigStatus::Invalid);
        assert!(detail.unwrap().contains("modified"));
    }

    #[test]
    fn classify_broken_chain_is_explained() {
        let (status, detail) = classify(0x800B010Au32 as i32);
        assert_eq!(status, SigStatus::Invalid);
        assert!(detail.unwrap().contains("chain"));
    }

    #[test]
    fn catalog_fallback_only_for_missing_signature() {
        assert!(should_try_catalog(0x800B0100u32 as i32));
        assert!(!should_try_catalog(0x80096010u32 as i32));
        assert!(!should_try_catalog(0));
    }
}
