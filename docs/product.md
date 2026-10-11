# Bimb product outline

Approved for implementation — 10 October 2026. The user accepted the proposal with “Looks good. Proceed.” See `validation.md` for the current preview’s implemented behavior and remaining release gates.

## Product promise

Bimb helps a person browse, find, and rediscover their own photos and videos across folders and removable drives, without depending on the internet. Originals stay where the user put them. A local catalog and small thumbnails remain available when a drive is disconnected.

The first audience is someone with phone-camera media scattered across computer folders and external storage. Professional photography workflows, a photo editor, and multi-device synchronization are later possibilities.

## Confirmed decisions

| Decision | Agreement |
|---|---|
| Storage model | Index files in place; Bimb organization stays in its catalog. |
| Initial library | Approximately 3,000 files and 1 TB, initially phone photos and videos. |
| Disconnected drives | Retain searchable metadata and small thumbnails on the computer. |
| Platform sequence | Apple Silicon on the current device first; broader Windows/macOS support later. |
| Local protection | Rely on the computer's account and disk protection initially; no app password/encrypted catalog requirement. |

The development host reports macOS 26.3.1 (a), arm64. This is the initial validation target, not a promise of support for every macOS release. Phone models and exact codecs have not yet been identified. Propose covering both ordinary Android and iPhone exports and validating actual examples during implementation.

## First-use experience

1. Show “Add a folder or drive” and explain that originals remain untouched.
2. Choose a folder using the OS picker. Register its underlying volume, selected root, and a friendly name. Show exclusions and estimated thumbnail storage as they become available.
3. Begin discovery immediately. Show counts for found, indexed, thumbnail-ready, unsupported, and failed items. Allow pause, resume, and cancel. Do not invent a percentage before enumeration establishes a denominator.
4. Make indexed results available while the remainder of the scan continues. Prioritize thumbnails currently visible in the grid.
5. On subsequent launches, open the library immediately. Display drive availability separately from background scan progress.

## Approved first-release scope

| Area | User-visible behavior |
|---|---|
| Sources | Multiple folders and removable drives; source names, connection state, rescan, relink, exclusions, remove from library. |
| Browse | Virtualized photo/video grid, timeline, folders, photo viewer, video playback, metadata panel, reveal original in Finder. |
| Search | Filename, folder, tags, captions, camera information, manually named places; combinable filters for capture dates, media type, source, favorites, and availability. |
| Sort | Capture date, date added, filename, file size, and video duration; ascending/descending with stable ties. Unknown values sort last. |
| Organization | Favorites, tags, captions, and manual albums stored only in Bimb. Removing an item from an album does not remove it from the library. |
| Memories | “On this day,” date-range collections, and collections based on named places. Exclude items/date ranges and disable memories. No face or semantic analysis required. |
| Places | Read embedded GPS, display coordinates, filter for GPS availability, and let users name places. Automatic city names and map tiles require separate offline datasets and are deferred. |
| Offline browsing | Keep small thumbnails and all catalog information. Opening full-resolution media or playing video requires an accessible original. Clearly identify cached views. |
| Maintenance | Scan error list with retry, thumbnail storage controls, catalog backup/restore, and portable export of user annotations. |

Search matches recorded information. A query such as “beach” only matches filenames, tags, captions, or place labels containing that information until local ML is added.

Proposed thumbnail policy: one image of up to 512 pixels on its longest edge per photo, or one representative frame per video; 1 GiB configurable budget. At an illustrative average of 50 KiB, 3,000 thumbnails occupy about 147 MiB before filesystem overhead. This is a planning estimate, not a measured result. Actual size appears in Settings. On reaching the budget, preserve existing thumbnails and pause creation with a clear action to increase the budget or clear selected source thumbnails. Do not silently evict disconnected-drive thumbnails.

## Media capability contract

| Media | Initial proposal |
|---|---|
| JPEG and PNG | Metadata, oriented thumbnail, and viewing. |
| HEIC/HEIF | Metadata, primary still image, thumbnail and viewing through the macOS media adapter; validate common phone variants. |
| MP4/MOV with H.264 or HEVC | Metadata, one thumbnail, and playback where supported by the tested platform adapter. Audio, rotation, variable frame rate, and HDR are separate validation cases. |
| Live Photos / Android motion photos | Preserve/index the underlying files; display the still component when identifiable. Unified animated presentation and grouping are deferred. Do not mislabel a companion video as a duplicate. |
| RAW / ProRAW, unusual codecs, corrupt files | Show basic discovered-file information and an explicit unsupported or failed-operation state. Full decode support is outside the initial acceptance scope. |

