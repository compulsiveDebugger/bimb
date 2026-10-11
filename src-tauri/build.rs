fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "overview",
            "query",
            "get_asset",
            "add_source",
            "scan_control",
            "refresh_sources",
            "annotate",
            "album",
            "update_source",
            "relink_source",
            "remove_source",
            "settings",
            "clear_thumbnails",
            "backup",
            "restore",
            "export_annotations",
            "open_original",
        ]),
    ))
    .expect("Could not build Tauri manifest");
}
