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
