pub mod catalog;
pub mod model;
pub mod platform;
mod scan;
mod watch;

use anyhow::{Context, Result, bail};
use catalog::{Catalog, bump};
use fs2::FileExt;
use model::*;
use rusqlite::params;
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

#[derive(Clone)]
pub struct Library(pub(crate) Arc<Inner>);
pub(crate) struct Inner {
    pub catalog: Catalog,
    pub home: PathBuf,
    pub worker: PathBuf,
    pub scans: Mutex<HashMap<String, Arc<AtomicU8>>>,
    pub scan_gate: Mutex<()>,
    pub preview_gate: Mutex<()>,
    pub maintenance: Mutex<()>,
    pub _lock: File,
}

impl Library {
    pub fn open(home: &Path) -> Result<Self> {
        platform::private_dir(home)?;
        let home = home.canonicalize()?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(home.join("catalog.lock"))?;
        lock.try_lock_exclusive()
            .context("This library is already open in another Bimb instance")?;
        platform::private_dir(&home.join("thumbnails"))?;
        platform::private_dir(&home.join("temporary"))?;
        // No workers exist yet. Temporary output from an interrupted run is disposable.
        for e in std::fs::read_dir(home.join("temporary"))?.flatten() {
            if e.file_type().is_ok_and(|t| t.is_file()) {
                let _ = std::fs::remove_file(e.path());
            }
        }
        let worker = platform::install_worker(&home)?;
        let catalog = Catalog::open(&home.join("catalog.sqlite"))?;
        let result = Self(Arc::new(Inner {
            catalog,
            home,
            worker,
            scans: Mutex::new(HashMap::new()),
            scan_gate: Mutex::new(()),
            preview_gate: Mutex::new(()),
            maintenance: Mutex::new(()),
            _lock: lock,
        }));
        result.repair_thumbnails()?;
        Ok(result)
    }
    pub fn catalog(&self) -> &Catalog {
        &self.0.catalog
    }
    pub fn start_monitor(&self) {
        watch::start(self);
    }
    pub fn overview(&self) -> Result<Overview> {
        self.0.catalog.overview()
    }
    pub fn query(&self, q: Query) -> Result<Page> {
        self.0.catalog.query(q)
    }
    pub fn source_issues(&self, id: &str, cursor: Option<String>) -> Result<SourceIssuePage> {
        self.catalog().source_issues(id, cursor)
    }
    pub fn asset(&self, id: &str) -> Result<Asset> {
        let id = id.to_owned();
        self.catalog().call(move |c| {
            Ok(c.query_row("SELECT * FROM library WHERE id=?", [id], catalog::asset)?)
        })
    }
    pub fn add_source(&self, path: PathBuf) -> Result<String> {
        let _guard = self.0.maintenance.lock().unwrap();
        if std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("Choose a folder rather than a symbolic link");
        }
        let path = path.canonicalize()?;
        if path.starts_with(&self.0.home) || self.0.home.starts_with(&path) {
            bail!("Choose a media folder that does not contain Bimb's catalog");
        }
        if path.extension().is_some_and(|s| s == "photoslibrary") {
            bail!("Export files from Photos before adding them");
        }
        let (volume, root_key) = platform::source_identity(&self.0.worker, &path)?;
        let id = uuid::Uuid::new_v4().to_string();
        let out = id.clone();
        self.catalog().call(move|c| {
            let registered:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM sources WHERE volume=? AND root_key=?)",params![volume,root_key],|r|r.get(0))?;
            if registered {bail!("This folder is already registered under another path. Relink that source instead.");}
            let existing=c.prepare("SELECT path FROM sources")?.query_map([],|r|r.get::<_,Vec<u8>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            if existing.len()>=32 {bail!("This preview supports up to 32 source folders");}
            for bytes in existing {let other=platform::from_bytes(bytes);if path.starts_with(&other)||other.starts_with(&path){bail!("This folder overlaps an existing source");}}
            let name=path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().into_owned();
            c.execute("INSERT INTO sources(id,name,path,display_path,volume,root_key) VALUES(?,?,?,?,?,?)",params![id,name,platform::path_bytes(&path),path.to_string_lossy(),volume,root_key])?;bump(c)?;Ok(())
        })?;
        drop(_guard);
        self.start_scan(&out)?;
        Ok(out)
    }
    pub fn relink(&self, id: &str, path: PathBuf) -> Result<()> {
        let _guard = self.0.maintenance.lock().unwrap();
        self.require_idle(id)?;
        let path = path.canonicalize()?;
        let old = self.catalog().source(id)?;
        let (volume, key) = platform::source_identity(&self.0.worker, &path)?;
        if old.volume != volume || old.root_key != key {
            bail!(
                "This is not the registered folder on its original volume. Choose that folder; copied libraries need a separate source."
            );
        }
        let id = id.to_owned();
        self.catalog().call(move |c| {
            c.execute(
                "UPDATE sources SET path=?,display_path=?,state='online' WHERE id=?",
                params![platform::path_bytes(&path), path.to_string_lossy(), id],
            )?;
            bump(c)?;
            Ok(())
        })
    }
    pub fn update_source(&self, id: String, name: String, exclusions: Vec<String>) -> Result<()> {
        let _guard = self.0.maintenance.lock().unwrap();
        self.require_idle(&id)?;
        if name.trim().is_empty() || name.len() > 128 || exclusions.len() > 100 {
            bail!("Invalid source settings");
        }
        for e in &exclusions {
            if e.is_empty()
                || Path::new(e).is_absolute()
                || Path::new(e)
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                bail!("Exclusions must be relative folder names");
            }
        }
        self.catalog().call(move |c| {
            c.execute(
                "UPDATE sources SET name=?,exclusions=? WHERE id=?",
                params![name.trim(), serde_json::to_string(&exclusions)?, id],
            )?;
            bump(c)?;
            Ok(())
        })
    }
    pub fn refresh_sources(&self) -> Result<()> {
        let records = self.catalog().call(|c| {
            let ids = c
                .prepare("SELECT id FROM sources")?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids.into_iter()
                .map(|id| catalog::source(c, &id))
                .collect::<Result<Vec<_>>>()
        })?;
        for mut source in records {
            let mut relinked = false;
            if !source.root.exists()
                && self.require_idle(&source.id).is_ok()
                && let Ok(relative) = source.root.strip_prefix("/Volumes")
            {
                let suffix: PathBuf = relative.components().skip(1).collect();
                let mut matches = Vec::new();
                if let Ok(volumes) = std::fs::read_dir("/Volumes") {
                    for volume in volumes.flatten().take(32) {
                        let candidate = volume.path().join(&suffix);
                        if std::fs::metadata(&candidate)
                            .is_ok_and(|m| platform::file_key(&m) == source.root_key)
                            && platform::source_identity(&self.0.worker, &candidate)
                                .is_ok_and(|(v, k)| v == source.volume && k == source.root_key)
                        {
                            matches.push(candidate);
                        }
                    }
                }
                if matches.len() == 1 && self.relink(&source.id, matches.remove(0)).is_ok() {
                    source = self.catalog().source(&source.id)?;
                    relinked = true;
                }
            }
            let state = match platform::source_identity(&self.0.worker, &source.root) {
                Ok((volume, key)) if volume == source.volume && key == source.root_key => "online",
                Ok(_) => "ambiguous",
                Err(_) => {
                    if source.root.exists() {
                        "unavailable"
                    } else {
                        "offline"
                    }
                }
            };
            let id = source.id.clone();
            let state = state.to_owned();
            let (changed, resume) = self.catalog().call(move |c| {
                let old: (String, String) = c.query_row(
                    "SELECT state,scan_state FROM sources WHERE id=?",
                    [&id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                let changed = old.0 != state;
                if changed {
                    c.execute("UPDATE sources SET state=? WHERE id=?", params![state, id])?;
                    bump(c)?;
                }
                Ok((
                    changed,
                    state == "online" && (relinked || old.0 != "online" || old.1 == "interrupted"),
                ))
            })?;
            let _ = changed;
            if resume {
                let _ = self.start_scan(&source.id);
            }
        }
        Ok(())
    }
    pub fn start_scan(&self, id: &str) -> Result<()> {
        let _guard = self.0.maintenance.lock().unwrap();
        let mut scans = self.0.scans.lock().unwrap();
        if let Some(flag) = scans.get(id) {
            if flag.load(Ordering::SeqCst) != 0 {
                bail!("Scan is finishing its current operation. Try resuming in a moment.");
            }
            return Ok(());
        }
        self.catalog().source(id)?;
        let key = id.to_owned();
        let flag = Arc::new(AtomicU8::new(0));
        scans.insert(key.clone(), flag.clone());
        let id = key.clone();
        self.catalog().call(move |c| {
            c.execute("UPDATE sources SET scan_state='queued' WHERE id=?", [id])?;
            Ok(())
        })?;
        let library = self.clone();
        std::thread::Builder::new().name("bimb-scan".into()).spawn(move|| {
            let result=scan::run(&library,&key,&flag);
            if let Err(error)=result && flag.load(Ordering::SeqCst)==0 {
                let id=key.clone();
                let message=format!("{error:#}");
                let _=library.catalog().call(move|c|{
                    let tx=c.transaction()?;
                    tx.execute("UPDATE sources SET scan_state='interrupted' WHERE id=?",[&id])?;
                    tx.execute("INSERT INTO issues(source_id,code,message) SELECT id,'scan_interrupted',? FROM sources WHERE id=?",params![message,id])?;
                    tx.commit()?;Ok(())
                });
            }
            library.0.scans.lock().unwrap().remove(&key);
        })?;
        Ok(())
    }
    pub fn control_scan(&self, id: &str, action: &str) -> Result<()> {
        match action {
            "resume" => self.start_scan(id),
            "pause" | "cancel" => {
                if let Some(flag) = self.0.scans.lock().unwrap().get(id) {
                    flag.store(if action == "pause" { 1 } else { 2 }, Ordering::SeqCst);
                }
                let id = id.to_owned();
                let state = if action == "pause" {
                    "paused"
                } else {
                    "cancelled"
                };
                self.catalog().call(move |c| {
                    c.execute(
                        "UPDATE sources SET scan_state=? WHERE id=?",
                        params![state, id],
                    )?;
                    Ok(())
                })
            }
            _ => bail!("Unknown scan action"),
        }
    }
    pub fn require_idle(&self, id: &str) -> Result<()> {
        if self.0.scans.lock().unwrap().contains_key(id) {
            bail!("Cancel the scan and wait for it to stop first");
        }
        Ok(())
    }
    pub fn remove_source(&self, id: String) -> Result<()> {
        let _guard = self.0.maintenance.lock().unwrap();
        self.require_idle(&id)?;
        // Automatic safety snapshot before deleting user organization.
        self.backup(&self.0.home.join("before-source-removal.sqlite"))?;
        let ids = self.catalog().call(move |c| {
            let ids = c
                .prepare("SELECT id FROM assets WHERE source_id=?")?
                .query_map([&id], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let tx = c.transaction()?;
            tx.execute("DELETE FROM sources WHERE id=?", [id])?;
            bump(&tx)?;
            tx.commit()?;
            Ok(ids)
        })?;
        for id in ids {
            let _ = std::fs::remove_file(self.0.home.join("thumbnails").join(format!("{id}.jpg")));
        }
        Ok(())
    }
    pub fn annotate(&self, id: String, a: Annotation) -> Result<()> {
        self.catalog().annotate(id, a)
    }
    pub fn album(
        &self,
        action: String,
        id: String,
        name: String,
        asset_ids: Vec<String>,
    ) -> Result<String> {
        if asset_ids.len() > 1000 || name.len() > 128 {
            bail!("Album request is too large");
        }
        self.catalog().call(move|c| {
            let tx=c.transaction()?;let mut result=id.clone();
            match action.as_str() {
                "create"=>{if name.trim().is_empty(){bail!("Name your album");}result=uuid::Uuid::new_v4().to_string();tx.execute("INSERT INTO albums VALUES(?,?)",params![result,name.trim()])?;},
                "rename"=>{if name.trim().is_empty(){bail!("Name your album");}tx.execute("UPDATE albums SET name=? WHERE id=?",params![name.trim(),id])?;},
                "delete"=>{tx.execute("DELETE FROM albums WHERE id=?",[id])?;},
                "add"=>{for asset in asset_ids {tx.execute("INSERT OR IGNORE INTO memberships VALUES(?,?,(SELECT coalesce(max(position),0)+1 FROM memberships WHERE album_id=?))",params![id,asset,id])?;}},
                "remove"=>{for asset in asset_ids {tx.execute("DELETE FROM memberships WHERE album_id=? AND asset_id=?",params![id,asset])?;}},
                _=>bail!("Unknown album action"),
            };bump(&tx)?;tx.commit()?;Ok(result)
        })
    }
    pub fn save_settings(&self, s: Settings) -> Result<()> {
        if s.thumbnail_budget > 100 * 1024 * 1024 * 1024 || s.excluded_dates.len() > 100 {
            bail!("Invalid settings");
        }
        for [from, to] in &s.excluded_dates {
            chrono::NaiveDate::parse_from_str(from, "%Y-%m-%d")?;
            chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d")?;
            if from > to {
                bail!("Date range starts after it ends");
            }
        }
        self.catalog().call(move |c| {
            c.execute(
                "INSERT OR REPLACE INTO meta VALUES('settings',?)",
                [serde_json::to_string(&s)?],
            )?;
            bump(c)?;
            Ok(())
        })
    }
    pub fn clear_thumbnails(&self, source_id: String) -> Result<()> {
        let _guard = self.0.maintenance.lock().unwrap();
        self.require_idle(&source_id)?;
        let ids = self.catalog().call(move |c| {
            let ids = c
                .prepare("SELECT id FROM assets WHERE source_id=?")?
                .query_map([&source_id], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            c.execute(
                "UPDATE assets SET thumbnail_state='cleared',thumbnail_bytes=0 WHERE source_id=?",
                [source_id],
            )?;
            bump(c)?;
            Ok(ids)
        })?;
        for id in ids {
            let _ = std::fs::remove_file(self.0.home.join("thumbnails").join(format!("{id}.jpg")));
        }
        Ok(())
    }
    pub fn backup(&self, destination: &Path) -> Result<()> {
        self.validate_export_path(destination)?;
        let tmp = destination.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let output = tmp.clone();
        self.catalog().call(move |c| {
            c.backup("main", &output, None)?;
            Ok(())
        })?;
        let check = rusqlite::Connection::open_with_flags(
            &tmp,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let integrity: String = check.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        drop(check);
        if integrity != "ok" {
            let _ = std::fs::remove_file(tmp);
            bail!("Backup validation failed");
        }
        File::open(&tmp)?.sync_all()?;
        std::fs::rename(tmp, destination)?;
        Ok(())
    }
    fn validate_export_path(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .context("Choose an export location")?
            .canonicalize()?;
        let target = parent.join(path.file_name().context("Choose a filename")?);
        if target.starts_with(&self.0.home)
            && target != self.0.home.join("before-restore.sqlite")
            && target != self.0.home.join("before-source-removal.sqlite")
        {
            bail!("Save exports outside Bimb's application data folder");
        }
        for s in self.overview()?.sources {
            if target.starts_with(self.catalog().source(&s.id)?.root) {
                bail!("Save the export outside indexed sources");
            }
        }
        Ok(())
    }
    pub fn restore(&self, path: &Path) -> Result<()> {
        let _guard = self.0.maintenance.lock().unwrap();
        if path.canonicalize()? == self.0.home.join("catalog.sqlite").canonicalize()? {
            bail!("Choose a backup, not the active catalog");
        }
        if !self.0.scans.lock().unwrap().is_empty() {
            bail!("Cancel active scans before restoring");
        }
        let check = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let integrity: String = check.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        let version: i64 = check.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if integrity != "ok" || !(1..=catalog::SCHEMA_VERSION).contains(&version) {
            bail!("Not a compatible Bimb catalog backup");
        }
        // Probe required schema before replacing anything.
        check.prepare("SELECT a.id,a.path,s.volume,s.root_key FROM assets a JOIN sources s ON s.id=a.source_id")?;
        drop(check);
        // Stage the input so restoring our own safety backup cannot overwrite it
        // when the current catalog is saved below.
        let staged = self
            .0
            .home
            .join("temporary")
            .join(format!("restore-{}.sqlite", uuid::Uuid::new_v4()));
        std::fs::copy(path, &staged)?;
        let result = (|| {
            // Upgrade and validate the staged backup before replacing the active catalog.
            let mut staged_catalog = rusqlite::Connection::open(&staged)?;
            catalog::migrate(&mut staged_catalog)?;
            staged_catalog.prepare("SELECT id,source_id,code,path,message FROM issues")?;
            drop(staged_catalog);
            self.backup(&self.0.home.join("before-restore.sqlite"))?;
            let path = staged.clone();
            self.catalog().call(move|c|{
            c.restore("main",&path,None::<fn(rusqlite::backup::Progress)>)?;
            c.execute_batch("PRAGMA foreign_keys=ON; UPDATE sources SET state='unavailable',scan_state='idle'; UPDATE assets SET thumbnail_state='queued',thumbnail_bytes=0;")?;bump(c)?;Ok(())
            })
        })();
        let _ = std::fs::remove_file(staged);
        result?;
        self.repair_thumbnails()?;
        Ok(())
    }
    pub fn export_annotations(&self, path: &Path) -> Result<()> {
        self.validate_export_path(path)?;
        let data=self.catalog().call(|c|{
            let assets=c.prepare("SELECT * FROM library ORDER BY id")?.query_map([],catalog::asset)?.collect::<rusqlite::Result<Vec<_>>>()?;
            let memberships=c.prepare("SELECT album_id,asset_id,position FROM memberships")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(serde_json::json!({"format":"bimb-annotations","version":1,"assets":assets,"memberships":memberships}))
        })?;
        let overview = self.overview()?;
        let mut data = data;
        data["sources"] = serde_json::to_value(overview.sources)?;
        data["albums"] = serde_json::to_value(overview.albums)?;
        data["settings"] = serde_json::to_value(overview.settings)?;
        let tmp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, serde_json::to_vec_pretty(&data)?)?;
        File::open(&tmp)?.sync_all()?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
    fn repair_thumbnails(&self) -> Result<()> {
        let dir = self.0.home.join("thumbnails");
        let records = self.catalog().call(|c| {
            Ok(c.prepare("SELECT id,thumbnail_state FROM assets")?
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })?;
        let valid: std::collections::HashSet<_> =
            records.iter().map(|(id, _)| format!("{id}.jpg")).collect();
        for e in std::fs::read_dir(&dir)?.flatten() {
            if !valid.contains(&e.file_name().to_string_lossy().into_owned()) {
                let _ = std::fs::remove_file(e.path());
            }
        }
        for (id, state) in records {
            if state == "ready" && !dir.join(format!("{id}.jpg")).is_file() {
                self.catalog().call(move |c| {
                    c.execute(
                        "UPDATE assets SET thumbnail_state='queued',thumbnail_bytes=0 WHERE id=?",
                        [id],
                    )?;
                    Ok(())
                })?;
            }
        }
        Ok(())
    }
    pub fn thumbnail(&self, id: &str) -> Result<Vec<u8>> {
        uuid::Uuid::parse_str(id)?;
        if self.asset(id)?.thumbnail_state != "ready" {
            bail!("Thumbnail is unavailable");
        }
        Ok(std::fs::read(
            self.0.home.join("thumbnails").join(format!("{id}.jpg")),
        )?)
    }
    pub fn original(&self, id: &str) -> Result<PathBuf> {
        let id = id.to_owned();
        let (source_id, relative, revision) = self.catalog().call(move |c| {
            Ok(c.query_row(
                "SELECT source_id,path,revision FROM assets WHERE id=?",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )?)
        })?;
        let source = self.catalog().source(&source_id)?;
        let (volume, key) = platform::source_identity(&self.0.worker, &source.root)
            .context("Connect the original drive to open this media")?;
        if volume != source.volume || key != source.root_key {
            bail!("Source identity changed; relink the original drive");
        }
        let path = platform::checked_media(&source.root, &platform::from_bytes(relative))?;
        if platform::revision(&std::fs::metadata(&path)?) != revision {
            bail!("This file changed. Rescan its source before opening it");
        }
        Ok(path)
    }
    pub fn preview(&self, id: &str) -> Result<Vec<u8>> {
        let _gate = self.0.preview_gate.lock().unwrap();
        let path = self.original(id)?;
        let tmp = self
            .0
            .home
            .join("temporary")
            .join(format!("{}.jpg", uuid::Uuid::new_v4()));
        let output = platform::worker(&self.0.worker, "probe", &path, Some(&tmp), 2560);
        let bytes = if output.is_ok_and(|v| v["thumbnail_state"] == "ready") {
            std::fs::read(&tmp).context("Preview unavailable")
        } else {
            Err(anyhow::anyhow!("Preview unavailable for this format"))
        };
        let _ = std::fs::remove_file(tmp);
        bytes
    }
    pub fn shutdown(&self) {
        for flag in self.0.scans.lock().unwrap().values() {
            flag.store(2, Ordering::SeqCst);
        }
    }
}
