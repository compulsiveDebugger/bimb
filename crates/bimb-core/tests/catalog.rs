use bimb_core::{
    catalog::Catalog,
    model::{Annotation, Query, Settings},
};
use rusqlite::params;

fn seeded() -> (tempfile::TempDir, Catalog) {
    let tmp = tempfile::tempdir().unwrap();
    let c = Catalog::open(&tmp.path().join("catalog.sqlite")).unwrap();
    c.call(|db|{
        db.execute("INSERT INTO sources(id,name,path,display_path,volume,root_key) VALUES('source','Holiday',X'2F','/','volume','root')",[])?;
        for (id,name,capture) in [("a","Beach summer.jpg",Some("2024-02-29T12:00:00")),("b","Hill's walk.jpg",Some("2023-10-10T13:00:00")),("c","unknown.jpg",None)] {
            db.execute("INSERT INTO assets(id,source_id,path,filename,folder,file_key,revision,kind,size,modified,added,seen,captured) VALUES(?,'source',?,?,'Trip',?,'r1','photo',100,'2025-01-01T00:00:00','2026-01-01',1,?)",params![id,id.as_bytes(),name,id,capture])?;
        }Ok(())
    }).unwrap();
    (tmp, c)
}
#[test]
fn annotations_are_searchable_and_pagination_has_stable_ties() {
    let (_tmp, c) = seeded();
    c.annotate(
        "a".into(),
        Annotation {
            tags: vec!["family".into()],
            place: "Goa".into(),
            favorite: true,
            ..Default::default()
        },
    )
    .unwrap();
    let page = c
        .query(Query {
            text: "family Goa".into(),
            favorites: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.assets[0].id, "a");
    let first = c
        .query(Query {
            sort: "size".into(),
            limit: 2,
            ..Default::default()
        })
        .unwrap();
    let second = c
        .query(Query {
            sort: "size".into(),
            limit: 2,
            cursor: first.next_cursor,
            ..Default::default()
        })
        .unwrap();
    let ids: std::collections::HashSet<_> = first
        .assets
        .iter()
        .chain(second.assets.iter())
        .map(|a| a.id.clone())
        .collect();
    assert_eq!(ids.len(), 3);
    for text in ["\"", "AND", "Hill's", "a OR b", "' ; DROP TABLE assets --"] {
        assert!(
            c.query(Query {
                text: text.into(),
                ..Default::default()
            })
            .is_ok()
        );
    }
}
#[test]
fn annotations_validate_dates_and_cursor_rejects_changed_catalog() {
    let (_tmp, c) = seeded();
    assert!(
        c.annotate(
            "a".into(),
            Annotation {
                date_override: "2025-02-29".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
    let first = c
        .query(Query {
            limit: 1,
            ..Default::default()
        })
        .unwrap();
    c.annotate(
        "a".into(),
        Annotation {
            date_override: "2024-02-29".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        c.query(Query {
            limit: 1,
            cursor: first.next_cursor,
            ..Default::default()
        })
        .unwrap_err()
        .to_string()
        .contains("library_changed")
    );
    assert!(
        c.query(Query {
            date_from: "2024-02-29".into(),
            date_to: "2024-02-29".into(),
            ..Default::default()
        })
        .unwrap()
        .assets
        .iter()
        .all(|a| a.id == "a")
    );
}
#[test]
fn memories_respect_provenance_and_exclusions() {
    use chrono::Datelike;
    let (_tmp, c) = seeded();
    let today = chrono::Local::now();
    let capture = format!("{}-{}T12:00:00", today.year() - 1, today.format("%m-%d"));
    c.call(move |db| {
        db.execute(
            "UPDATE assets SET captured=? WHERE id IN ('a','b')",
            [&capture],
        )?;
        db.execute("UPDATE assets SET modified=? WHERE id='c'", [capture])?;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        c.query(Query {
            memory: true,
            ..Default::default()
        })
        .unwrap()
        .total,
        2
    );
    c.annotate(
        "a".into(),
        Annotation {
            exclude_memory: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        c.query(Query {
            memory: true,
            ..Default::default()
        })
        .unwrap()
        .total,
        1
    );
    c.call(|db| {
        db.execute(
            "INSERT INTO meta VALUES('settings',?)",
            [serde_json::to_string(&Settings {
                memories_enabled: false,
                ..Default::default()
            })?],
        )?;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        c.query(Query {
            memory: true,
            ..Default::default()
        })
        .unwrap()
        .total,
        0
    );
}

#[test]
fn scan_issue_pages_are_source_scoped_and_reject_a_new_scan() {
    use bimb_core::platform;
    let (_tmp, c) = seeded();
    c.call(|db| {
        db.execute("INSERT INTO sources(id,name,path,display_path,volume,root_key) VALUES('other','Other',X'2F','/','volume','other')", [])?;
        for index in 0..205 {
            let path = format!("旅行/Folder {index}");
            db.execute("INSERT INTO issues(source_id,code,path,message) VALUES('source','folder_unreadable',?,'Permission denied')", [platform::path_bytes(std::path::Path::new(&path))])?;
        }
        db.execute("INSERT INTO issues(source_id,code) VALUES('other','scan_interrupted')", [])?;
        db.execute("UPDATE assets SET error='unsupported_media' WHERE id='a'", [])?;
        Ok(())
    }).unwrap();
    let first = c.source_issues("source", None).unwrap();
    assert_eq!(first.total, 205);
    assert_eq!(first.issues.len(), 100);
    assert_eq!(first.issues[0].path.as_deref(), Some("旅行/Folder 0"));
    assert_eq!(first.issues[0].message, "Permission denied");
    assert!(c.source_issues("other", first.next_cursor.clone()).is_err());
    let second = c
        .source_issues("source", first.next_cursor.clone())
        .unwrap();
    let third = c.source_issues("source", second.next_cursor).unwrap();
    assert_eq!(third.issues.len(), 5);
    assert!(third.next_cursor.is_none());
    let ids: std::collections::HashSet<_> = first
        .issues
        .iter()
        .chain(second.issues.iter())
        .chain(third.issues.iter())
        .map(|issue| issue.id)
        .collect();
    assert_eq!(ids.len(), 205);
    let source = c
        .overview()
        .unwrap()
        .sources
        .into_iter()
        .find(|s| s.id == "source")
        .unwrap();
    assert_eq!(source.media_errors, 1);
    assert_eq!(source.scan_errors, 205);
    assert_eq!(source.errors, 206);
    c.call(|db| {
        db.execute("DELETE FROM issues WHERE source_id='source'", [])?;
        db.execute(
            "UPDATE sources SET generation=generation+1 WHERE id='source'",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    assert!(
        c.source_issues("source", first.next_cursor)
            .unwrap_err()
            .to_string()
            .contains("scan_changed")
    );
    assert!(c.source_issues("missing", None).is_err());
    assert!(c.source_issues("source", Some("invalid".into())).is_err());
}

#[test]
#[cfg(unix)]
fn version_one_catalogs_preserve_legacy_issues_and_upgrade_once() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("catalog.sqlite");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(bimb_core::catalog::SCHEMA).unwrap();
    db.execute_batch("ALTER TABLE sources DROP COLUMN relative_root; ALTER TABLE sources DROP COLUMN filesystem; ALTER TABLE sources DROP COLUMN drive_kind; ALTER TABLE sources DROP COLUMN read_only; DELETE FROM meta WHERE key='path_encoding'; DROP TABLE collections; DROP TABLE issues; CREATE TABLE issues(id INTEGER PRIMARY KEY,source_id TEXT REFERENCES sources(id) ON DELETE CASCADE,code TEXT NOT NULL); PRAGMA user_version=1;").unwrap();
    db.execute("INSERT INTO sources(id,name,path,display_path,volume,root_key) VALUES('source','Holiday',X'2F','/','volume','root')", []).unwrap();
    db.execute(
        "INSERT INTO issues(source_id,code) VALUES('source','folder_unreadable')",
        [],
    )
    .unwrap();
    drop(db);
    let catalog = Catalog::open(&path).unwrap();
    let page = catalog.source_issues("source", None).unwrap();
    assert_eq!(page.total, 1);
    assert!(page.issues[0].path.is_none());
    assert!(page.issues[0].message.is_empty());
    let version: i64 = catalog
        .call(|db| Ok(db.query_row("PRAGMA user_version", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(version, bimb_core::catalog::SCHEMA_VERSION);
    // Opening a migrated catalog again must not attempt a duplicate ALTER TABLE.
    assert_eq!(
        Catalog::open(&path)
            .unwrap()
            .source_issues("source", None)
            .unwrap()
            .total,
        1
    );
}

#[test]
fn saved_collections_recompute_matches_and_preserve_filters_after_reopen() {
    let (tmp, c) = seeded();
    let id = c
        .save_collection(
            String::new(),
            "  Family in Goa  ".into(),
            Query {
                text: "family".into(),
                source: "source".into(),
                place: "Goa".into(),
                favorites: true,
                sort: "name".into(),
                ascending: true,
                cursor: Some("stale page".into()),
                limit: 1,
                ..Default::default()
            },
        )
        .unwrap();
    let collection = c.overview().unwrap().collections.remove(0);
    assert_eq!(collection.name, "Family in Goa");
    assert!(collection.query.cursor.is_none());
    assert_eq!(collection.query.limit, Query::default().limit);
    assert_eq!(c.query(collection.query.clone()).unwrap().total, 0);
    for asset in ["a", "b"] {
        c.annotate(
            asset.into(),
            Annotation {
                favorite: true,
                place: "Goa".into(),
                tags: vec!["family".into()],
                ..Default::default()
            },
        )
        .unwrap();
    }
    let page = c.query(collection.query.clone()).unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.assets[0].id, "a");
    c.annotate("a".into(), Annotation::default()).unwrap();
    assert_eq!(c.query(collection.query.clone()).unwrap().total, 1);
    // Newly indexed rows become matches without editing a collection membership.
    c.call(|db| {
        db.execute("INSERT INTO assets(id,source_id,path,filename,folder,file_key,revision,kind,size,modified,added,seen,favorite,place,tags) VALUES('d','source',X'64','another.jpg','Trip','d','r1','photo',100,'2025-01-01','2026-01-01',1,1,'Goa','[\"family\"]')", [])?;
        Ok(())
    }).unwrap();
    assert_eq!(c.query(collection.query).unwrap().total, 2);
    let reopened = Catalog::open(&tmp.path().join("catalog.sqlite")).unwrap();
    let saved = reopened.overview().unwrap().collections.remove(0);
    assert_eq!(saved.id, id);
    assert_eq!(saved.query.text, "family");
    assert!(saved.query.ascending);
    assert_eq!(
        reopened.query(saved.query.clone()).unwrap().assets[0].id,
        "d"
    );
    // Removing a source must not silently widen the search to the entire library.
    reopened
        .call(|db| {
            db.execute("DELETE FROM sources WHERE id='source'", [])?;
            Ok(())
        })
        .unwrap();
    assert_eq!(reopened.query(saved.query).unwrap().total, 0);
    assert_eq!(reopened.overview().unwrap().collections.len(), 1);
}

#[test]
fn collection_edits_validate_input_and_never_change_media_or_albums() {
    let (_tmp, c) = seeded();
    c.call(|db| {
        db.execute("INSERT INTO albums VALUES('album','Holiday')", [])?;
        db.execute("INSERT INTO memberships VALUES('album','a',1)", [])?;
        Ok(())
    })
    .unwrap();
    let query = Query {
        text: "旅行".into(),
        source: "source".into(),
        kind: "video".into(),
        favorites: true,
        availability: "offline".into(),
        date_from: "2024-02-29".into(),
        date_to: "2025-01-01".into(),
        place: "Goa".into(),
        folder: "Trip".into(),
        album: "album".into(),
        memory: true,
        errors: true,
        gps: true,
        sort: "duration".into(),
        ascending: true,
        ..Default::default()
    };
    let id = c
        .save_collection(String::new(), "旅行".into(), query.clone())
        .unwrap();
    assert_eq!(
        serde_json::to_value(&c.overview().unwrap().collections[0].query).unwrap(),
        serde_json::to_value(&query).unwrap()
    );
    assert!(
        c.save_collection(String::new(), "旅行".into(), Query::default())
            .is_err()
    );
    assert!(
        c.save_collection(String::new(), " ".into(), Query::default())
            .is_err()
    );
    assert!(
        c.save_collection(String::new(), "x".repeat(129), Query::default())
            .is_err()
    );
    for invalid in [
        Query {
            text: "x".repeat(1025),
            ..Default::default()
        },
        Query {
            date_from: "2025-02-29".into(),
            ..Default::default()
        },
        Query {
            date_from: "2025-01-01".into(),
            date_to: "2024-01-01".into(),
            ..Default::default()
        },
        Query {
            sort: "unsupported".into(),
            ..Default::default()
        },
    ] {
        assert!(
            c.save_collection(id.clone(), "Invalid replacement".into(), invalid)
                .is_err()
        );
    }
    assert_eq!(c.overview().unwrap().collections[0].name, "旅行");
    assert!(
        c.save_collection("missing".into(), "Missing".into(), Query::default())
            .is_err()
    );
    c.save_collection(
        id.clone(),
        "Renamed".into(),
        Query {
            album: "album".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let saved = c.overview().unwrap().collections.remove(0);
    assert_eq!(saved.id, id);
    assert_eq!(saved.name, "Renamed");
    assert_eq!(c.query(saved.query.clone()).unwrap().total, 1);
    c.call(|db| {
        db.execute("DELETE FROM albums WHERE id='album'", [])?;
        Ok(())
    })
    .unwrap();
    assert_eq!(c.query(saved.query).unwrap().total, 0);
    c.delete_collection(id.clone()).unwrap();
    assert!(c.delete_collection(id).is_err());
    assert!(c.overview().unwrap().collections.is_empty());
    assert_eq!(c.query(Query::default()).unwrap().total, 3);
}

#[test]
#[cfg(unix)]
fn version_two_catalogs_gain_collections_without_changing_existing_organization() {
    let (tmp, c) = seeded();
    c.annotate(
        "a".into(),
        Annotation {
            favorite: true,
            caption: "Keep me".into(),
            ..Default::default()
        },
    )
    .unwrap();
    c.call(|db| {
        db.execute_batch("INSERT INTO albums VALUES('album','Holiday'); INSERT INTO memberships VALUES('album','a',1); INSERT INTO issues(source_id,code,message) VALUES('source','folder_unreadable','Permission denied'); ALTER TABLE sources DROP COLUMN relative_root; ALTER TABLE sources DROP COLUMN filesystem; ALTER TABLE sources DROP COLUMN drive_kind; ALTER TABLE sources DROP COLUMN read_only; DELETE FROM meta WHERE key='path_encoding'; DROP TABLE collections; PRAGMA user_version=2;")?;
        Ok(())
    }).unwrap();
    let migrated = Catalog::open(&tmp.path().join("catalog.sqlite")).unwrap();
    let overview = migrated.overview().unwrap();
    assert!(overview.collections.is_empty());
    assert_eq!(overview.albums[0].count, 1);
    assert_eq!(overview.favorites, 1);
    assert_eq!(
        migrated.source_issues("source", None).unwrap().issues[0].message,
        "Permission denied"
    );
    let id = migrated
        .save_collection(
            String::new(),
            "Favorites".into(),
            Query {
                favorites: true,
                ..Default::default()
            },
        )
        .unwrap();
    let again = Catalog::open(&tmp.path().join("catalog.sqlite")).unwrap();
    assert_eq!(again.overview().unwrap().collections[0].id, id);
    assert_eq!(
        again
            .query(Query {
                favorites: true,
                ..Default::default()
            })
            .unwrap()
            .assets[0]
            .caption,
        "Keep me"
    );
}

#[test]
fn catalogs_reject_foreign_native_paths_before_migration() {
    let (tmp, c) = seeded();
    c.call(|db| {
        let other = if cfg!(windows) {
            "unix-bytes"
        } else {
            "windows-utf16le"
        };
        db.execute("UPDATE meta SET value=? WHERE key='path_encoding'", [other])?;
        Ok(())
    })
    .unwrap();
    let error = match Catalog::open(&tmp.path().join("catalog.sqlite")) {
        Ok(_) => panic!("foreign paths must not be decoded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("another operating system"));
    assert_eq!(c.query(Query::default()).unwrap().total, 3);
}

#[test]
#[cfg(unix)]
fn version_three_preserves_collections_and_backfills_drive_fields() {
    let (tmp, c) = seeded();
    let collection = c
        .save_collection(
            String::new(),
            "Favorites".into(),
            Query {
                favorites: true,
                ..Default::default()
            },
        )
        .unwrap();
    c.call(|db| {
        db.execute_batch("ALTER TABLE sources DROP COLUMN relative_root; ALTER TABLE sources DROP COLUMN filesystem; ALTER TABLE sources DROP COLUMN drive_kind; ALTER TABLE sources DROP COLUMN read_only; DELETE FROM meta WHERE key='path_encoding'; PRAGMA user_version=3;")?;
        Ok(())
    }).unwrap();
    let migrated = Catalog::open(&tmp.path().join("catalog.sqlite")).unwrap();
    let info = migrated.overview().unwrap();
    assert_eq!(info.collections[0].id, collection);
    assert_eq!(info.sources[0].filesystem, "");
    assert_eq!(info.sources[0].read_only, None);
    assert!(migrated.source("source").unwrap().relative_root.is_none());
    assert_eq!(migrated.query(Query::default()).unwrap().total, 3);
}