Container extension alone never guarantees playback. Metadata extraction, thumbnail generation, full viewing, and playback each have an independent support/result state. No background transcoding of 1 TB of originals.

## Privacy and safety proposal

- No Bimb account, mandatory cloud services, telemetry, media upload, recurring online license checks, or background update checks in the initial release. Core functions work indefinitely offline. Manual application updates initially.
- Ship UI assets locally. No remote fonts, analytics, maps, or geocoding calls. Future model/dataset installation must be explicit and also support importing an offline package.
- Source files are opened for reading only. No original deletion, movement, metadata write-back, or sidecar writes in the first release.
- Catalog and thumbnails contain private information, including GPS and images. OS permissions protect them; Bimb does not claim they are independently encrypted or protected from other software running as the same user.
- OS services and externally opened applications have their own network behavior. Bimb's guarantee covers its own application workflow, with a disconnected-machine acceptance check.
- “Remove source” displays the catalog entries/organization and cached media that will be removed. Original files remain untouched. Offer catalog backup before catalog-destructive removal.
- Indexing is not an original-media backup. A catalog backup preserves Bimb organization; it cannot recover original photos or videos from thumbnails.

## Explicitly later

Face grouping/recognition, semantic search, OCR, automatic place naming/maps, duplicate detection, advanced Live Photo grouping, RAW development, original-file management, cloud/device sync, shared catalogs, NAS-specific support, and general plugin execution.

Long-term “all” platform support means a staged compatibility matrix, not an automatic promise to support every historical OS or CPU. Windows x64 and supported Intel Macs are candidates for the next desktop phase; Windows ARM, mobile, and other platforms require separate agreement.

## Approved decisions

The following decisions were accepted together with the product outline. The architectural details are in `architecture.md`.

| ID | Decision | Recommendation |
|---|---|---|
| P1 | First-release scope | The feature and media tables above; memories use dates and named places. |
| P2 | Offline contract | Account-free, indefinitely offline, no telemetry; manual updates. |
| P3 | Destructive operations | No writes/deletions to originals; explicit catalog removal only. |
| P4 | Thumbnail retention | 512-pixel maximum, 1 GiB initial budget, retain existing thumbnails when full. |
| A1 | Desktop shell | Tauri 2 with Svelte/TypeScript UI; Rust owns all catalog and media workflow logic. |
| A2 | Persistence | Local SQLite through rusqlite, FTS5 search, separate thumbnail files, one application writer. |
| A3 | Identity and indexing | Volume-aware sources, stable asset IDs, resumable bounded pipeline, conservative reconciliation. |
| A4 | Initial media backend | macOS Image I/O and AVFoundation behind Rust interfaces; narrow native bridge if required. |
| A5 | Playback | In-app playback using tested local WebView/native capabilities; explicit external-player fallback for unsupported cases. |
| A6 | Recovery | Durable annotations, consistent catalog backups, transactional migrations, versioned derived data. |
| A7 | Extensibility | Internal analyzer interfaces and versioned outputs; no general plugin system or ML dependencies initially. |
| A8 | Platform rollout | Current Apple Silicon Mac first; approve the Windows/Intel compatibility matrix before that phase. |
| V1 | Validation | Targeted correctness/recovery/media tests and a small benchmark suite described in the architecture. |

Tauri 2 with Svelte/TypeScript was accepted for the desktop shell; the catalog and media workflows remain in Rust with a narrow native codec adapter.

## Delivery gates

1. Agree on this product outline and the architecture, amending the decision table as needed. No application scaffolding before this gate.
2. Implement a narrow vertical slice: add a source, persist metadata, generate thumbnails, search, disconnect/reconnect. Validate representative phone media and native integration before expanding the UI. Bring material design changes back for agreement.
3. Complete browsing/playback, organization, sorting, memories, recovery, and settings within the approved scope.
4. Validate the real 3,000-file workload, drive failures, restart behavior, privacy, and packaging on Apple Silicon.
5. Agree on and implement Windows/other desktop targets using the same Rust core and platform adapters.

Commercial license, public release/distribution, signing credentials, and later ML model licenses require decisions before distribution or the relevant later feature, not before drafting the design.
