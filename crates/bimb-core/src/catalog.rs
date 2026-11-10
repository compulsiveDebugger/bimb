use crate::{model::*, platform};
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Datelike, Local};
use rusqlite::{Connection, OptionalExtension, params, types::Value};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

type Work = Box<dyn FnOnce(&mut Connection) + Send>;
#[derive(Clone)]
pub struct Catalog {
    sender: mpsc::SyncSender<Work>,
}

#[derive(Clone, Debug)]
pub struct SourceRecord {
    pub id: String,
    pub root: PathBuf,
    pub volume: String,
    pub root_key: String,
    pub generation: i64,
    pub exclusions: Vec<String>,
    pub relative_root: Option<PathBuf>,
}

pub const SCHEMA: &str = r#"
PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT OR IGNORE INTO meta VALUES('generation','0');
CREATE TABLE IF NOT EXISTS sources(
 id TEXT PRIMARY KEY, name TEXT NOT NULL, path BLOB NOT NULL, display_path TEXT NOT NULL,
 volume TEXT NOT NULL, root_key TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'online',
 scan_state TEXT NOT NULL DEFAULT 'idle', generation INTEGER NOT NULL DEFAULT 0,
 last_scan TEXT, exclusions TEXT NOT NULL DEFAULT '[]', relative_root BLOB,
 filesystem TEXT NOT NULL DEFAULT '', drive_kind TEXT NOT NULL DEFAULT '', read_only INTEGER);
CREATE TABLE IF NOT EXISTS assets(
 id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
 path BLOB NOT NULL, filename TEXT NOT NULL, folder TEXT NOT NULL, file_key TEXT NOT NULL,
 revision TEXT NOT NULL, kind TEXT NOT NULL, size INTEGER NOT NULL, modified TEXT NOT NULL,
 added TEXT NOT NULL, seen INTEGER NOT NULL, state TEXT NOT NULL DEFAULT 'present',
 metadata TEXT NOT NULL DEFAULT '{}', captured TEXT, metadata_state TEXT NOT NULL DEFAULT 'queued',
 thumbnail_state TEXT NOT NULL DEFAULT 'queued', thumbnail_bytes INTEGER NOT NULL DEFAULT 0,
 error TEXT, attempts INTEGER NOT NULL DEFAULT 0,
 favorite INTEGER NOT NULL DEFAULT 0, caption TEXT NOT NULL DEFAULT '', tags TEXT NOT NULL DEFAULT '[]',
 place TEXT NOT NULL DEFAULT '', date_override TEXT NOT NULL DEFAULT '', exclude_memory INTEGER NOT NULL DEFAULT 0,
 UNIQUE(source_id,path));
