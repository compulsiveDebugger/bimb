#![cfg(any(target_os = "macos", windows))]
use bimb_core::{
    Library,
    model::{Annotation, Query},
    platform,
};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

fn library(home: &Path) -> Library {
    #[cfg(windows)]
    {
        Library::open_with_media_worker(home, Path::new(env!("CARGO_BIN_EXE_bimb-media"))).unwrap()
    }
    #[cfg(not(windows))]
    {
        Library::open(home).unwrap()
    }
}
fn wait(lib: &Library, id: &str) {
    let start = Instant::now();
    loop {
        let source = lib.overview().unwrap().sources.remove(0);
        if lib.require_idle(id).is_ok() {
            assert_eq!(source.scan_state, "complete");
            return;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "Scan timed out");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn native_drive_identity_and_media_survive_rescan_rename_and_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("旅行 photos");
    fs::create_dir(&root).unwrap();
    let original = root.join("phone é.jpg");
    let bytes = include_bytes!("fixtures/mountain.jpg");
    fs::write(&original, bytes).unwrap();
    fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/broken.jpg"),
        root.join("broken.jpg"),
    )
    .unwrap();
    let lib = library(&tmp.path().join("catalog"));
    let id = lib.add_source(root.clone()).unwrap();
    wait(&lib, &id);
    let info = &lib.overview().unwrap().sources[0];
    assert!(!info.filesystem.is_empty());
    assert!(info.read_only.is_some());
    assert!(!lib.catalog().source(&id).unwrap().root_key.is_empty());
    assert!(lib.catalog().source(&id).unwrap().relative_root.is_some());
    let page = lib.query(Query::default()).unwrap();
    let asset = page
        .assets
        .iter()
        .find(|a| a.filename == "phone é.jpg")
        .unwrap();
    assert_eq!(asset.metadata_state, "ready", "{:?}", asset.error);
    assert_eq!(asset.thumbnail_state, "ready", "{:?}", asset.error);
    assert!(asset.width.is_some_and(|w| w > 0));
    assert!(lib.preview(&asset.id).unwrap().starts_with(&[0xff, 0xd8]));
    assert_eq!(
        page.assets
            .iter()
            .find(|a| a.filename == "broken.jpg")
            .unwrap()
            .metadata_state,
        "failed"
    );
    lib.annotate(
        asset.id.clone(),
        Annotation {
            favorite: true,
            caption: "Keep me".into(),
            ..Default::default()
        },
    )
    .unwrap();
    fs::rename(&original, root.join("renamed.jpg")).unwrap();
    lib.start_scan(&id).unwrap();
    wait(&lib, &id);
    let renamed = lib.asset(&asset.id).unwrap();
    assert_eq!(renamed.filename, "renamed.jpg");
    assert!(renamed.favorite);
    assert_eq!(fs::read(lib.original(&asset.id).unwrap()).unwrap(), bytes);
    fs::rename(&root, tmp.path().join("disconnected")).unwrap();
    lib.refresh_sources().unwrap();
    assert_eq!(lib.asset(&asset.id).unwrap().availability, "offline");
    assert!(!lib.thumbnail(&asset.id).unwrap().is_empty());
    assert!(lib.original(&asset.id).is_err());
    fs::rename(tmp.path().join("disconnected"), &root).unwrap();
    lib.refresh_sources().unwrap();
    wait(&lib, &id);
    assert_eq!(lib.asset(&asset.id).unwrap().caption, "Keep me");
    assert_eq!(lib.overview().unwrap().sources[0].state, "online");
}

#[test]
fn native_paths_round_trip_without_unicode_loss() {
    #[cfg(windows)]
    let path = {
        use std::os::windows::ffi::OsStringExt;
        std::path::PathBuf::from(std::ffi::OsString::from_wide(&[
            b'C' as u16,
            58,
            92,
            0xd800,
            0x65c5,
            0x884c,
        ]))
    };
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStringExt;
        std::path::PathBuf::from(std::ffi::OsString::from_vec(
            b"/photos/\xff\xfe.jpg".to_vec(),
        ))
    };
    assert_eq!(platform::from_bytes(platform::path_bytes(&path)), path);
}

