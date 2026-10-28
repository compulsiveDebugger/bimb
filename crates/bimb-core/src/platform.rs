use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[cfg(windows)]
mod windows;

#[derive(Debug, serde::Deserialize)]
pub struct SourceInfo {
    pub volume_id: String,
    pub mount: PathBuf,
    #[serde(default)]
    pub filesystem: String,
    #[serde(default)]
    pub drive_kind: String,
    pub read_only: Option<bool>,
    #[serde(skip)]
    pub root_key: String,
}

pub fn path_encoding() -> &'static str {
    if cfg!(windows) {
        "windows-utf16le"
    } else {
        "unix-bytes"
    }
}

#[cfg(unix)]
pub fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}
#[cfg(unix)]
pub fn from_bytes(bytes: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(bytes).into()
}
#[cfg(windows)]
pub fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}
#[cfg(windows)]
pub fn from_bytes(bytes: Vec<u8>) -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    std::ffi::OsString::from_wide(
        &bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| u16::from_le_bytes([v[0], v[1]]))
            .collect::<Vec<_>>(),
    )
    .into()
}

pub fn private_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
pub fn file_key(path: &Path, m: &std::fs::Metadata) -> String {
    #[cfg(unix)]
    {
        let _ = path;
        use std::os::unix::fs::MetadataExt;
        format!(
            "{}:{}",
            m.ino(),
            m.created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|t| t.as_nanos())
                .unwrap_or_default()
        )
    }
    #[cfg(windows)]
    {
        let _ = m;
        windows::file_identity(path)
            .map(|v| v.0)
            .unwrap_or_default()
    }
}
pub fn device(path: &Path, m: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        let _ = path;
        use std::os::unix::fs::MetadataExt;
        m.dev()
    }
    #[cfg(windows)]
    {
        let _ = m;
        windows::file_identity(path)
            .map(|v| v.1)
            .unwrap_or_default()
    }
}
pub fn revision(path: &Path, m: &std::fs::Metadata) -> String {
    format!(
        "{}:{}:{}",
        file_key(path, m),
        m.len(),
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|t| t.as_nanos())
            .unwrap_or_default()
    )
}
pub fn is_placeholder(m: &std::fs::Metadata) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        m.st_flags() & 0x4000_0000 != 0
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Offline/recall attributes: do not hydrate cloud files during a scan.
        m.file_attributes() & (0x1000 | 0x40000 | 0x400000) != 0
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = m;
        false
    }
}

pub fn is_link(m: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}

pub fn install_worker(home: &Path) -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::PermissionsExt;
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/bimb-media"));
        let path = home.join("bimb-media");
        // Refresh only at startup, before any worker can be running.
        let temporary = home.join("bimb-media.new");
        std::fs::write(&temporary, bytes)?;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o700))?;
        std::fs::rename(temporary, &path)?;
        Ok(path)
    }
    #[cfg(windows)]
    {
        let _ = home;
        // The desktop executable dispatches this mode before starting Tauri.
        Ok(std::env::current_exe()?)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = home;
        bail!("Bimb currently supports macOS and Windows")
    }
}

pub fn worker(
    executable: &Path,
    operation: &str,
    path: &Path,
    output: Option<&Path>,
    size: usize,
) -> Result<serde_json::Value> {
    let request = serde_json::json!({ "operation": operation, "path": STANDARD.encode(path_bytes(path)), "output": output, "maxSize": size });
    let mut command = Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command
            .arg("--bimb-media-worker")
            .creation_flags(0x08000000);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("Could not start media worker")?;
    child
        .stdin
        .take()
        .context("Worker input unavailable")?
        .write_all(&serde_json::to_vec(&request)?)?;
    let stdout = child.stdout.take().context("Worker output unavailable")?;
    // Drain concurrently with a hard output limit, avoiding pipe deadlock.
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let bytes = reader
                .join()
                .map_err(|_| anyhow::anyhow!("Worker reader failed"))??;
            if !status.success() || bytes.len() > 1_048_576 {
                bail!("media_worker_failed");
            }
            return serde_json::from_slice(&bytes).context("Invalid media response");
        }
        if start.elapsed() > Duration::from_secs(25) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            bail!("media_timeout");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn source_identity(executable: &Path, path: &Path) -> Result<(String, String)> {
    let info = source_info(executable, path)?;
    Ok((info.volume_id, info.root_key))
}

pub fn source_info(executable: &Path, path: &Path) -> Result<SourceInfo> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || is_link(&metadata) {
        bail!("Choose a real folder, not a symbolic link");
    }
    #[cfg(windows)]
    let mut info = {
        let _ = executable;
        windows::source_info(path)?
    };
    #[cfg(not(windows))]
    let mut info: SourceInfo =
        serde_json::from_value(worker(executable, "source", path, None, 512)?)
            .context("Could not identify this local drive")?;
    info.root_key = file_key(path, &metadata);
    if info.volume_id.is_empty() || info.root_key.is_empty() {
        bail!("A local folder with a stable volume identity is required");
    }
    Ok(info)
}

/// Mount locations are observations; every reconnect candidate is verified by identity.
pub fn mount_roots(volume: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        windows::mount_roots(volume)
    }
    #[cfg(not(windows))]
    {
        let _ = volume;
        let mut roots = vec![PathBuf::from("/")];
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            roots.extend(entries.flatten().take(128).map(|e| e.path()));
        }
        roots
    }
}

/// Called before the host creates a window. Media decoding stays in a bounded subprocess.
pub fn media_worker_main() -> bool {
    #[cfg(windows)]
    if std::env::args_os()
        .nth(1)
        .is_some_and(|s| s == "--bimb-media-worker")
    {
        windows::media_worker_main();
        return true;
    }
    false
}

pub fn open_original(path: &Path, reveal: bool) -> Result<()> {
    #[cfg(windows)]
    {
        windows::open_original(path, reveal)
    }
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("/usr/bin/open");
        if reveal {
            command.arg("-R");
        }
        if !command.arg(path).status()?.success() {
            bail!("Could not open the original");
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = (path, reveal);
        bail!("Opening originals requires macOS or Windows")
    }
}

pub fn checked_media(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail!("Invalid media path");
    }
    let mut current = root.to_owned();
    for part in relative.components() {
        current.push(part);
        let m = std::fs::symlink_metadata(&current)?;
        if is_link(&m) || is_placeholder(&m) {
            bail!("Linked or undownloaded files are not opened");
        }
    }
    let resolved = current.canonicalize()?;
    if !resolved.starts_with(root.canonicalize()?) {
        bail!("File left its registered source");
    }
    Ok(resolved)
}