CREATE INDEX IF NOT EXISTS asset_source ON assets(source_id);
CREATE INDEX IF NOT EXISTS asset_identity ON assets(source_id,file_key);
CREATE INDEX IF NOT EXISTS asset_capture ON assets(captured,id);
CREATE TABLE IF NOT EXISTS albums(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE);
CREATE TABLE IF NOT EXISTS collections(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,query TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS memberships(album_id TEXT REFERENCES albums(id) ON DELETE CASCADE,
 asset_id TEXT REFERENCES assets(id) ON DELETE CASCADE, position INTEGER NOT NULL, PRIMARY KEY(album_id,asset_id));
CREATE TABLE IF NOT EXISTS issues(id INTEGER PRIMARY KEY,source_id TEXT REFERENCES sources(id) ON DELETE CASCADE,
 code TEXT NOT NULL,path BLOB,message TEXT NOT NULL DEFAULT '');
CREATE INDEX IF NOT EXISTS issue_source ON issues(source_id,id);
CREATE VIRTUAL TABLE IF NOT EXISTS search USING fts5(asset_id UNINDEXED, text, tokenize='unicode61 remove_diacritics 2');
CREATE TRIGGER IF NOT EXISTS asset_search_insert AFTER INSERT ON assets BEGIN
 INSERT INTO search(asset_id,text) VALUES(new.id,new.filename||' '||new.folder||' '||new.caption||' '||new.tags||' '||new.place||' '||coalesce(json_extract(new.metadata,'$.camera'),'')); END;
CREATE TRIGGER IF NOT EXISTS asset_search_delete AFTER DELETE ON assets BEGIN DELETE FROM search WHERE asset_id=old.id; END;
CREATE TRIGGER IF NOT EXISTS asset_search_update AFTER UPDATE OF filename,folder,caption,tags,place,metadata ON assets BEGIN
 DELETE FROM search WHERE asset_id=old.id;
 INSERT INTO search(asset_id,text) VALUES(new.id,new.filename||' '||new.folder||' '||new.caption||' '||new.tags||' '||new.place||' '||coalesce(json_extract(new.metadata,'$.camera'),'')); END;
CREATE VIEW IF NOT EXISTS library AS SELECT a.*,s.name AS source_name,
 CASE WHEN a.date_override<>'' THEN a.date_override WHEN a.captured IS NOT NULL THEN a.captured ELSE a.modified END AS effective_date,
 CASE WHEN a.date_override<>'' THEN 'user' WHEN a.captured IS NOT NULL THEN 'embedded' ELSE 'file_modified' END AS date_source,
 CASE WHEN s.state<>'online' THEN s.state ELSE a.state END AS availability
 FROM assets a JOIN sources s ON s.id=a.source_id;
"#;

pub const SCHEMA_VERSION: i64 = 4;

pub(crate) fn migrate(connection: &mut Connection) -> Result<()> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        bail!("This catalog needs a newer Bimb version");
    }
    // Schemas 1–3 shipped only on macOS, with Unix-byte paths. Never decode them as UTF-16.
    let encoding: Option<String> = if version == 0 {
        None
    } else {
        connection
            .query_row(
                "SELECT value FROM meta WHERE key='path_encoding'",
                [],
                |r| r.get(0),
            )
            .optional()?
    };
    let encoding = encoding.as_deref().unwrap_or(if version > 0 {
        "unix-bytes"
    } else {
        platform::path_encoding()
    });
    if encoding != platform::path_encoding() {
        bail!(
            "This catalog uses paths from another operating system. Cross-OS catalog remapping is not supported yet; use a separate catalog on each OS."
        );
    }
    let tx = connection.transaction()?;
    if version == 1 {
        tx.execute_batch("ALTER TABLE issues ADD COLUMN path BLOB; ALTER TABLE issues ADD COLUMN message TEXT NOT NULL DEFAULT '';")?;
    }
    if (1..=3).contains(&version) {
        tx.execute_batch("ALTER TABLE sources ADD COLUMN relative_root BLOB; ALTER TABLE sources ADD COLUMN filesystem TEXT NOT NULL DEFAULT ''; ALTER TABLE sources ADD COLUMN drive_kind TEXT NOT NULL DEFAULT ''; ALTER TABLE sources ADD COLUMN read_only INTEGER;")?;
    }
    tx.execute_batch(SCHEMA)?;
    tx.execute(
        "INSERT OR IGNORE INTO meta VALUES('path_encoding',?)",
        [platform::path_encoding()],
    )?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}

pub fn bump(c: &Connection) -> Result<()> {
    c.execute(
        "UPDATE meta SET value=CAST(value AS INTEGER)+1 WHERE key='generation'",
        [],
    )?;
    Ok(())
}
pub fn settings(c: &Connection) -> Result<Settings> {
    let value: Option<String> = c
        .query_row("SELECT value FROM meta WHERE key='settings'", [], |r| {
            r.get(0)
        })
        .optional()?;
    value
        .map(|v| serde_json::from_str(&v).map_err(Into::into))
        .unwrap_or_else(|| Ok(Settings::default()))
}
fn collections(c: &Connection) -> Result<Vec<Collection>> {
    let rows = c
        .prepare("SELECT id,name,query FROM collections ORDER BY name COLLATE NOCASE,id")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, String>(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(id, name, query)| {
            Ok(Collection {
                id,
                name,
                query: serde_json::from_str(&query)?,
            })
        })
        .collect()
}

