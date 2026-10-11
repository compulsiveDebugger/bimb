use crate::{Library, catalog, model::Metadata, platform};
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};
use std::{
    path::Path,
    sync::atomic::{AtomicU8, Ordering},
};

fn active(flag: &AtomicU8) -> bool {
    // A pause checkpoints by ending this pass; resuming safely rewalks discoveries.
    // Releasing the worker lets other sources progress while this one is paused.
    flag.load(Ordering::SeqCst) == 0
}
fn kind(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if [
        "jpg", "jpeg", "png", "heic", "heif", "webp", "tif", "tiff", "dng", "arw", "cr2", "cr3",
        "nef", "gif",
    ]
    .contains(&ext.as_str())
    {
        Some("photo")
    } else if ["mp4", "mov", "m4v", "3gp", "avi", "mkv"].contains(&ext.as_str()) {
        Some("video")
    } else {
        None
    }
}
pub fn normalized_date(raw: Option<&str>) -> Option<String> {
    let raw = raw?;
    if let Ok(date) = chrono::NaiveDateTime::parse_from_str(raw, "%Y:%m:%d %H:%M:%S") {
        return Some(date.format("%Y-%m-%dT%H:%M:%S").to_string());
    }
    if let Ok(date) = DateTime::parse_from_rfc3339(raw) {
        return Some(date.format("%Y-%m-%dT%H:%M:%S").to_string());
    }
    if let Ok(date) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
        return Some(date.format("%Y-%m-%dT%H:%M:%S").to_string());
    }
    None
}

