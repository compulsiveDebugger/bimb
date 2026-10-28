#![cfg(target_os = "macos")]
use bimb_core::{
    Library,
    model::{Annotation, Query},
};
use std::{
    fs,
    time::{Duration, Instant},
};

fn wait(lib: &Library, id: &str) {
    wait_for_state(lib, id, "complete");
}
fn wait_for_state(lib: &Library, id: &str, expected: &str) {
    let start = Instant::now();
    loop {
        let overview = lib.overview().unwrap();
        let source = overview.sources.iter().find(|s| s.id == id).unwrap();
        if !["queued", "scanning"].contains(&source.scan_state.as_str())
            && lib.require_idle(id).is_ok()
        {
            assert_eq!(source.scan_state, expected);
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(45),
            "scan timed out: {}",
            source.scan_state
        );
        std::thread::sleep(Duration::from_millis(30));
    }
}

#[test]
fn unreadable_folders_report_paths_and_recover_on_rescan() {
    use std::os::unix::fs::PermissionsExt;
    struct RestorePermissions(std::path::PathBuf);
    impl Drop for RestorePermissions {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
        }
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("photos");
    let folder = root.join("旅行/locked");
    fs::create_dir_all(&folder).unwrap();
    fs::copy(fixture(), folder.join("original.jpg")).unwrap();
    let lib = Library::open(&tmp.path().join("catalog")).unwrap();
    let id = lib.add_source(root).unwrap();
    wait(&lib, &id);
    let asset = lib.query(Query::default()).unwrap().assets.remove(0);
    lib.annotate(
        asset.id.clone(),
        Annotation {
            favorite: true,
            ..Default::default()
        },
    )
    .unwrap();
    fs::remove_file(folder.join("original.jpg")).unwrap();
    let permissions = RestorePermissions(folder.clone());
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o000)).unwrap();
    lib.start_scan(&id).unwrap();
    wait_for_state(&lib, &id, "partial");
    let page = lib.source_issues(&id, None).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.issues[0].path.as_deref(), Some("旅行/locked"));
    assert_eq!(page.issues[0].code, "folder_unreadable");
    assert!(!page.issues[0].message.is_empty());
    let source = lib.overview().unwrap().sources.remove(0);
    assert_eq!(source.scan_errors, 1);
    assert_eq!(source.media_errors, 0);
    assert_eq!(lib.asset(&asset.id).unwrap().availability, "present");
    assert!(lib.asset(&asset.id).unwrap().favorite);
    drop(permissions);
    lib.start_scan(&id).unwrap();
    wait(&lib, &id);
    assert_eq!(lib.source_issues(&id, None).unwrap().total, 0);
    assert_eq!(lib.asset(&asset.id).unwrap().availability, "missing");
    assert!(lib.asset(&asset.id).unwrap().favorite);
}

