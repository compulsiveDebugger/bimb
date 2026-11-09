# Install Bimb

[Documentation home](README.md) · [First-use walkthrough](user-guide.md#your-first-library) · [Troubleshooting](troubleshooting.md)

Installing a packaged copy requires no Rust, Node.js, Xcode or Visual Studio. Those tools are only for people [building from source](development.md).

## Before downloading

| Computer | Package to choose | Current evidence |
|---|---|---|
| Apple Silicon Mac (M1 or newer), macOS 26 or newer | `Bimb_<version>_macos_arm64.dmg` | Local app build and native smoke tests; installer acceptance tracked in [validation](validation.md). |
| Windows 11, x64 Intel/AMD PC | `Bimb_<version>_windows_x64-setup.exe` | Native core tests and lint passed in CI; fresh-PC install, desktop UI and playback need acceptance testing. |
| Intel Mac, older macOS, Windows ARM or other systems | No validated package in this preview | Do not assume a package for another CPU will work. |

On a Mac, **Apple menu → About This Mac** shows the chip and macOS version. On Windows, **Settings → System → About → System type** shows the architecture. Windows 10 is outside the validated preview matrix.

You need space on the computer for the application, catalog and cached thumbnails. The initial thumbnail budget is 1 GiB and can be changed. Original media can stay on internal or external storage.

## Get the right download

1. Look at the project's [GitHub Releases page](https://github.com/compulsiveDebugger/bimb/releases). Use an installer explicitly published for your platform. A release is available only when a maintainer has published its files; this documentation does not mean a public release exists.
2. For an unpublished preview, a maintainer can share a successful **Desktop compatibility** workflow's installer artifact. In [GitHub Actions](https://github.com/compulsiveDebugger/bimb/actions/workflows/desktop.yml), select the intended branch/commit, open a completed native job and find its `Bimb-preview-…` artifact. GitHub requires sign-in and repository read access for artifact downloads. Artifacts expire after 30 days.
3. Extract the downloaded artifact ZIP. It contains the installer, `START-HERE.txt`, `SHA256SUMS.txt`, `BUILDINFO.json` and offline guides. Run the installer inside it.

The workflow must run successfully on a revision containing installer packaging before those artifacts are available. GitHub's **Source code (zip)** and **Source code (tar.gz)** downloads are source files, not application installers. A raw `bimb.exe` from a developer's build directory is not the supported installation package.

## Install on macOS

1. Quit an older running copy of Bimb.
2. Double-click the `.dmg` file. A Finder window opens.
3. Drag **Bimb** into the **Applications** folder shown in that window. macOS may ask for permission to write to Applications. A local `~/Applications` folder is an alternative when system policy permits it.
4. Eject the Bimb disk image in Finder.
5. Open **Applications → Bimb**. Run the copied application rather than the copy on the disk image.
6. Choose **Add folder** and select a small folder of exported photos to start.

A maintainer may provide a `.app.zip` preview when disk-image creation is unavailable. Extract it, copy **Bimb.app** to Applications, then open the copied app. The ZIP must preserve the app bundle's executable permissions and resources; use the project's archive command when creating one.

### macOS preview trust

The repository's preview bundle is **ad-hoc signed**. That checks bundle integrity but does not establish an Apple-verified publisher or notarization. A downloaded preview can still be blocked by Gatekeeper. Only open a preview you intended to obtain from a trusted maintainer; consult [blocked-launch help](troubleshooting.md#macos-blocks-the-application). A public release needs a Developer ID signature, notarization and a stapled ticket. Do not disable Gatekeeper to install Bimb.

## Install on Windows

1. Quit an older running copy of Bimb.
2. Double-click the `-setup.exe` file.
3. Follow the installer and keep the default location unless you have a reason to change it. The configured NSIS installer installs for your current user.
4. Finish installation and open **Bimb** from the Start menu.
5. Choose **Add folder** and select a small exported-media folder.

The installer includes Microsoft's offline WebView2 installer, so it can install the webview dependency without a download. This makes the Windows download larger. Machine policies or dependency installation can still require administrator help; a per-user configuration is not a guarantee that every managed PC permits installation.

Windows preview signing is not configured. SmartScreen or organizational policy may warn or block the installer. Verify the source and package before proceeding; if Smart App Control or your organization blocks unsigned software, obtain an approved signed build. Do not turn off system protections to use a preview. See [Windows launch help](troubleshooting.md#windows-blocks-the-installer-or-bimb-will-not-open).

## Verify the download (optional)

Compare the file's SHA-256 with the same filename in `SHA256SUMS.txt`:

```sh
# macOS: run in the extracted download folder, substituting the actual filename.
shasum -a 256 Bimb_0.1.0_macos_arm64.dmg
```

```powershell
# Windows PowerShell: substitute the actual filename.
Get-FileHash .\Bimb_0.1.0_windows_x64-setup.exe -Algorithm SHA256
```

A matching hash detects an incomplete or altered download relative to the supplied manifest. It is not a publisher signature; trust the channel that provided both files.

## Upgrade safely

1. In Bimb, open **Preferences → Back up catalog** and save the backup outside indexed sources. Back up originals separately.
2. Quit Bimb and install the newer package for the same OS/CPU. Replace the app on macOS or run the Windows setup program.
3. Reopen Bimb. The local catalog stays separate from the installed application. Confirm your sources, albums and annotations appear.

Catalog upgrades can prevent older versions from reopening the catalog. Windows installers block version downgrades. Keep your pre-upgrade catalog backup; do not expect replacing the application with an older build to undo a migration. Catalogs from schemas 1–3 belong to macOS; current catalogs record their native path encoding. Moving a catalog between macOS and Windows is not supported yet.

Updates are manual. Bimb has no automatic updater or account activation.

## Where Bimb keeps its data

| Platform | Normal catalog folder |
|---|---|
| macOS | `~/Library/Application Support/app.bimb.desktop` |
| Windows | `%APPDATA%\app.bimb.desktop` |

The folder contains the SQLite catalog, thumbnails, temporary working files and platform helper data. These locations are separate from your originals. They rely on your account/disk protection and are not independently encrypted. `BIMB_DATA_DIR`, when deliberately set by a developer, overrides the normal catalog location.

Keep one writable catalog on a local computer disk. Do not share an active catalog through a network drive or cloud-sync folder.

## Uninstall

Back up the catalog first if you want to keep organization. On macOS, quit Bimb and move **Bimb.app** from Applications to Trash. On Windows, use **Settings → Apps → Installed apps → Bimb → Uninstall**. If an uninstaller offers to remove application data, leave that option unchecked to keep the catalog.

Application removal does not require deleting originals. To permanently remove Bimb's catalog and thumbnails, quit the app and deliberately remove its catalog folder only after saving any wanted backup. Removing that folder loses Bimb-only organization; reinstalling and rescanning cannot recreate captions, albums or favorites from originals.

## Installation is complete when…

- Bimb opens from Applications or Start with no development server running.
- A small JPEG/PNG folder can be added, browsed and searched.
- A caption/favorite survives closing and reopening the application.
- Original media remains in its source folder.

Continue with the [user guide](user-guide.md). Maintainers should run the fuller [installer acceptance checklist](releasing.md#installer-acceptance-checklist) before calling a build ready for general use.

References: [Tauri DMG packaging](https://v2.tauri.app/distribute/dmg/), [offline WebView2 installation](https://v2.tauri.app/distribute/windows-installer/#offline-installer), [macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Windows signing](https://v2.tauri.app/distribute/sign/windows/), [GitHub artifact downloads](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts).