pub fn run(lib: &Library, id: &str, flag: &AtomicU8) -> Result<()> {
    let _gate = lib.0.scan_gate.lock().unwrap();
    if !active(flag) {
        return Ok(());
    }
    let mut source = lib.catalog().source(id)?;
    let (volume, key) = platform::source_identity(&lib.0.worker, &source.root)?;
    if volume != source.volume || key != source.root_key {
        bail!("source_changed");
    }
    let device = platform::device(&std::fs::metadata(&source.root)?);
    let source_id = id.to_owned();
    source.generation=lib.catalog().call(move|c|{
        let tx=c.transaction()?;
        tx.execute("UPDATE sources SET generation=generation+1,scan_state='scanning',state='online' WHERE id=?",[&source_id])?;
        tx.execute("DELETE FROM issues WHERE source_id=?",[&source_id])?;
        let generation=tx.query_row("SELECT generation FROM sources WHERE id=?",[source_id],|r|r.get(0))?;tx.commit()?;Ok(generation)
    })?;
    let mut complete = true;
    let exclusions = source.exclusions.clone();
    let root = source.root.clone();
    let walker = walkdir::WalkDir::new(&source.root)
        .follow_links(false)
        .max_open(16)
        .into_iter()
        .filter_entry(move |entry| {
            if entry.depth() == 0 {
                return true;
            }
            let relative = entry.path().strip_prefix(&root).unwrap_or(entry.path());
            let excluded = exclusions.iter().any(|p| relative.starts_with(p));
            let name = entry.file_name().to_string_lossy();
            !excluded
                && !name.starts_with('.')
                && !name.ends_with(".photoslibrary")
                && !name.ends_with(".app")
                && entry
                    .metadata()
                    .is_ok_and(|m| platform::device(&m) == device)
        });
    for entry in walker {
        if !active(flag) {
            return Ok(());
        }
        let current = std::fs::metadata(&source.root)?;
        if platform::file_key(&current) != source.root_key || platform::device(&current) != device {
            bail!("source_disconnected");
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                complete = false;
                issue(lib, id, "folder_unreadable")?;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(media_kind) = kind(entry.path()) else {
            continue;
        };
        let info = match std::fs::symlink_metadata(entry.path()) {
            Ok(m) => m,
            Err(_) => {
                complete = false;
                continue;
            }
        };
        let relative = entry.path().strip_prefix(&source.root)?.to_owned();
        let revision = platform::revision(&info);
        let file_key = platform::file_key(&info);
        let modified: DateTime<Utc> = info.modified()?.into();
        let added = Utc::now().to_rfc3339();
        let source_id = id.to_owned();
        let path = platform::path_bytes(&relative);
        let filename = relative
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let folder = relative
            .parent()
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .into_owned();
        let generation = source.generation;
        let rev = revision.clone();
        let size = info.len().min(i64::MAX as u64) as i64;
        let (asset_id,needs_work,changed)=lib.catalog().call(move|c|{
            let tx=c.transaction()?;
            let existing:Option<(String,String,String,String,String,i64)>=tx.query_row("SELECT id,revision,file_key,metadata_state,thumbnail_state,attempts FROM assets WHERE source_id=? AND path=?",params![source_id,path],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
            let mut existing=existing;
            // A same-path replacement with a different file identity keeps the old record's annotations separate.
            if let Some((old,_,old_key,_,_,_))=&existing&& old_key!=&file_key {
                let tombstone=format!(".bimb-replaced/{old}").into_bytes();
                tx.execute("UPDATE assets SET path=?,state='replaced' WHERE id=?",params![tombstone,old])?;existing=None;
            }
            // Preserve identity on a uniquely established rename, but never merge simultaneous hard links.
            if existing.is_none()&&!file_key.is_empty(){
                let candidates=tx.prepare("SELECT id,revision,path,metadata_state,thumbnail_state,attempts FROM assets WHERE source_id=? AND file_key=? AND state<>'replaced'")?.query_map(params![source_id,file_key],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Vec<u8>>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
                if candidates.len()==1 {let (old,old_rev,old_path,ms,ts,attempts)=&candidates[0];let root=catalog::source(&tx,&source_id)?.root;
                    if !root.join(platform::from_bytes(old_path.clone())).exists()&&old_rev==&rev {existing=Some((old.clone(),old_rev.clone(),file_key.clone(),ms.clone(),ts.clone(),*attempts));tx.execute("UPDATE assets SET path=? WHERE id=?",params![path,old])?;}
                }
            }
            let asset_id=existing.as_ref().map(|v|v.0.clone()).unwrap_or_else(||uuid::Uuid::new_v4().to_string());
            let changed=existing.as_ref().is_none_or(|v|v.1!=rev);
            let needs=changed||existing.as_ref().is_some_and(|v|v.3!="ready"||v.4!="ready");
            tx.execute("INSERT INTO assets(id,source_id,path,filename,folder,file_key,revision,kind,size,modified,added,seen) VALUES(?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET filename=excluded.filename,folder=excluded.folder,file_key=excluded.file_key,revision=excluded.revision,kind=excluded.kind,size=excluded.size,modified=excluded.modified,seen=excluded.seen,state='present'",params![asset_id,source_id,path,filename,folder,file_key,rev,media_kind,size,modified.format("%Y-%m-%dT%H:%M:%S").to_string(),added,generation])?;
            if changed {tx.execute("UPDATE assets SET metadata='{}',captured=NULL,metadata_state='queued',thumbnail_state='queued',thumbnail_bytes=0,error=NULL,attempts=0 WHERE id=?",[&asset_id])?;catalog::bump(&tx)?;}
            tx.commit()?;Ok((asset_id,needs,changed))
        })?;
        if changed {
            let _ = std::fs::remove_file(
                lib.0
                    .home
                    .join("thumbnails")
                    .join(format!("{asset_id}.jpg")),
            );
        }
        if !needs_work {
            continue;
        }
        if platform::is_placeholder(&info) {
            set_error(lib, &asset_id, "not_downloaded")?;
            continue;
        }
        if !active(flag) {
            return Ok(());
        }
        let (used, budget) = lib.catalog().call(|c| {
            let used: i64 = c.query_row(
                "SELECT coalesce(sum(thumbnail_bytes),0) FROM assets",
                [],
                |r| r.get(0),
            )?;
            Ok((used as u64, catalog::settings(c)?.thumbnail_budget))
        })?;
        let can_thumbnail = used < budget;
        let temporary = lib.0.home.join("temporary").join(format!("{asset_id}.jpg"));
        let safe_path = match platform::checked_media(&source.root, &relative) {
            Ok(p) => p,
            Err(_) => {
                set_error(lib, &asset_id, "file_unavailable")?;
                complete = false;
                continue;
            }
        };
        let result = platform::worker(
            &lib.0.worker,
            "probe",
            &safe_path,
            if can_thumbnail {
                Some(&temporary)
            } else {
                None
            },
            512,
        );
        let mut meta: Metadata = match result {
            Ok(v) => serde_json::from_value(v).unwrap_or_default(),
            Err(_) => Metadata {
                metadata_state: "failed".into(),
                thumbnail_state: "failed".into(),
                error: Some("media_worker_failed_or_timed_out".into()),
                ..Default::default()
            },
        };
        if !active(flag) {
            let _ = std::fs::remove_file(&temporary);
            return Ok(());
        }
        let stable = std::fs::symlink_metadata(entry.path())
            .is_ok_and(|m| platform::revision(&m) == revision)
            && std::fs::metadata(&source.root).is_ok_and(|m| {
                platform::file_key(&m) == source.root_key && platform::device(&m) == device
            });
        if !stable {
            let _ = std::fs::remove_file(&temporary);
            set_error(lib, &asset_id, "file_changed_during_read")?;
            complete = false;
            continue;
        }
        let mut thumbnail_bytes = 0;
        if !can_thumbnail {
            meta.thumbnail_state = "budget_full".into();
        }
        if meta.thumbnail_state == "ready" {
            thumbnail_bytes = std::fs::metadata(&temporary)?.len();
            if used.saturating_add(thumbnail_bytes) > budget {
                meta.thumbnail_state = "budget_full".into();
                thumbnail_bytes = 0;
                let _ = std::fs::remove_file(&temporary);
            } else {
                std::fs::rename(
                    &temporary,
                    lib.0
                        .home
                        .join("thumbnails")
                        .join(format!("{asset_id}.jpg")),
                )?;
            }
        } else {
            let _ = std::fs::remove_file(&temporary);
        }
        let captured = normalized_date(meta.captured.as_deref());
        let revision = revision.clone();
        lib.catalog().call(move|c|{
            let tx=c.transaction()?;
            tx.execute("UPDATE assets SET metadata=?,captured=?,metadata_state=?,thumbnail_state=?,thumbnail_bytes=?,error=?,attempts=attempts+1 WHERE id=? AND revision=?",params![serde_json::to_string(&meta)?,captured,meta.metadata_state,meta.thumbnail_state,thumbnail_bytes as i64,meta.error,asset_id,revision])?;
            catalog::bump(&tx)?;tx.commit()?;Ok(())
        })?;
    }
    if !active(flag) {
        return Ok(());
    }
    let (volume, key) = platform::source_identity(&lib.0.worker, &source.root)?;
    if volume != source.volume || key != source.root_key {
        bail!("source_changed");
    }
    if complete {
        let id = id.to_owned();
        let generation = source.generation;
        let unseen = lib.catalog().call(move |c| {
            Ok(c.prepare(
                "SELECT id,path FROM assets WHERE source_id=? AND seen<>? AND state='present'",
            )?
            .query_map(params![id, generation], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
        })?;
        let mut missing = Vec::new();
        for (asset_id, path) in unseen {
            let relative = platform::from_bytes(path);
            if source.exclusions.iter().any(|p| relative.starts_with(p)) {
                continue;
            }
            // A skipped nested mount is not authoritative scan coverage.
            if relative.ancestors().skip(1).any(|p| {
                std::fs::metadata(source.root.join(p)).is_ok_and(|m| platform::device(&m) != device)
            }) {
                continue;
            }
            if matches!(std::fs::symlink_metadata(source.root.join(relative)),Err(e) if e.kind()==std::io::ErrorKind::NotFound)
            {
                missing.push(asset_id);
            }
        }
        // A disappearing root must not turn the candidate list into missing files.
        let current = std::fs::metadata(&source.root)?;
        if platform::file_key(&current) != source.root_key
            || platform::device(&current) != device
            || !active(flag)
        {
            bail!("source_changed");
        }
        lib.catalog().call(move |c| {
            let tx = c.transaction()?;
            for id in missing {
                tx.execute("UPDATE assets SET state='missing' WHERE id=?", [id])?;
            }
            catalog::bump(&tx)?;
            tx.commit()?;
            Ok(())
        })?;
    }
    let id = id.to_owned();
    lib.catalog().call(move |c| {
        c.execute(
            "UPDATE sources SET scan_state=?,last_scan=? WHERE id=?",
            params![
                if complete { "complete" } else { "partial" },
                Utc::now().to_rfc3339(),
                id
            ],
        )?;
        catalog::bump(c)?;
        Ok(())
    })
}
fn issue(lib: &Library, id: &str, code: &str) -> Result<()> {
    let id = id.to_owned();
    let code = code.to_owned();
    lib.catalog().call(move |c| {
        c.execute(
            "INSERT INTO issues(source_id,code) VALUES(?,?)",
            params![id, code],
        )?;
        Ok(())
    })
}
fn set_error(lib: &Library, id: &str, code: &str) -> Result<()> {
    let id = id.to_owned();
    let code = code.to_owned();
    lib.catalog().call(move |c| {
        c.execute(
            "UPDATE assets SET error=?,metadata_state='failed',thumbnail_state='failed' WHERE id=?",
            params![code, id],
        )?;
        catalog::bump(c)?;
        Ok(())
    })
}