#[test]
fn scan_issue_backups_restore_current_and_version_one_catalogs() {
    use bimb_core::platform;
    let tmp = tempfile::tempdir().unwrap();
    let lib = Library::open(&tmp.path().join("catalog")).unwrap();
    let root = tmp.path().join("photos");
    fs::create_dir(&root).unwrap();
    lib.catalog().call(move |db| {
        db.execute("INSERT INTO sources(id,name,path,display_path,volume,root_key) VALUES('source','Holiday',?,?,'volume','root')", rusqlite::params![platform::path_bytes(&root),root.to_string_lossy()])?;
        db.execute("INSERT INTO issues(source_id,code,path,message) VALUES('source','folder_unreadable',?,'Permission denied')", [platform::path_bytes(std::path::Path::new("旅行/locked"))])?;
        Ok(())
    }).unwrap();
    let backup = tmp.path().join("backup.sqlite");
    lib.backup(&backup).unwrap();
    lib.restore(&backup).unwrap();
    let page = lib.source_issues("source", None).unwrap();
    assert_eq!(page.issues[0].path.as_deref(), Some("旅行/locked"));
    assert_eq!(page.issues[0].message, "Permission denied");
    let db = rusqlite::Connection::open(&backup).unwrap();
    db.execute_batch("DROP TABLE collections; DROP TABLE issues; CREATE TABLE issues(id INTEGER PRIMARY KEY,source_id TEXT REFERENCES sources(id) ON DELETE CASCADE,code TEXT NOT NULL); INSERT INTO issues(source_id,code) VALUES('source','folder_unreadable'); PRAGMA user_version=1;").unwrap();
    drop(db);
    lib.restore(&backup).unwrap();
    let page = lib.source_issues("source", None).unwrap();
    assert_eq!(page.total, 1);
    assert!(page.issues[0].path.is_none());
    assert_eq!(lib.overview().unwrap().sources[0].name, "Holiday");
    // Upgrading a backup must leave the selected backup file itself untouched.
    let db = rusqlite::Connection::open(&backup).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn collections_survive_backup_restore_and_export_and_restore_version_two() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = Library::open(&tmp.path().join("catalog")).unwrap();
    let id = lib
        .save_collection(
            String::new(),
            "旅行".into(),
            Query {
                place: "Goa".into(),
                date_from: "2024-01-01".into(),
                sort: "name".into(),
                ascending: true,
                ..Default::default()
            },
        )
        .unwrap();
    let backup = tmp.path().join("backup.sqlite");
    lib.backup(&backup).unwrap();
    lib.save_collection(id.clone(), "Changed".into(), Query::default())
        .unwrap();
    lib.restore(&backup).unwrap();
    let saved = lib.overview().unwrap().collections.remove(0);
    assert_eq!(saved.id, id);
    assert_eq!(saved.name, "旅行");
    assert_eq!(saved.query.place, "Goa");
    assert_eq!(saved.query.date_from, "2024-01-01");
    assert!(saved.query.ascending);
    let export = tmp.path().join("annotations.json");
    lib.export_annotations(&export).unwrap();
    let data: serde_json::Value = serde_json::from_slice(&fs::read(export).unwrap()).unwrap();
    assert_eq!(data["collections"][0]["name"], "旅行");
    assert_eq!(data["collections"][0]["query"]["place"], "Goa");
    assert!(data["collections"][0]["query"]["cursor"].is_null());
    // A real schema-2 backup has no collection table. Upgrade only the staged copy.
    let db = rusqlite::Connection::open(&backup).unwrap();
    db.execute_batch("DROP TABLE collections; PRAGMA user_version=2;")
        .unwrap();
    drop(db);
    lib.restore(&backup).unwrap();
    assert!(lib.overview().unwrap().collections.is_empty());
    let db = rusqlite::Connection::open(&backup).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    lib.restore(&tmp.path().join("catalog/before-restore.sqlite"))
        .unwrap();
    assert_eq!(lib.overview().unwrap().collections[0].id, id);
}
fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mountain.jpg")
}

