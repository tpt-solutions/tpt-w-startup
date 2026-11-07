// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Authenticode signature check via `WinVerifyTrust` (Phase 2).
//!
//! Two forms are checked, in order:
//! 1. An embedded signature in the file itself (most third-party software).
//! 2. A catalog signature (most Windows system binaries are signed this way,
//!    so an embedded-only check would call them unsigned).
//!
//! Revocation checking is off. It needs network access and can stall a scan.
//! The result describes the signature's trust, not whether the publisher is
//! one we recognise; that is the curated database's job.

use std::ffi::c_void;
use std::path::Path;
use std::ptr::{null, null_mut};
use tpt_startup_core::SignatureStatus;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows_sys::Win32::Security::Cryptography::Catalog::{
    CryptCATAdminAcquireContext, CryptCATAdminCalcHashFromFileHandle,
    CryptCATAdminEnumCatalogFromHash, CryptCATAdminReleaseCatalogContext,
    CryptCATAdminReleaseContext, CryptCATCatalogInfoFromContext, CATALOG_INFO,
};
use windows_sys::Win32::Security::WinTrust::{
    WinVerifyTrust, WINTRUST_CATALOG_INFO, WINTRUST_DATA, WINTRUST_FILE_INFO, WTD_CHOICE_CATALOG,
    WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, OPEN_EXISTING,
};

/// `WINTRUST_ACTION_GENERIC_VERIFY_V2`: the standard Authenticode policy.
const ACTION_GENERIC_VERIFY_V2: GUID = GUID {
    data1: 0x00AA_C56B,
    data2: 0xCD44,
    data3: 0x11D0,
    data4: [0x8C, 0xC2, 0x00, 0xC0, 0x4F, 0xC2, 0x95, 0xEE],
};

/// `TRUST_E_NOSIGNATURE`: the file has no signature of this form.
const TRUST_E_NOSIGNATURE: i32 = 0x800B_0100u32 as i32;
const GENERIC_READ: u32 = 0x8000_0000;

/// Check the Authenticode signature of `path`.
///
/// `Unknown` when the file is missing (a stale entry is not a bad signature),
/// `Valid` when a signature verifies, `Unsigned` when neither form is present,
/// and `Invalid` when a signature is present but does not verify.
pub fn check(path: &Path) -> SignatureStatus {
    if !path.is_file() {
        return SignatureStatus::Unknown;
    }
    let wide = to_wide(path.as_os_str());

    let embedded = trust_embedded(&wide);
    if embedded == 0 {
        return SignatureStatus::Valid;
    }
    if embedded != TRUST_E_NOSIGNATURE {
        return SignatureStatus::Invalid;
    }
    match trust_catalog(&wide) {
        None => SignatureStatus::Unsigned,
        Some(0) => SignatureStatus::Valid,
        Some(_) => SignatureStatus::Invalid,
    }
}

fn to_wide(s: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().chain(std::iter::once(0)).collect()
}

/// Run WinVerifyTrust on `data`, then always close the state it allocated.
///
/// # Safety
/// `data` must be fully initialised and every pointer it holds must stay
/// valid for the duration of the call.
unsafe fn run_trust(data: &mut WINTRUST_DATA) -> i32 {
    let mut action = ACTION_GENERIC_VERIFY_V2;
    let hwnd: HWND = null_mut();
    let data_ptr = data as *mut WINTRUST_DATA as *mut c_void;
    data.dwStateAction = WTD_STATEACTION_VERIFY;
    let status = WinVerifyTrust(hwnd, &mut action, data_ptr);
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    WinVerifyTrust(hwnd, &mut action, data_ptr);
    status
}

fn base_data() -> WINTRUST_DATA {
    // SAFETY: WINTRUST_DATA is a plain C struct; all-zero is a valid starting
    // point, and the fields we use are set below.
    let mut data: WINTRUST_DATA = unsafe { std::mem::zeroed() };
    data.cbStruct = std::mem::size_of::<WINTRUST_DATA>() as u32;
    data.dwUIChoice = WTD_UI_NONE;
    data.fdwRevocationChecks = WTD_REVOKE_NONE;
    data
}

/// Embedded-signature check. Returns the raw WinVerifyTrust status.
fn trust_embedded(wide: &[u16]) -> i32 {
    // SAFETY: `wide` and the local file info outlive the call below.
    unsafe {
        let mut file_info: WINTRUST_FILE_INFO = std::mem::zeroed();
        file_info.cbStruct = std::mem::size_of::<WINTRUST_FILE_INFO>() as u32;
        file_info.pcwszFilePath = wide.as_ptr();
        let mut data = base_data();
        data.dwUnionChoice = WTD_CHOICE_FILE;
        data.Anonymous.pFile = &mut file_info;
        run_trust(&mut data)
    }
}