fn collection_query(mut q: Query) -> Result<Query> {
    if q.text.len() > 1024
        || q.source.len() > 128
        || q.album.len() > 128
        || q.folder.len() > 4096
        || q.place.len() > 256
        || !["", "photo", "video"].contains(&q.kind.as_str())
        || ![
            "",
            "present",
            "offline",
            "missing",
            "unavailable",
            "ambiguous",
        ]
        .contains(&q.availability.as_str())
        || !["capture", "added", "name", "size", "duration"].contains(&q.sort.as_str())
    {
        bail!("Invalid collection filters");
    }
    for date in [&q.date_from, &q.date_to] {
        if !date.is_empty() {
            if date.len() != 10 {
                bail!("Use YYYY-MM-DD for collection dates");
            }
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .context("Use YYYY-MM-DD for collection dates")?;
        }
    }
    if !q.date_from.is_empty() && !q.date_to.is_empty() && q.date_from > q.date_to {
        bail!("Date range starts after it ends");
    }
    // A saved search always starts from the first page, independently of the
    // caller's current cursor or page size.
    q.cursor = None;
    q.limit = Query::default().limit;
    Ok(q)
}
pub fn source(c: &Connection, id: &str) -> Result<SourceRecord> {
    Ok(c.query_row(
        "SELECT id,path,volume,root_key,generation,exclusions,relative_root FROM sources WHERE id=?",
        [id],
        |r| {
            let exclusions: String = r.get(5)?;
            Ok(SourceRecord {
                id: r.get(0)?,
                root: platform::from_bytes(r.get(1)?),
                volume: r.get(2)?,
                root_key: r.get(3)?,
                generation: r.get(4)?,
                exclusions: serde_json::from_str(&exclusions).unwrap_or_default(),
                relative_root: r.get::<_,Option<Vec<u8>>>(6)?.map(platform::from_bytes),
            })
        },
    )?)
}
pub fn asset(r: &rusqlite::Row<'_>) -> rusqlite::Result<Asset> {
    let meta: String = r.get("metadata")?;
    let m: Metadata = serde_json::from_str(&meta).unwrap_or_default();
    let tags: String = r.get("tags")?;
    Ok(Asset {
        id: r.get("id")?,
        source_id: r.get("source_id")?,
        source_name: r.get("source_name")?,
        filename: r.get("filename")?,
        folder: r.get("folder")?,
        kind: r.get("kind")?,
        size: r.get("size")?,
        modified: r.get("modified")?,
        added: r.get("added")?,
        captured: r.get("captured")?,
        date_source: r.get("date_source")?,
        effective_date: r.get("effective_date")?,
        revision: r.get("revision")?,
        width: m.width,
        height: m.height,
        duration: m.duration,
        camera: m.camera,
        latitude: m.latitude,
        longitude: m.longitude,
        metadata_state: r.get("metadata_state")?,
        thumbnail_state: r.get("thumbnail_state")?,
        error: r.get("error")?,
        availability: r.get("availability")?,
        favorite: r.get("favorite")?,
        caption: r.get("caption")?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        place: r.get("place")?,
        date_override: r.get("date_override")?,
        exclude_memory: r.get("exclude_memory")?,
    })
}