/// Run against a synthetic folder on a mounted FAT/exFAT/NTFS image. No source writes.
#[test]
#[ignore = "requires BIMB_TEST_DRIVE_FIXTURE pointing at a synthetic mounted image"]
fn mounted_drive_fixture_is_read_without_writing_originals() {
    let root = std::path::PathBuf::from(
        std::env::var_os("BIMB_TEST_DRIVE_FIXTURE").expect("Set BIMB_TEST_DRIVE_FIXTURE"),
    );
    let before = fs::read(root.join("mountain.jpg")).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let lib = library(&tmp.path().join("catalog"));
    let id = lib.add_source(root.clone()).unwrap();
    wait(&lib, &id);
    let source = lib.overview().unwrap().sources.remove(0);
    if let Ok(expected) = std::env::var("BIMB_TEST_READ_ONLY") {
        assert_eq!(source.read_only, Some(expected.parse().unwrap()));
    }
    println!(
        "{}: {} ({:?}, read_only={:?})",
        source.path, source.filesystem, source.drive_kind, source.read_only
    );
    let asset = lib
        .query(Query::default())
        .unwrap()
        .assets
        .into_iter()
        .find(|a| a.filename == "mountain.jpg")
        .unwrap();
    assert_eq!(asset.thumbnail_state, "ready", "{:?}", asset.error);
    assert_eq!(fs::read(lib.original(&asset.id).unwrap()).unwrap(), before);
    lib.start_scan(&id).unwrap();
    wait(&lib, &id);
    assert_eq!(lib.query(Query::default()).unwrap().total, 1);
    assert_eq!(fs::read(root.join("mountain.jpg")).unwrap(), before);
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires macOS DiskImages access for disposable FAT32 and exFAT images"]
fn forced_image_detach_interrupts_and_reconnects_scan() {
    use std::{path::PathBuf, process::Command};

    struct Mount(Option<PathBuf>);
    impl Mount {
        fn attach(image: &Path, read_only: bool) -> Self {
            let mut command = Command::new("hdiutil");
            command.arg("attach").arg(image).args([
                "-nobrowse",
                "-noautoopen",
                "-mountrandom",
                "/Volumes",
            ]);
            if read_only {
                command.arg("-readonly");
            }
            let output = command.output().unwrap();
            assert!(output.status.success(), "{:?}", output);
            let stdout = String::from_utf8(output.stdout).unwrap();
            let mount = stdout
                .lines()
                .filter_map(|line| line.split('\t').next_back())
                .map(str::trim)
                .find(|path| path.starts_with("/Volumes/"))
                .unwrap_or_else(|| panic!("No mount point in hdiutil output: {stdout}"));
            Self(Some(mount.into()))
        }

        fn detach(&mut self) -> bool {
            let Some(path) = &self.0 else { return true };
            let detached = Command::new("hdiutil")
                .arg("detach")
                .arg(path)
                .arg("-force")
                .status()
                .is_ok_and(|status| status.success());
            if detached {
                self.0 = None;
            }
            detached
        }
    }
    impl Drop for Mount {
        fn drop(&mut self) {
            self.detach();
        }
    }

    fn until(description: &str, mut condition: impl FnMut() -> bool) {
        let start = Instant::now();
        while !condition() {
            assert!(start.elapsed() < Duration::from_secs(60), "{description}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    for filesystem in ["MS-DOS FAT32", "ExFAT"] {
        let tmp = tempfile::tempdir().unwrap();
        let image = tmp.path().join("bimb-reconnect.dmg");
        let status = Command::new("hdiutil")
            .args([
                "create",
                "-size",
                "96m",
                "-fs",
                filesystem,
                "-volname",
                "BIMB-TEST",
            ])
            .arg(&image)
            .status()
            .unwrap();
        assert!(status.success());
        let mut writable = Mount::attach(&image, false);
        let root = writable.0.as_ref().unwrap().join("media");
        fs::create_dir(&root).unwrap();
        let bytes = include_bytes!("fixtures/mountain.jpg");
        fs::write(root.join("mountain.jpg"), bytes).unwrap();
        let lib = library(&tmp.path().join("catalog"));
        let source = lib.add_source(root.clone()).unwrap();
        wait(&lib, &source);
        let asset = lib.query(Query::default()).unwrap().assets.remove(0);
        let thumbnail = lib.thumbnail(&asset.id).unwrap();
        lib.annotate(
            asset.id.clone(),
            Annotation {
                favorite: true,
                caption: "Keep me".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let batch = root.join("batch");
        fs::create_dir(&batch).unwrap();
        for number in 0..120 {
            fs::write(batch.join(format!("new-{number:03}.jpg")), bytes).unwrap();
        }
        lib.start_scan(&source).unwrap();
        until("rescan did not discover new files", || {
            lib.require_idle(&source).is_err() && lib.overview().unwrap().total > 1
        });
        assert!(
            writable.detach(),
            "Could not detach the {filesystem} test image"
        );
        until("scan remained active after detach", || {
            lib.require_idle(&source).is_ok()
        });
        lib.refresh_sources().unwrap();
        assert_eq!(lib.overview().unwrap().sources[0].scan_state, "interrupted");
        assert_eq!(lib.asset(&asset.id).unwrap().availability, "offline");
        assert_eq!(lib.thumbnail(&asset.id).unwrap(), thumbnail);
        assert!(lib.original(&asset.id).is_err());

        let read_only = Mount::attach(&image, true);
        let new_root = read_only.0.as_ref().unwrap().join("media");
        assert_ne!(new_root, root);
        lib.refresh_sources().unwrap();
        wait(&lib, &source);
        let overview = lib.overview().unwrap();
        assert_eq!(overview.sources[0].state, "online");
        assert_eq!(overview.sources[0].read_only, Some(true));
        assert_eq!(lib.catalog().source(&source).unwrap().root, new_root);
        assert_eq!(overview.total, 121);
        let restored = lib.asset(&asset.id).unwrap();
        assert!(restored.favorite);
        assert_eq!(restored.caption, "Keep me");
        assert_eq!(lib.thumbnail(&asset.id).unwrap(), thumbnail);
        assert_eq!(fs::read(lib.original(&asset.id).unwrap()).unwrap(), bytes);
    }
}
