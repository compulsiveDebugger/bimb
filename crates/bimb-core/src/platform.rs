use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

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
            .chunks_exact(2)
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
pub fn file_key(m: &std::fs::Metadata) -> String {
    #[cfg(unix)]
    {
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
    #[cfg(not(unix))]
    {
        let _ = m;
        String::new()
    }
}
pub fn device(m: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        m.dev()
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        0
    }
}
pub fn revision(m: &std::fs::Metadata) -> String {
    format!(
        "{}:{}:{}",
        file_key(m),
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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = m;
        false
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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = home;
        bail!("This preview currently supports macOS only")
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
    let mut child = Command::new(executable)
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
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("Choose a real folder, not a symbolic link");
    }
    let result = worker(executable, "source", path, None, 512)?;
    let volume = result["volume_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("A local folder with a stable volume identity is required")?;
    Ok((volume.to_owned(), file_key(&metadata)))
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
        if m.file_type().is_symlink() || is_placeholder(&m) {
            bail!("Linked or undownloaded files are not opened");
        }
    }
    let resolved = current.canonicalize()?;
    if !resolved.starts_with(root.canonicalize()?) {
        bail!("File left its registered source");
    }
    Ok(resolved)
}