impl Catalog {
    pub fn open(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(3))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        migrate(&mut connection)?;
        connection.execute(
            "UPDATE sources SET scan_state='interrupted' WHERE scan_state IN ('scanning','queued')",
            [],
        )?;
        let (sender, receiver) = mpsc::sync_channel::<Work>(64);
        std::thread::Builder::new()
            .name("bimb-catalog".into())
            .spawn(move || {
                for task in receiver {
                    task(&mut connection);
                }
            })?;
        Ok(Self { sender })
    }
    pub fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.sender
            .send(Box::new(move |c| {
                let _ = tx.send(f(c));
            }))
            .map_err(|_| anyhow::anyhow!("Catalog stopped"))?;
        rx.recv().context("Catalog stopped")?
    }
    pub fn source(&self, id: &str) -> Result<SourceRecord> {
        let id = id.to_owned();
        self.call(move |c| source(c, &id))
    }
    pub fn source_issues(&self, id: &str, cursor: Option<String>) -> Result<SourceIssuePage> {
        let id = id.to_owned();
        self.call(move |c| {
            let generation: i64 = c.query_row("SELECT generation FROM sources WHERE id=?", [&id], |r| r.get(0))?;
            let mut after = 0;
            if let Some(cursor) = cursor {
                let decoded = URL_SAFE_NO_PAD.decode(cursor).context("Invalid issue cursor")?;
                let (source, scan, last): (String, i64, i64) = serde_json::from_slice(&decoded).context("Invalid issue cursor")?;
                if source != id || scan != generation {
                    bail!("scan_changed");
                }
                after = last;
            }
            let total = c.query_row("SELECT count(*) FROM issues WHERE source_id=?", [&id], |r| r.get(0))?;
            let mut issues = c.prepare("SELECT id,path,code,message FROM issues WHERE source_id=? AND id>? ORDER BY id LIMIT 101")?
                .query_map(params![id, after], |r| {
                    let path: Option<Vec<u8>> = r.get(1)?;
                    Ok(SourceIssue { id:r.get(0)?, path:path.map(|p|platform::from_bytes(p).to_string_lossy().into_owned()), code:r.get(2)?, message:r.get(3)? })
                })?.collect::<rusqlite::Result<Vec<_>>>()?;
            let next_cursor = if issues.len() > 100 {
                issues.truncate(100);
                Some(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&(&id, generation, issues[99].id))?))
            } else { None };
            Ok(SourceIssuePage { issues, total, next_cursor })
        })
    }
    pub fn overview(&self) -> Result<Overview> {
        self.call(|c| {
            let sources = c.prepare("SELECT s.*, (SELECT count(*) FROM assets WHERE source_id=s.id) total,(SELECT count(*) FROM assets WHERE source_id=s.id AND error IS NOT NULL) media_errors,(SELECT count(*) FROM issues WHERE source_id=s.id) scan_errors FROM sources s ORDER BY name")?
                .query_map([], |r| { let ex:String=r.get("exclusions")?; Ok(Source {
                    id:r.get("id")?,name:r.get("name")?,path:r.get("display_path")?,volume_id:r.get("volume")?,state:r.get("state")?,scan_state:r.get("scan_state")?,filesystem:r.get("filesystem")?,drive_kind:r.get("drive_kind")?,read_only:r.get("read_only")?,
                    total:r.get("total")?,errors:r.get::<_,i64>("media_errors")?+r.get::<_,i64>("scan_errors")?,media_errors:r.get("media_errors")?,scan_errors:r.get("scan_errors")?,last_scan:r.get("last_scan")?,exclusions:serde_json::from_str(&ex).unwrap_or_default() }) })?.collect::<rusqlite::Result<Vec<_>>>()?;
            let albums=c.prepare("SELECT a.id,a.name,count(m.asset_id) FROM albums a LEFT JOIN memberships m ON a.id=m.album_id GROUP BY a.id ORDER BY a.name")?.query_map([],|r|Ok(Album{id:r.get(0)?,name:r.get(1)?,count:r.get(2)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let (total,photos,videos,favorites,bytes,thumbnail_bytes) = c.query_row("SELECT count(*),coalesce(sum(kind='photo'),0),coalesce(sum(kind='video'),0),coalesce(sum(favorite),0),coalesce(sum(size),0),coalesce(sum(thumbnail_bytes),0) FROM assets",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get::<_,i64>(5)?)))?;
            let folders=c.prepare("SELECT DISTINCT folder FROM assets WHERE folder<>'' ORDER BY folder LIMIT 1000")?.query_map([],|r|r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
            let places=c.prepare("SELECT DISTINCT place FROM assets WHERE place<>'' ORDER BY place LIMIT 1000")?.query_map([],|r|r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
            Ok(Overview{sources,albums,collections:collections(c)?,settings:settings(c)?,total,photos,videos,favorites,bytes,thumbnail_bytes:thumbnail_bytes as u64,folders,places})
        })
    }
    pub fn save_collection(&self, id: String, name: String, query: Query) -> Result<String> {
        let name = name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 128 {
            bail!("Name your collection using up to 128 characters");
        }
        let query = collection_query(query)?;
        self.call(move |c| {
            let tx = c.transaction()?;
            let duplicate: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM collections WHERE name=? AND id<>?)",
                params![name, id],
                |r| r.get(0),
            )?;
            if duplicate {
                bail!("A collection with that name already exists");
            }
            let query = serde_json::to_string(&query)?;
            let result = if id.is_empty() {
                let count: i64 =
                    tx.query_row("SELECT count(*) FROM collections", [], |r| r.get(0))?;
                if count >= 100 {
                    bail!("This preview supports up to 100 saved collections");
                }
                let id = uuid::Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO collections(id,name,query) VALUES(?,?,?)",
                    params![id, name, query],
                )?;
                id
            } else {
                if tx.execute(
                    "UPDATE collections SET name=?,query=? WHERE id=?",
                    params![name, query, id],
                )? != 1
                {
                    bail!("Collection no longer exists in this catalog");
                }
                id
            };
            bump(&tx)?;
            tx.commit()?;
            Ok(result)
        })
    }
    pub fn delete_collection(&self, id: String) -> Result<()> {
        self.call(move |c| {
            let tx = c.transaction()?;
            if tx.execute("DELETE FROM collections WHERE id=?", [id])? != 1 {
                bail!("Collection no longer exists in this catalog");
            }
            bump(&tx)?;
            tx.commit()?;
            Ok(())
        })
    }
    pub fn query(&self, mut q: Query) -> Result<Page> {
        if q.text.len() > 1024 {
            bail!("Search is too long");
        }
        q.limit = q.limit.clamp(1, 240);
        self.call(move |c| {
            let generation:String=c.query_row("SELECT value FROM meta WHERE key='generation'",[],|r|r.get(0))?;
            let cursor=q.cursor.take(); let query_identity=serde_json::to_string(&q)?;
            let config=settings(c)?;
            let mut clauses=vec!["1=1".to_string()]; let mut values:Vec<Value>=vec![];
            let mut filter=|sql:&str,value:String| {clauses.push(sql.into()); values.push(value.into());};
            if !q.text.trim().is_empty() {
                let terms=q.text.split_whitespace().map(|s|format!("\"{}\"*",s.replace('"',"\"\""))).collect::<Vec<_>>().join(" AND ");
                filter("id IN (SELECT asset_id FROM search WHERE search MATCH ?)",terms);
            }
            if !q.source.is_empty() { filter("source_id=?",q.source.clone()); }
            if !q.kind.is_empty() { filter("kind=?",q.kind.clone()); }
            if !q.availability.is_empty() { filter("availability=?",q.availability.clone()); }
            if !q.date_from.is_empty() { filter("substr(effective_date,1,10)>=?",q.date_from.clone()); }
            if !q.date_to.is_empty() { filter("substr(effective_date,1,10)<=?",q.date_to.clone()); }
            if !q.place.is_empty() { filter("place=?",q.place.clone()); }
            if !q.folder.is_empty() { filter("folder=?",q.folder.clone()); }
            if !q.album.is_empty() { filter("id IN (SELECT asset_id FROM memberships WHERE album_id=?)",q.album.clone()); }
            if q.memory {
                let today=Local::now();
                filter("substr(effective_date,6,5)=?",today.format("%m-%d").to_string());
                filter("substr(effective_date,1,4)<?",today.year().to_string());
                clauses.push("exclude_memory=0 AND date_source<>'file_modified'".into());
                if !config.memories_enabled {clauses.push("0=1".into());}
                for [from,to] in config.excluded_dates { clauses.push("substr(effective_date,1,10) NOT BETWEEN ? AND ?".into()); values.push(from.into());values.push(to.into()); }
            }
            if q.favorites { clauses.push("favorite=1".into()); }
            if q.errors { clauses.push("error IS NOT NULL".into()); }
            if q.gps { clauses.push("json_extract(metadata,'$.latitude') IS NOT NULL".into()); }
            let where_sql=clauses.join(" AND ");
            let total=c.query_row(&format!("SELECT count(*) FROM library WHERE {where_sql}"),rusqlite::params_from_iter(values.iter()),|r|r.get(0))?;
            let key=match q.sort.as_str() {
                "name"=>"lower(filename)", "added"=>"added", "size"=>"printf('%020d',size)",
                "duration"=>"printf('%020.3f',coalesce(json_extract(metadata,'$.duration'),-1))", _=>"effective_date",
            };
            let null_key=if q.sort=="duration" {"(json_extract(metadata,'$.duration') IS NULL)"} else {"0"};
            let order=if q.ascending {"ASC"} else {"DESC"}; let comparator=if q.ascending {">"} else {"<"};
            let mut extra=String::new();
            if let Some(cursor)=cursor {
                let decoded=URL_SAFE_NO_PAD.decode(cursor).context("Invalid page cursor")?;
                let parts:(String,String,i64,String,String)=serde_json::from_slice(&decoded).context("Invalid page cursor")?;
                if parts.0!=generation || parts.1!=query_identity { bail!("library_changed"); }
                extra=format!(" AND (null_key > ? OR (null_key = ? AND (sort_key {comparator} ? OR (sort_key = ? AND id {comparator} ?))))");
                values.extend([parts.2.into(),parts.2.into(),parts.3.clone().into(),parts.3.into(),parts.4.into()]);
            }
            let sql=format!("WITH ranked AS (SELECT library.*, {key} sort_key, {null_key} null_key FROM library WHERE {where_sql}) SELECT * FROM ranked WHERE 1=1 {extra} ORDER BY null_key ASC,sort_key {order},id {order} LIMIT {}",q.limit+1);
            let rows=c.prepare(&sql)?.query_map(rusqlite::params_from_iter(values.iter()),|r|Ok((asset(r)?,r.get::<_,String>("sort_key")?,r.get::<_,i64>("null_key")?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let has_more=rows.len()>q.limit;
            let rows:Vec<_>=rows.into_iter().take(q.limit).collect();
            let next_cursor=if has_more {rows.last().map(|(a,k,n)|URL_SAFE_NO_PAD.encode(serde_json::to_vec(&(generation,query_identity,n,k,&a.id)).unwrap()))}else{None};
            Ok(Page{assets:rows.into_iter().map(|(a,_,_)|a).collect(),total,next_cursor})
        })
    }
    pub fn annotate(&self, id: String, mut a: Annotation) -> Result<()> {
        if a.caption.len() > 8192
            || a.place.len() > 256
            || a.tags.len() > 64
            || a.tags.iter().any(|t| t.len() > 128)
        {
            bail!("Annotation is too large");
        }
        if !a.date_override.is_empty() {
            chrono::NaiveDate::parse_from_str(&a.date_override, "%Y-%m-%d")
                .context("Use YYYY-MM-DD for a date override")?;
        }
        a.tags = a
            .tags
            .into_iter()
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
            .collect();
        a.tags.sort();
        a.tags.dedup();
        self.call(move |c| {
            let tx=c.transaction()?;
            if tx.execute("UPDATE assets SET favorite=?,caption=?,tags=?,place=?,date_override=?,exclude_memory=? WHERE id=?",params![a.favorite,a.caption,serde_json::to_string(&a.tags)?,a.place,a.date_override,a.exclude_memory,id])?!=1 {bail!("Media no longer exists in this catalog");}
            bump(&tx)?;tx.commit()?;Ok(())
        })
    }
}
