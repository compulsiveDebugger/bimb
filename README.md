# Bimb

An offline personal photo and video library. Index folders where they already live, organize in a local catalog, and browse cached thumbnails when a drive is disconnected. Originals are never intentionally modified.

**Status: 0.1.0 local Apple Silicon preview.** Product and architecture approved on 10 October 2026. This build targets macOS 26 on the current Mac; broader macOS and Windows support is a later phase. It is not a signed/notarized public release.

## Run

Open `target/release/bundle/macos/Bimb.app` after building, then choose **Add folder**. Start with a small exported phone-media folder. Bimb does not read an Apple Photos library directly.

The normal catalog lives in `~/Library/Application Support/app.bimb.desktop`. It contains SQLite data, cached thumbnails, and a local native media helper. There is no account, telemetry, remote UI, or cloud service. Catalog data and thumbnails rely on macOS account/disk protection and are not independently encrypted.

## Development

Requires Apple Silicon, macOS 26, Xcode command-line tools including Swift, Rust 1.99 or later, and Node 22.12 or later. Dependency installation requires internet; the built application does not require a development server.

```sh
npm ci
npm run desktop
npm run desktop:build
```

The scripts use the optional project-local Rust installation in `.tools` when present and otherwise use Rust from PATH. Set `BIMB_DATA_DIR` to an absolute directory to isolate a test catalog from your personal library.

```sh
npm run check
sh scripts/cargo.sh test --workspace --offline
sh scripts/cargo.sh clippy --workspace --all-targets --offline -- -D warnings
sh scripts/cargo.sh run -p bimb-core --example catalog_benchmark --offline
```

Native media tests need access to macOS ImageIO/AVFoundation services. A restricted runner can block video/HEIC even when the desktop application supports them. The fixture generator is `scripts/make-fixtures.swift`; committed fixtures contain synthetic imagery only.

## Included

- Multiple sources, incremental rescans, pause/cancel/resume, filesystem notifications and periodic reconciliation. Source scan reports show affected folders/files and error details separately from media errors.
- Volume-aware reconnect/relink, original revision checks, conservative missing-file detection, and source exclusions.
- JPEG/PNG/HEIC previews, H.264/HEVC test-video playback, independent metadata/thumbnail failure states.
- Virtualized library and timeline; text search, filters, stable sorting, favorites, tags, captions, named places, manual albums, saved dynamic collections, and on-this-day memories.
- Offline metadata and thumbnails; 512-pixel thumbnail limit and configurable 1 GiB default budget.
- Catalog backup/restore, automatic safety backups before catalog removal/restore, and JSON annotation export. These are **not backups of originals**. Catalog backups exclude thumbnails, which can be regenerated when sources are connected.

Use **Save as collection** to name the current search, filters and sort order. Open it from **Saved collections** to see current matches, including newly indexed or annotated media. Change filters and choose **Edit collection** to save an update; **Reset to saved** discards unsaved filter changes. Deleting a collection preserves its media and annotations. Collections are included in catalog backups and JSON organization exports.

Catalogs now migrate automatically to schema 3, preserving existing organization and scan issues. Backups from schemas 1, 2 and 3 can be restored; upgraded catalogs require this build or newer.

## Structure and status

- `crates/bimb-core`: Rust catalog, identity, indexing, media coordination, and watcher.
- `crates/bimb-core/native`: narrow Swift ImageIO/AVFoundation adapter.
- `src-tauri`: desktop commands and bounded local media protocol.
- `src`: Svelte/TypeScript interface.
- [Product decisions](docs/product.md), [architecture](docs/architecture.md), [reference-product research](docs/research.md), and [validation and remaining release gates](docs/validation.md).

Real phone HDR/audio/variable-frame-rate variants, the full 1 TB workload, physical external-drive hot unplug, disk-full/crash fault injection, and other operating systems still need validation. Face recognition, semantic search, maps, Live Photo grouping, and original-file editing are outside this phase.