#[test]
fn rescan_rename_disconnect_and_backup_preserve_user_data() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("photos");
    fs::create_dir(&root).unwrap();
    fs::copy(fixture(), root.join("original.jpg")).unwrap();
    let before = fs::read(root.join("original.jpg")).unwrap();
    let lib = Library::open(&tmp.path().join("catalog")).unwrap();
    let id = lib.add_source(root.clone()).unwrap();
    wait(&lib, &id);
    let asset = lib.query(Query::default()).unwrap().assets.remove(0);
    assert_eq!(asset.thumbnail_state, "ready");
    assert_eq!(asset.width, Some(480));
    assert_eq!(asset.height, Some(640));
    assert_eq!(asset.latitude, Some(12.97));
    lib.annotate(
        asset.id.clone(),
        Annotation {
            favorite: true,
            caption: "Family holiday".into(),
            ..Default::default()
        },
    )
    .unwrap();
    fs::rename(root.join("original.jpg"), root.join("renamed.jpg")).unwrap();
    lib.start_scan(&id).unwrap();
    wait(&lib, &id);
    let renamed = lib.query(Query::default()).unwrap();
    assert_eq!(renamed.total, 1);
    assert_eq!(renamed.assets[0].id, asset.id);
    assert!(renamed.assets[0].favorite);
    let backup = tmp.path().join("backup.sqlite");
    lib.backup(&backup).unwrap();
    assert!(
        lib.restore(&tmp.path().join("catalog/catalog.sqlite"))
            .is_err()
    );
    let moved = tmp.path().join("disconnected");
    fs::rename(&root, &moved).unwrap();
    lib.refresh_sources().unwrap();
    assert_eq!(
        lib.query(Query::default()).unwrap().assets[0].availability,
        "offline"
    );
    assert!(!lib.thumbnail(&asset.id).unwrap().is_empty());
    assert!(lib.original(&asset.id).is_err());
    lib.relink(&id, moved.clone()).unwrap();
    lib.start_scan(&id).unwrap();
    wait(&lib, &id);
    assert_eq!(lib.query(Query::default()).unwrap().assets[0].id, asset.id);
    assert_eq!(before, fs::read(moved.join("renamed.jpg")).unwrap());
    lib.annotate(asset.id.clone(), Annotation::default())
        .unwrap();
    lib.restore(&backup).unwrap();
    assert!(lib.asset(&asset.id).unwrap().favorite);
    lib.restore(&tmp.path().join("catalog/before-restore.sqlite"))
        .unwrap();
    assert!(!lib.asset(&asset.id).unwrap().favorite);
}

#[test]
fn mixed_media_failures_and_source_boundaries() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("media");
    fs::create_dir(&root).unwrap();
    let fixtures = fixture().parent().unwrap().to_owned();
    for entry in fs::read_dir(fixtures).unwrap() {
        let e = entry.unwrap();
        fs::copy(e.path(), root.join(e.file_name())).unwrap();
    }
    let outside = tmp.path().join("outside.jpg");
    fs::copy(fixture(), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link.jpg")).unwrap();
    let lib = Library::open(&tmp.path().join("catalog")).unwrap();
    let id = lib.add_source(root.clone()).unwrap();
    wait(&lib, &id);
    assert!(lib.add_source(root.clone()).is_err());
    assert!(lib.add_source(tmp.path().to_owned()).is_err());
    let page = lib.query(Query::default()).unwrap();
    assert_eq!(page.total, 6);
    for asset in &page.assets {
        if asset.filename == "broken.jpg" {
            assert_eq!(asset.metadata_state, "failed");
        } else {
            assert_eq!(
                asset.thumbnail_state, "ready",
                "{} {:?}",
                asset.filename, asset.error
            );
        }
    }
    let a = page
        .assets
        .iter()
        .find(|a| a.filename == "mountain.jpg")
        .unwrap();
    fs::remove_file(root.join("mountain.jpg")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("mountain.jpg")).unwrap();
    assert!(lib.original(&a.id).is_err());
    lib.start_scan(&id).unwrap();
    lib.control_scan(&id, "cancel").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(lib.query(Query::default()).unwrap().total, 6);
}

#[test]
fn budget_and_catalog_lock_are_enforced() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("catalog");
    let root = tmp.path().join("photos");
    fs::create_dir(&root).unwrap();
    fs::copy(fixture(), root.join("photo.jpg")).unwrap();
    let lib = Library::open(&home).unwrap();
    assert!(Library::open(&home).is_err());
    let mut s = lib.overview().unwrap().settings;
    s.thumbnail_budget = 0;
    lib.save_settings(s).unwrap();
    let id = lib.add_source(root).unwrap();
    wait(&lib, &id);
    let a = &lib.query(Query::default()).unwrap().assets[0];
    assert_eq!(a.metadata_state, "ready");
    assert_eq!(a.thumbnail_state, "budget_full");
    assert_eq!(lib.overview().unwrap().thumbnail_bytes, 0);
}

