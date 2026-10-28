use super::SourceInfo;
use ::windows::{
    Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::*,
        System::{
            Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize},
            WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        },
        UI::{
            Shell::{SHOpenFolderAndSelectItems, SHParseDisplayName, ShellExecuteW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
    core::{HSTRING, PCWSTR},
};
use anyhow::{Context, Result, bail};
use std::{
    fs::OpenOptions,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};

mod media;

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
fn terminated(value: &[u16]) -> &[u16] {
    &value[..value.iter().position(|c| *c == 0).unwrap_or(value.len())]
}

pub fn file_identity(path: &Path) -> Result<(String, u64)> {
    // Attribute access also works on read-only volumes and directories. Never follow reparse points.
    let file = OpenOptions::new()
        .access_mode(0)
        .share_mode(7)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe {
        GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info)?;
    }
    let index = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    let birth = ((info.ftCreationTime.dwHighDateTime as u64) << 32)
        | info.ftCreationTime.dwLowDateTime as u64;
    Ok((format!("{index}:{birth}"), info.dwVolumeSerialNumber as u64))
}

pub fn source_info(path: &Path) -> Result<SourceInfo> {
    let path_wide = wide(path);
    let mut mount = vec![0u16; 32768];
    let mut volume = vec![0u16; 128];
    let mut filesystem = vec![0u16; 256];
    let mut flags = 0;
    unsafe {
        GetVolumePathNameW(PCWSTR(path_wide.as_ptr()), &mut mount)?;
        // Fixed USB disks and removable cards both qualify; network drives do not.
        let kind = GetDriveTypeW(PCWSTR(mount.as_ptr()));
        if ![2, 3, 5, 6].contains(&kind) {
            bail!("Choose a mounted local drive or folder");
        }
        GetVolumeNameForVolumeMountPointW(PCWSTR(mount.as_ptr()), &mut volume)
            .context("This drive does not expose a stable volume identity")?;
        GetVolumeInformationW(
            PCWSTR(mount.as_ptr()),
            None,
            None,
            None,
            Some(&mut flags),
            Some(&mut filesystem),
        )?;
        Ok(SourceInfo {
            volume_id: String::from_utf16(terminated(&volume))?,
            mount: PathBuf::from(std::ffi::OsString::from_wide(terminated(&mount)))
                .canonicalize()?,
            filesystem: String::from_utf16_lossy(terminated(&filesystem)),
            drive_kind: if kind == 2 {
                "removable"
            } else if kind == 5 {
                "optical"
            } else {
                "fixed"
            }
            .into(),
            read_only: Some(flags & 0x0008_0000 != 0), // FILE_READ_ONLY_VOLUME
            root_key: String::new(),
        })
    }
}

pub fn mount_roots(volume: &str) -> Vec<PathBuf> {
    let volume = HSTRING::from(volume);
    let mut paths = vec![0u16; 32768];
    let mut required = 0;
    if unsafe { GetVolumePathNamesForVolumeNameW(&volume, Some(&mut paths), &mut required) }
        .is_err()
    {
        return Vec::new();
    }
    paths
        .split(|c| *c == 0)
        .take_while(|p| !p.is_empty())
        .map(|p| PathBuf::from(std::ffi::OsString::from_wide(p)))
        .collect()
}

pub fn open_original(path: &Path, reveal: bool) -> Result<()> {
    let path = HSTRING::from(media::storage_path(path).as_path());
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let result = (|| -> Result<()> {
            if reveal {
                let mut item = std::ptr::null_mut();
                SHParseDisplayName(&path, None, &mut item, 0, None)?;
                let result = SHOpenFolderAndSelectItems(item, None, 0);
                CoTaskMemFree(Some(item.cast()));
                result?;
                Ok(())
            } else {
                let result = ShellExecuteW(
                    None,
                    &HSTRING::from("open"),
                    &path,
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                );
                if result.0 as isize <= 32 {
                    bail!("Could not open the original in its default app");
                }
                Ok(())
            }
        })();
        CoUninitialize();
        result
    }
}

pub fn media_worker_main() {
    use std::io::{Read, Write};
    let result = (|| -> Result<crate::model::Metadata> {
        let mut bytes = Vec::new();
        std::io::stdin().take(1_048_577).read_to_end(&mut bytes)?;
        if bytes.len() > 1_048_576 {
            bail!("Request too large");
        }
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        let result = media::probe(&bytes);
        unsafe {
            RoUninitialize();
        }
        result
    })()
    .unwrap_or_else(|_| crate::model::Metadata {
        metadata_state: "failed".into(),
        thumbnail_state: "failed".into(),
        error: Some("unsupported_or_corrupt_media".into()),
        ..Default::default()
    });
    if let Ok(bytes) = serde_json::to_vec(&result) {
        let _ = std::io::stdout().write_all(&bytes);
    }
}
