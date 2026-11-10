# Bimb 0.1.0 release notes

**Draft for maintainer review.** The available macOS package is a development preview. Public distribution is pending license approval, publisher signing, and installer acceptance.

Bimb indexes photos and videos in their existing folders. You can search the local catalog, organize media with favorites, tags, captions, albums, and saved searches, and browse cached thumbnails when an original drive is disconnected.

## Packages and compatibility

| Platform | Package | Evidence |
|---|---|---|
| Apple Silicon, macOS 26 or later | `Bimb_0.1.0_macos_arm64.dmg` | Local ad-hoc preview. The developer Mac verified installation, native media decoding, and migration from an earlier catalog. Fresh-machine acceptance and Developer ID notarization are pending. |
| Windows 11 x64 | `Bimb_0.1.0_windows_x64-setup.exe` | Intended per-user NSIS installer with embedded offline WebView2 setup. An earlier CI revision passed native core tests and built the desktop host. The installer, offline setup, desktop behavior, and publisher signature still require verification. |

Install and verify the checksum using the [installation guide](installation.md).

## Media and drives

- Local macOS tests generated metadata and thumbnails for synthetic JPEG, PNG, HEIC, H.264 MP4, and HEVC MOV files. The native viewer played synthetic clips with rotation, variable frame intervals, AAC audio tracks, and HLG metadata. Real phone exports, audio output, and HDR display fidelity are still pending.
- On macOS, FAT32 and exFAT images were scanned in writable and read-only mode. After a forced detach, cached thumbnails remained available and indexing resumed when the image was mounted at a different path. Physical-drive and Windows drive-letter acceptance remain pending.
- The packaged macOS app indexed 3,000 small synthetic files with zero media errors. Performance on the 1 TB library is still unmeasured. [Validation results](validation.md)

## Upgrade and recovery

- Back up your catalog before updating. Earlier macOS catalogs from schemas 1–3 migrate to schema 4. The upgraded catalog requires a schema-4-compatible build. Keep separate catalogs and backups for macOS and Windows.
- Catalog backups preserve organization and source references. Keep independent backups of original media. Bimb regenerates thumbnails when the original sources are available after a restore.
- Local macOS fault tests verified that annotations survive a full disk, an interrupted scan, and a forced shutdown during indexing. Bimb stages restores before replacing the live catalog. [Backup and restore](user-guide.md#back-up-and-restore-your-organization)

## Before publishing

Confirm the distribution license and publisher identities. Build both final installers from the same reviewed revision. Complete the clean-machine and physical-drive checks, verify notarization and Windows signatures, and record the checksums of the final packages. [Release procedure](releasing.md)