#[test]
fn interrupted_scan_never_marks_offline_media_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("photos");
    fs::create_dir(&root).unwrap();
    for i in 0..40 {
        fs::copy(fixture(), root.join(format!("photo-{i:03}.jpg"))).unwrap();
    }
    let lib = Library::open(&tmp.path().join("catalog")).unwrap();
    let id = lib.add_source(root.clone()).unwrap();
    wait(&lib, &id);
    let first = lib.query(Query::default()).unwrap().assets[0].id.clone();
    lib.annotate(
        first.clone(),
        Annotation {
            favorite: true,
            ..Default::default()
        },
    )
    .unwrap();
    // New revisions make the rescan spend time in the decoder.
    for i in 0..40 {
        let p = root.join(format!("photo-{i:03}.jpg"));
        let bytes = fs::read(&p).unwrap();
        std::thread::sleep(Duration::from_millis(1));
        fs::write(p, bytes).unwrap();
    }
    lib.start_scan(&id).unwrap();
    std::thread::sleep(Duration::from_millis(80));
    let moved = tmp.path().join("unplugged");
    fs::rename(&root, &moved).unwrap();
    let start = Instant::now();
    while lib.require_idle(&id).is_err() {
        assert!(start.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(30));
    }
    lib.refresh_sources().unwrap();
    let page = lib.query(Query::default()).unwrap();
    assert_eq!(page.total, 40);
    assert!(page.assets.iter().all(|a| a.availability == "offline"));
    assert!(lib.asset(&first).unwrap().favorite);
    lib.relink(&id, moved).unwrap();
    lib.start_scan(&id).unwrap();
    wait(&lib, &id);
    assert_eq!(lib.query(Query::default()).unwrap().total, 40);
    assert!(lib.asset(&first).unwrap().favorite);
}

#[test]
fn paths_round_trip_and_symlink_sources_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("photos");
    fs::create_dir(&root).unwrap();
    // APFS rejects invalid UTF-8 at file creation; verify the catalog encoding
    // independently, then exercise an actual Unicode filename through scanning.
    let raw = b"phone-\xff.jpg".to_vec();
    assert_eq!(
        bimb_core::platform::path_bytes(&bimb_core::platform::from_bytes(raw.clone())),
        raw
    );
    let filename = std::ffi::OsString::from("phone-旅行-é.jpg");
    fs::copy(fixture(), root.join(&filename)).unwrap();
    let lib = Library::open(&tmp.path().join("catalog")).unwrap();
    let id = lib.add_source(root.clone()).unwrap();
    wait(&lib, &id);
    let a = &lib.query(Query::default()).unwrap().assets[0];
    assert_eq!(
        lib.original(&a.id).unwrap(),
        root.join(filename).canonicalize().unwrap()
    );
    let link = tmp.path().join("alias");
    std::os::unix::fs::symlink(&root, &link).unwrap();
    assert!(lib.add_source(link).is_err());
}

#[test]
fn monitor_reconciles_attachment_and_subsequent_file_events() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("photos");
    fs::create_dir(&root).unwrap();
    fs::copy(fixture(), root.join("one.jpg")).unwrap();
    let lib = Library::open(&tmp.path().join("catalog")).unwrap();
    let id = lib.add_source(root.clone()).unwrap();
    wait(&lib, &id);
    lib.start_monitor();
    for (name, count) in [("two.jpg", 2), ("three.jpg", 3)] {
        fs::copy(fixture(), root.join(name)).unwrap();
        let start = Instant::now();
        while lib.query(Query::default()).unwrap().total != count {
            assert!(
                start.elapsed() < Duration::from_secs(40),
                "watcher did not index {name}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        wait(&lib, &id);
    }
}