/// Catalog-signature check. `None` if no catalog covers the file.
fn trust_catalog(wide: &[u16]) -> Option<i32> {
    // SAFETY: each handle is released on every path below.
    unsafe {
        let mut admin: isize = 0;
        if CryptCATAdminAcquireContext(&mut admin, null(), 0) == 0 {
            return None;
        }
        let result = trust_catalog_with(admin, wide);
        CryptCATAdminReleaseContext(admin, 0);
        result
    }
}

/// # Safety
/// `admin` must be a valid catalog-admin handle owned by the caller.
unsafe fn trust_catalog_with(admin: isize, wide: &[u16]) -> Option<i32> {
    let file: HANDLE = CreateFileW(
        wide.as_ptr(),
        GENERIC_READ,
        FILE_SHARE_READ,
        null(),
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        null_mut(),
    );
    // INVALID_HANDLE_VALUE is -1 cast to a handle.
    if file as isize == -1 {
        return None;
    }
    let result = trust_catalog_file(admin, file, wide);
    CloseHandle(file);
    result
}

/// # Safety
/// `admin` and `file` must be valid handles for the duration of the call.
unsafe fn trust_catalog_file(admin: isize, file: HANDLE, wide: &[u16]) -> Option<i32> {
    // First call with a null buffer to learn the hash size.
    let mut size: u32 = 0;
    CryptCATAdminCalcHashFromFileHandle(file, &mut size, null_mut(), 0);
    if size == 0 {
        return None;
    }
    let mut hash = vec![0u8; size as usize];
    if CryptCATAdminCalcHashFromFileHandle(file, &mut size, hash.as_mut_ptr(), 0) == 0 {
        return None;
    }
    hash.truncate(size as usize);

    // Catalogs index members by the uppercase hex of the file hash.
    let tag: Vec<u16> = hash
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<String>()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // Only the first covering catalog is tried. Most files belong to one.
    let mut prev: isize = 0;
    let cat = CryptCATAdminEnumCatalogFromHash(admin, hash.as_ptr(), size, 0, &mut prev);
    if cat == 0 {
        return None;
    }
    let mut info: CATALOG_INFO = std::mem::zeroed();
    info.cbStruct = std::mem::size_of::<CATALOG_INFO>() as u32;
    let have_info = CryptCATCatalogInfoFromContext(cat, &mut info, 0) != 0;
    let status = if have_info {
        let mut cat_info: WINTRUST_CATALOG_INFO = std::mem::zeroed();
        cat_info.cbStruct = std::mem::size_of::<WINTRUST_CATALOG_INFO>() as u32;
        cat_info.pcwszCatalogFilePath = info.wszCatalogFile.as_ptr();
        cat_info.pcwszMemberTag = tag.as_ptr();
        cat_info.pcwszMemberFilePath = wide.as_ptr();
        cat_info.hMemberFile = file;
        cat_info.pbCalculatedFileHash = hash.as_mut_ptr();
        cat_info.cbCalculatedFileHash = size;
        cat_info.hCatAdmin = admin;

        let mut data = base_data();
        data.dwUnionChoice = WTD_CHOICE_CATALOG;
        data.Anonymous.pCatalog = &mut cat_info;
        Some(run_trust(&mut data))
    } else {
        None
    };
    CryptCATAdminReleaseCatalogContext(admin, cat, 0);
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_system_binary_is_valid_via_catalog() {
        // Windows system binaries are catalog-signed, so this exercises the
        // fallback path rather than the embedded check.
        let notepad = Path::new(r"C:\Windows\System32\notepad.exe");
        if notepad.is_file() {
            assert_eq!(check(notepad), SignatureStatus::Valid);
        }
    }

    #[test]
    fn missing_file_is_unknown_not_invalid() {
        assert_eq!(
            check(Path::new(r"C:\definitely\not\here\x.exe")),
            SignatureStatus::Unknown
        );
    }

    #[test]
    fn unsigned_file_is_not_valid() {
        let dir = std::env::temp_dir().join(format!("tpt-sig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("plain.exe");
        std::fs::write(&file, b"not really a program").unwrap();
        let status = check(&file);
        let _ = std::fs::remove_dir_all(&dir);
        assert_ne!(status, SignatureStatus::Valid);
    }
}
