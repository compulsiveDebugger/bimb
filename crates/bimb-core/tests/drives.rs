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
