# Troubleshoot Bimb

[Documentation home](README.md) · [Installation](installation.md) · [User guide](user-guide.md)

Before changing files, back up the catalog when Bimb can open it. Keep originals separately backed up. A failed scan, unavailable drive or broken thumbnail is not a reason to delete your catalog.

## macOS blocks the application

Check the package source, CPU and minimum macOS version. The current Mac package targets Apple Silicon and macOS 26+. Use the copied app in Applications, not a copy still inside a disk image.

Preview builds are ad-hoc signed and are not notarized. If you intentionally downloaded a trusted preview and macOS offers the option, review it under **System Settings → Privacy & Security → Open Anyway**, then follow the confirmation. The option appears only after an attempted blocked launch. See [Apple's explanation](https://support.apple.com/en-us/102445).

If macOS reports damage or malware, do not assume it is a harmless preview warning. Verify the download checksum and ask the maintainer for a verified replacement. Do not disable Gatekeeper, strip quarantine attributes or run an unknown bundle from Terminal as an installation workaround. A public release should pass the maintainer's signing/notarization checks.

## Windows blocks the installer or Bimb will not open

Confirm that the PC is x64 and that you extracted the artifact ZIP before running `-setup.exe`. Use the installer rather than moving a standalone executable from a build folder.

Unsigned previews can trigger SmartScreen. Check the source, package hash and publisher information before any allow/run decision. If organizational policy or Smart App Control prevents launch, request an approved signed package or contact the administrator. Do not disable Defender, SmartScreen or Smart App Control to install Bimb. Even a newly signed application can initially receive reputation warnings; see [Microsoft's guidance](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/app-control-for-business/design/smart-app-control).

If the app opens to a blank window, repair/reinstall using the complete setup program so its WebView2 dependency can be installed. The configured package embeds the offline WebView2 installer. Capture the exact error if dependency installation is blocked. Runtime repair does not require deleting Bimb's catalog.

## Bimb asks for drive or folder access

Open the selected folder in Finder/File Explorer and confirm your account can read it. Unlock encrypted volumes and fully download any required cloud files before scanning them. Approve an OS file-access prompt only for folders you intend to index. On macOS, review **Privacy & Security → Files and Folders** if access was previously denied.

Read-only sources can be indexed; Bimb keeps its writable catalog/cache on the computer. The OS must provide a filesystem driver. For example, a Windows PC does not gain APFS support simply by installing Bimb.

## The drive is connected but the source stays offline or ambiguous

1. Confirm the original disk and registered folder are present and readable.
2. Allow about 30 seconds for availability refresh.
3. Cancel an active scan and wait for it to stop, then choose **Relink** on that source and select the original folder.
4. If identity is rejected, check whether this is a copied/replacement drive or a folder whose filesystem identity changed.

Matching names or drive letters do not establish identity. Copied libraries are added as new sources. FAT file IDs can change during renames/reorganization; Bimb keeps previous annotations rather than guessing. Cross-OS catalogs require path remapping that is not implemented in this preview. Do not edit SQLite identity fields to force a match.

## Photos appear twice

Separate copied files are separate assets. Live Photo still/video components also appear separately. Bimb does not automatically deduplicate or delete originals. Overlapping roots are rejected, but independent source copies are still distinct.

## An image or video cannot be shown

Reconnect the original source first. Cached thumbnails are not sufficient for full views or playback. If an item changed since its scan, rescan before trying to open it.

Check the item's media error and try **Open in another app**. Metadata, thumbnail extraction and playback are separate operations; one can work while another fails. A file extension is not a codec guarantee. Windows HEIC/HEVC requires appropriate installed codecs; RAW, unusual codecs, HDR variants and corrupt files may have limited support. Bimb does not automatically transcode originals or install codec packs.

## A scan stays partial or reports errors

Open **Show scan issues** on the source card. Resolve the reported permissions, disconnection or unreadable subtree, then choose **Rescan**. Partial/error coverage retains existing observations rather than marking the inaccessible contents missing. Media errors have a separate **Show media needing attention** view.

Pause/cancel may wait for the current media operation to end. Workers have a timeout; an inaccessible drive can also cause OS I/O delays. Resume safely rewalks discoveries rather than restarting all completed extraction.

## Thumbnails say “Storage limit reached”

Open **Preferences → Offline thumbnails**. Increase the budget and save, or explicitly clear thumbnails for a selected source. Reconnect/rescan to create missing thumbnails. Reducing the budget preserves existing cached images. Clearing thumbnails on a disconnected source removes those cached views until its originals become available.

## Search or memories look empty

Reset filters and check the selected source/album/collection. Search finds recorded text, not visual concepts. A collection tied to a removed source or album intentionally stays empty.

Memories uses this day in prior years and known capture dates. Check date corrections, per-item exclusions, excluded date ranges and the memories preference. A file-modification date is labeled as a fallback and is excluded from on-this-day matches.

## A backup or export cannot be saved

Choose a writable location outside indexed sources and outside Bimb's application-data directory. Check available space and OS permissions. Catalog backups exclude originals and thumbnails; JSON exports cannot currently be imported as a catalog restore.

## “This library is already open”

Close other Bimb windows/processes using the same catalog, including developer/test copies, and try again. Do not delete a lock file while an instance is running. Multiple machines must not share a live SQLite catalog.

## The catalog will not open, or disk space ran out

Quit Bimb and preserve the application-data directory before attempting recovery. If copying that directory, include SQLite `-wal`/`-shm` files that remain alongside the database; copying a live database alone is not a consistent backup. Free space on the catalog disk without deleting originals or catalog files you still need.

This preview does not have a complete startup recovery interface. Ask a maintainer to recover from a known-good catalog backup using an isolated data directory. Do not erase/reindex the catalog as the first step: rescanning originals cannot reconstruct user-only captions, tags, albums and favorites. Restore failures and newer-schema messages should be reported with the exact error.

## Report a useful issue

Use the project's [issue tracker](https://github.com/compulsiveDebugger/bimb/issues). Include the Bimb version/commit from the package's `BUILDINFO.json`, OS/CPU, installer type, relevant filesystem, exact error and minimal reproduction steps. Say whether the source is internal/external and whether other applications can read it.

Do not attach a personal catalog, private photos, filenames, captions or GPS by default. A synthetic reproduction and redacted error are usually sufficient. Record whether the behavior occurs with a small local JPEG/PNG folder and whether source files remained unchanged.
