use crate::Library;
use notify::{RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

pub fn start(library: &Library) {
    let weak = std::sync::Arc::downgrade(&library.0);
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::sync_channel(128);
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                // Watcher overflow is repaired by the periodic reconciliation below.
                if let Ok(event) = event
                    && !matches!(event.kind, notify::EventKind::Access(_))
                {
                    let _ = tx.try_send(event.paths);
                }
            })
            .ok();
        let mut watched: HashMap<String, PathBuf> = HashMap::new();
        let mut pending: HashSet<String> = HashSet::new();
        let mut last_event = Instant::now();
        let mut refresh = Instant::now() - Duration::from_secs(30);
        let mut reconcile = Instant::now();
        loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(paths) => {
                    for (id, root) in &watched {
                        if paths.iter().any(|p| p.starts_with(root)) {
                            pending.insert(id.clone());
                        }
                    }
                    last_event = Instant::now();
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    std::thread::sleep(Duration::from_secs(1))
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            let Some(inner) = weak.upgrade() else {
                break;
            };
            let lib = Library(inner);
            if refresh.elapsed() >= Duration::from_secs(30) {
                let _ = lib.refresh_sources();
                if let Ok(info) = lib.overview() {
                    let live: HashSet<_> = info.sources.iter().map(|s| s.id.clone()).collect();
                    watched.retain(|id, path| {
                        if !live.contains(id) {
                            if let Some(w) = &mut watcher {
                                let _ = w.unwatch(path);
                            }
                            false
                        } else {
                            true
                        }
                    });
                    for source in info.sources {
                        if source.state != "online" {
                            if let Some(old) = watched.remove(&source.id)
                                && let Some(w) = &mut watcher
                            {
                                let _ = w.unwatch(&old);
                            }
                            continue;
                        }
                        // Use the lossless path from the catalog, never its display string.
                        if let Ok(record) = lib.catalog().source(&source.id) {
                            if watched.get(&source.id) != Some(&record.root)
                                && source.state == "online"
                            {
                                if let Some(old) = watched.remove(&source.id)
                                    && let Some(w) = &mut watcher
                                {
                                    let _ = w.unwatch(&old);
                                }
                                if let Some(w) = &mut watcher
                                    && w.watch(&record.root, RecursiveMode::Recursive).is_ok()
                                {
                                    // Reconcile changes made before notification registration,
                                    // including media added while a source was disconnected.
                                    pending.insert(source.id.clone());
                                    watched.insert(source.id.clone(), record.root);
                                }
                            }
                            if reconcile.elapsed() >= Duration::from_secs(900)
                                && source.state == "online"
                                && matches!(
                                    source.scan_state.as_str(),
                                    "idle" | "complete" | "partial"
                                )
                            {
                                pending.insert(source.id);
                            }
                        }
                    }
                }
                if reconcile.elapsed() >= Duration::from_secs(900) {
                    reconcile = Instant::now();
                }
                refresh = Instant::now();
            }
            if last_event.elapsed() >= Duration::from_secs(2) {
                for id in pending.clone() {
                    if let Ok(info) = lib.overview() {
                        if let Some(s) = info.sources.iter().find(|s| s.id == id) {
                            if matches!(s.scan_state.as_str(), "paused" | "cancelled") {
                                pending.remove(&id);
                                continue;
                            }
                            if s.state == "online" && lib.require_idle(&id).is_ok() {
                                let _ = lib.start_scan(&id);
                                pending.remove(&id);
                            }
                        } else {
                            pending.remove(&id);
                        }
                    }
                }
            }
        }
    });
}
