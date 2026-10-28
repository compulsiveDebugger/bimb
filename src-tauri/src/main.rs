#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use bimb_core::{Library, model::*};
use std::io::{Read, Seek, SeekFrom};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

type Response<T> = Result<T, String>;
async fn work<T: Send + 'static>(
    f: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Response<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| "Background task stopped".to_string())?
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn overview(lib: State<'_, Library>) -> Response<Overview> {
    let lib = lib.inner().clone();
    work(move || lib.overview()).await
}
#[tauri::command]
async fn source_issues(
    lib: State<'_, Library>,
    id: String,
    cursor: Option<String>,
) -> Response<SourceIssuePage> {
    let lib = lib.inner().clone();
    work(move || lib.source_issues(&id, cursor)).await
}
#[tauri::command]
async fn query(lib: State<'_, Library>, query: Query) -> Response<Page> {
    let lib = lib.inner().clone();
    work(move || lib.query(query)).await
}
#[tauri::command]
async fn get_asset(lib: State<'_, Library>, id: String) -> Response<Asset> {
    let lib = lib.inner().clone();
    work(move || lib.asset(&id)).await
}
#[tauri::command]
async fn add_source(app: tauri::AppHandle, lib: State<'_, Library>) -> Response<Option<String>> {
    let lib = lib.inner().clone();
    work(move || {
        let Some(path) = app
            .dialog()
            .file()
            .set_title("Add a media folder or drive")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        Ok(Some(lib.add_source(path.into_path().map_err(|_| {
            anyhow::anyhow!("Choose a local folder")
        })?)?))
    })
    .await
}
#[tauri::command]
async fn scan_control(lib: State<'_, Library>, id: String, action: String) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || lib.control_scan(&id, &action)).await
}
#[tauri::command]
async fn refresh_sources(lib: State<'_, Library>) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || lib.refresh_sources()).await
}
#[tauri::command]
async fn annotate(lib: State<'_, Library>, id: String, annotation: Annotation) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || lib.annotate(id, annotation)).await
}
#[tauri::command]
async fn album(
    lib: State<'_, Library>,
    action: String,
    id: String,
    name: String,
    asset_ids: Vec<String>,
) -> Response<String> {
    let lib = lib.inner().clone();
    work(move || lib.album(action, id, name, asset_ids)).await
}
#[tauri::command]
async fn save_collection(
    lib: State<'_, Library>,
    id: String,
    name: String,
    query: Query,
) -> Response<String> {
    let lib = lib.inner().clone();
    work(move || lib.save_collection(id, name, query)).await
}
#[tauri::command]
async fn delete_collection(lib: State<'_, Library>, id: String) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || lib.delete_collection(id)).await
}
#[tauri::command]
async fn update_source(
    lib: State<'_, Library>,
    id: String,
    name: String,
    exclusions: Vec<String>,
) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || lib.update_source(id, name, exclusions)).await
}
#[tauri::command]
async fn relink_source(app: tauri::AppHandle, lib: State<'_, Library>, id: String) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || {
        if let Some(p) = app
            .dialog()
            .file()
            .set_title("Locate the original source folder")
            .blocking_pick_folder()
        {
            lib.relink(
                &id,
                p.into_path()
                    .map_err(|_| anyhow::anyhow!("Choose a local folder"))?,
            )?;
        }
        Ok(())
    })
    .await
}
#[tauri::command]
async fn remove_source(lib: State<'_, Library>, id: String) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || lib.remove_source(id)).await
}
#[tauri::command]
async fn settings(lib: State<'_, Library>, settings: Settings) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || lib.save_settings(settings)).await
}
#[tauri::command]
async fn clear_thumbnails(lib: State<'_, Library>, id: String) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || lib.clear_thumbnails(id)).await
}
#[tauri::command]
async fn backup(app: tauri::AppHandle, lib: State<'_, Library>) -> Response<bool> {
    let lib = lib.inner().clone();
    work(move || {
        if let Some(p) = app
            .dialog()
            .file()
            .set_title("Back up Bimb's catalog (originals not included)")
            .set_file_name("Bimb-catalog.sqlite")
            .blocking_save_file()
        {
            lib.backup(
                &p.into_path()
                    .map_err(|_| anyhow::anyhow!("Choose a local file"))?,
            )?;
            return Ok(true);
        }
        Ok(false)
    })
    .await
}
#[tauri::command]
async fn restore(app: tauri::AppHandle, lib: State<'_, Library>) -> Response<bool> {
    let lib = lib.inner().clone();
    work(move || {
        if let Some(p) = app
            .dialog()
            .file()
            .set_title("Restore a Bimb catalog backup")
            .add_filter("Bimb catalog", &["sqlite"])
            .blocking_pick_file()
        {
            lib.restore(
                &p.into_path()
                    .map_err(|_| anyhow::anyhow!("Choose a local file"))?,
            )?;
            return Ok(true);
        }
        Ok(false)
    })
    .await
}
#[tauri::command]
async fn export_annotations(app: tauri::AppHandle, lib: State<'_, Library>) -> Response<bool> {
    let lib = lib.inner().clone();
    work(move || {
        if let Some(p) = app
            .dialog()
            .file()
            .set_title("Export Bimb organization")
            .set_file_name("Bimb-annotations.json")
            .blocking_save_file()
        {
            lib.export_annotations(
                &p.into_path()
                    .map_err(|_| anyhow::anyhow!("Choose a local file"))?,
            )?;
            return Ok(true);
        }
        Ok(false)
    })
    .await
}
#[tauri::command]
async fn open_original(lib: State<'_, Library>, id: String, reveal: bool) -> Response<()> {
    let lib = lib.inner().clone();
    work(move || {
        let path = lib.original(&id)?;
        #[cfg(target_os = "macos")]
        {
            let mut command = std::process::Command::new("/usr/bin/open");
            if reveal {
                command.arg("-R");
            }
            command.arg(&path);
            if !command.status()?.success() {
                anyhow::bail!("Could not open the original");
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (path, reveal);
            anyhow::bail!("Opening originals is not implemented on this platform yet");
        }
        Ok(())
    })
    .await
}

fn media_response(
    lib: &Library,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let build = || -> anyhow::Result<tauri::http::Response<Vec<u8>>> {
        let parts: Vec<_> = request
            .uri()
            .path()
            .trim_start_matches('/')
            .split('/')
            .collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid media URL");
        }
        let mut response = tauri::http::Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .header("X-Content-Type-Options", "nosniff");
        let body = match parts[0] {
            "thumb" => {
                response = response.header("Content-Type", "image/jpeg");
                lib.thumbnail(parts[1])?
            }
            "preview" => {
                response = response.header("Content-Type", "image/jpeg");
                lib.preview(parts[1])?
            }
            "video" => {
                let path = lib.original(parts[1])?;
                let mut file = std::fs::File::open(&path)?;
                let length = file.metadata()?.len();
                let range = request.headers().get("range").and_then(|h| h.to_str().ok());
                let (start, end) = parse_range(range, length)?;
                file.seek(SeekFrom::Start(start))?;
                let mut bytes = vec![0; (end - start + 1) as usize];
                file.read_exact(&mut bytes)?;
                let mime = if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("mov"))
                {
                    "video/quicktime"
                } else {
                    "video/mp4"
                };
                response = response
                    .status(206)
                    .header("Content-Type", mime)
                    .header("Accept-Ranges", "bytes")
                    .header("Content-Range", format!("bytes {start}-{end}/{length}"));
                bytes
            }
            _ => anyhow::bail!("Invalid media operation"),
        };
        Ok(response.header("Content-Length", body.len()).body(body)?)
    };
    build().unwrap_or_else(|_| {
        tauri::http::Response::builder()
            .status(404)
            .header("Access-Control-Allow-Origin", "*")
            .body(Vec::new())
            .unwrap()
    })
}
fn parse_range(header: Option<&str>, length: u64) -> anyhow::Result<(u64, u64)> {
    if length == 0 {
        anyhow::bail!("Empty media");
    }
    let max_chunk = 4 * 1024 * 1024;
    let (start, requested_end) = if let Some(header) = header {
        let value = header
            .strip_prefix("bytes=")
            .ok_or_else(|| anyhow::anyhow!("Invalid range"))?;
        let (a, b) = value
            .split_once('-')
            .ok_or_else(|| anyhow::anyhow!("Invalid range"))?;
        if a.is_empty() {
            let tail = b.parse::<u64>()?;
            if tail == 0 {
                anyhow::bail!("Invalid range");
            }
            (length.saturating_sub(tail), length - 1)
        } else {
            (
                a.parse::<u64>()?,
                if b.is_empty() {
                    length - 1
                } else {
                    b.parse::<u64>()?
                },
            )
        }
    } else {
        (0, length - 1)
    };
    if start >= length || requested_end < start {
        anyhow::bail!("Invalid range");
    }
    Ok((
        start,
        requested_end
            .min(length - 1)
            .min(start.saturating_add(max_chunk - 1)),
    ))
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let home = std::env::var_os("BIMB_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);
            let lib = Library::open(&home)?;
            lib.start_monitor();
            app.manage(lib.clone());
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("bimb", |context, request, responder| {
            let lib = context.app_handle().state::<Library>().inner().clone();
            tauri::async_runtime::spawn_blocking(move || {
                responder.respond(media_response(&lib, request))
            });
        })
        .invoke_handler(tauri::generate_handler![
            overview,
            source_issues,
            query,
            get_asset,
            add_source,
            scan_control,
            refresh_sources,
            annotate,
            album,
            save_collection,
            delete_collection,
            update_source,
            relink_source,
            remove_source,
            settings,
            clear_thumbnails,
            backup,
            restore,
            export_annotations,
            open_original
        ])
        .build(tauri::generate_context!())
        .expect("Bimb could not open its catalog")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                app.state::<Library>().shutdown();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::parse_range;

    #[test]
    fn video_ranges_are_bounded_and_validate_seek_requests() {
        assert_eq!(parse_range(None, 10_000_000).unwrap(), (0, 4_194_303));
        assert_eq!(parse_range(Some("bytes=50-"), 100).unwrap(), (50, 99));
        assert_eq!(parse_range(Some("bytes=-10"), 100).unwrap(), (90, 99));
        assert_eq!(parse_range(Some("bytes=90-200"), 100).unwrap(), (90, 99));
        for value in [
            "bytes=100-",
            "bytes=50-20",
            "bytes=-0",
            "bytes=0-1,3-4",
            "items=0-1",
        ] {
            assert!(parse_range(Some(value), 100).is_err());
        }
        assert!(parse_range(None, 0).is_err());
    }
}
