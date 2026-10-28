# Bimb

An offline personal photo and video library. Index folders where they already live, organize in a local catalog, and browse cached thumbnails when a drive is disconnected. Originals are never intentionally modified.

**Status: 0.1.0 development preview.** The macOS 26 Apple Silicon build has native validation. A Windows x64 adapter and build path are implemented, with Windows runtime validation still pending. It is not a signed/notarized public release.

## Run

Open `target/release/bundle/macos/Bimb.app` after building, then choose **Add folder**. Start with a small exported phone-media folder. Bimb does not read an Apple Photos library directly.

The normal catalog lives in `~/Library/Application Support/app.bimb.desktop` on macOS and `%APPDATA%\app.bimb.desktop` on Windows. It contains SQLite data, cached thumbnails, and platform-specific helper data. There is no account, telemetry, remote UI, or cloud service. Catalog data and thumbnails rely on OS account/disk protection and are not independently encrypted.

## Drives and operating systems

Choose a folder on an internal drive, external USB disk/SSD, or memory card. Sources are read only from Bimb's perspective; the catalog and thumbnails stay on the computer. The OS must mount and grant read access to the source. Bimb does not format drives, install filesystem drivers, unlock encrypted disks, or require writable originals.

| Filesystem | macOS | Windows |
|---|---|---|
| APFS / HFS+ | Through the native OS driver | Requires a compatible installed filesystem driver |
| NTFS | Readable mounted volumes, including read-only mounts | Through the native OS driver |
| FAT12 / FAT16 / FAT32 / exFAT | Readable mounted volumes | Readable mounted volumes |
| Other formats | Requires an OS driver, permissions and stable volume/folder identity | Requires an OS driver, permissions and stable volume/folder identity |

This is an implementation compatibility matrix, not certification of every filesystem, device or driver. FAT/exFAT image and physical NTFS-drive validation remain open locally. Source cards show the reported format, drive type and read-only status. Windows reports fixed/removable device classes; an external USB SSD can be classified as fixed.

Reconnect uses the original volume and folder identity, including macOS mount-name changes and Windows drive-letter/mounted-folder changes. Multiple matches require explicit relinking. File IDs on FAT-family filesystems can change during renames or directory reorganization: Bimb preserves the older catalog entry and annotations rather than guessing which file inherited them. Copies and replacement drives are separate sources. Use separate catalogs on macOS and Windows; cross-OS catalog path remapping is not implemented, and incompatible restores are rejected before replacing the active catalog.

Filesystem support is separate from codec support. macOS uses ImageIO/AVFoundation; Windows uses Windows Imaging Component/WinRT and installed OS codecs. Windows JPEG/PNG metadata, oriented sRGB previews, video properties and video thumbnails have an adapter; HEIC/HEVC and other optional formats depend on installed codecs. In-app playback additionally depends on the webview; **Open in another app** and **Reveal original** use the native OS shell. Unsupported files keep catalog entries with media errors.

## Development

Requires Rust 1.99 or later and Node 22.12 or later. macOS builds need macOS 26 and Xcode command-line tools including Swift. Windows x64 builds need Visual Studio C++ build tools, the Windows SDK and WebView2; the NSIS installer embeds the offline WebView2 installer. Dependency installation and installer preparation require internet; the built application does not require a development server.

```sh
npm ci
npm run desktop
npm run desktop:build
```

The desktop script runs on macOS and Windows, chooses an app bundle or NSIS installer respectively, and uses the optional project-local Rust installation in `.tools` when present. Set `BIMB_DATA_DIR` to an absolute directory to isolate a test catalog from your personal library. On Windows, run core checks with `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.

```sh
npm run check
sh scripts/cargo.sh test --workspace --offline
sh scripts/cargo.sh clippy --workspace --all-targets --offline -- -D warnings
sh scripts/cargo.sh run -p bimb-core --example catalog_benchmark --offline
```

Native media tests need access to macOS ImageIO/AVFoundation services. A restricted runner can block video/HEIC even when the desktop application supports them. The fixture generator is `scripts/make-fixtures.swift`; committed fixtures contain synthetic imagery only.

`python3 scripts/test-drive-images.py` exercises new temporary FAT32/exFAT images on macOS, both writable and read-only. It requires DiskImages services and never selects physical disks. The compatibility workflow runs native core/desktop checks on macOS and Windows and runs this image test on macOS.

## Included

- Multiple sources, incremental rescans, pause/cancel/resume, filesystem notifications and periodic reconciliation. Source scan reports show affected folders/files and error details separately from media errors.
- Volume-aware reconnect/relink, original revision checks, conservative missing-file detection, and source exclusions.
- JPEG/PNG/HEIC previews, H.264/HEVC test-video playback, independent metadata/thumbnail failure states.
- Virtualized library and timeline; text search, filters, stable sorting, favorites, tags, captions, named places, manual albums, saved dynamic collections, and on-this-day memories.
- Offline metadata and thumbnails; 512-pixel thumbnail limit and configurable 1 GiB default budget.
- Catalog backup/restore, automatic safety backups before catalog removal/restore, and JSON annotation export. These are **not backups of originals**. Catalog backups exclude thumbnails, which can be regenerated when sources are connected.

Use **Save as collection** to name the current search, filters and sort order. Open it from **Saved collections** to see current matches, including newly indexed or annotated media. Change filters and choose **Edit collection** to save an update; **Reset to saved** discards unsaved filter changes. Deleting a collection preserves its media and annotations. Collections are included in catalog backups and JSON organization exports.

Catalogs now migrate automatically to schema 4, preserving organization and scan issues while adding drive details and native path provenance. Same-OS backups from schemas 1–4 can be restored; schemas 1–3 are macOS catalogs. Upgraded catalogs require this build or newer.

## Structure and status

- `crates/bimb-core`: Rust catalog, identity, indexing, media coordination, and watcher.
- `crates/bimb-core/native`: narrow Swift ImageIO/AVFoundation adapter; `src/platform/windows`: Windows drive, shell and media adapters.
- `src-tauri`: desktop commands and bounded local media protocol.
- `src`: Svelte/TypeScript interface.
- [Product decisions](docs/product.md), [architecture](docs/architecture.md), [reference-product research](docs/research.md), and [validation and remaining release gates](docs/validation.md).

Real phone HDR/audio/variable-frame-rate variants, the full 1 TB workload, physical external-drive hot unplug, disk-full/crash fault injection, and other operating systems still need validation. Face recognition, semantic search, maps, Live Photo grouping, and original-file editing are outside this phase.
