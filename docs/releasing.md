# Package, verify and release Bimb

[Documentation home](README.md) · [Development](development.md) · [Installation](installation.md) · [Validation evidence](validation.md)

## Current readiness

The intended end-user flow is a drag-to-Applications DMG on macOS and a per-user setup EXE on Windows. Source-building is not an end-user installation step.

The project is still a development preview. A successful compilation alone does not prove installer behavior, OS trust, codec coverage or fresh-machine usability. Public distribution needs licensing decisions, macOS Developer ID signing/notarization, a Windows signing plan and the acceptance checks below. No signing credentials or certificate purchase are assumed by this repository.

| Packaging item | Repository behavior |
|---|---|
| Mac app and DMG | `npm run desktop:build` requests both on macOS. Explicit icon resources are bundled. |
| Mac preview signature | `signingIdentity: "-"` requests a complete ad-hoc bundle signature; it is not notarization or publisher verification. |
| Windows installer | Native NSIS `-setup.exe`, explicitly `currentUser`; older app-version installation is blocked. |
| Windows runtime dependency | Offline WebView2 setup is embedded. Initial packaging downloads it on the build machine. |
| Windows signing | Not configured by default; a trusted publisher signature must be supplied for public distribution. |
| Deliverables | Collector includes installer/archive, SHA-256, build information and offline Markdown guides. |
| CI | Native tests/lint/build, actual installer packaging, Mac bundle/image checks and downloadable preview artifacts. A separate disk-image job preserves filesystem test failures. |
| Publishing | Workflows upload preview artifacts only. They do not create a public GitHub Release or tag. |

## Build on the target operating system

Use a clean intended revision and satisfy [development prerequisites](development.md). Keep the application version consistent in `package.json`, workspace `Cargo.toml` and `src-tauri/tauri.conf.json`. Run the documented checks, then:

```sh
npm run desktop:build -- --ci
npm run installers:collect
```

Native outputs are under `target/release/bundle`. The collector copies a matching version/architecture installer into `target/release/installers`, gives it a descriptive OS/CPU filename and adds the guides. It rejects a missing installer; it does not convert a standalone executable into a setup program. Re-run the collector after signing so checksums match the final files.

On a Mac with unavailable DiskImages services, `npm run desktop:app` followed by `npm run desktop:archive` creates a `.app.zip` preview with `ditto`, preserving bundle structure and permissions. Treat the DMG path as unverified until it has been built and mounted successfully on another supported Mac.

## Native CI artifacts

The **Desktop compatibility** workflow runs on pushes, pull requests and manual dispatch. Its `native` jobs run on macOS 26 and Windows. Native tests and lint precede packaging. A separate `disk-images` job runs the FAT32/exFAT image tests; its failure remains a release gate and is not silently ignored to produce a green installer result.

After a native job succeeds, the `Bimb-preview-<runner>-<commit>` artifact contains the collected directory. [Download it from the workflow run](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts), extract it, and review `START-HERE.txt`, `BUILDINFO.json` and the checksum. Sign-in/repository read access is needed for these temporary artifacts. The configured retention is 30 days.

A workflow file in an unpushed checkout has not run on GitHub. A successful old run does not validate a changed installer configuration. Record the exact commit and native job URL used for acceptance. For end users, upload approved packages to a deliberately published release so downloading does not depend on Actions access or artifact retention.

## macOS signing and notarization

Preview ad-hoc signing repairs bundle integrity but still permits Gatekeeper warnings. For an outside-App-Store release, install the appropriate **Developer ID Application** certificate and configure notarization credentials using protected environment variables or a secret store. See [Tauri's macOS signing procedure](https://v2.tauri.app/distribute/sign/macos/).

Override the preview identity when creating a public package. For example, on the Mac build host:

```sh
npm run desktop:build -- --ci --config '{"bundle":{"macOS":{"signingIdentity":"Developer ID Application: LEGAL NAME (TEAMID)"}}}'
```

Replace that public certificate name with the actual installed identity. Supply notarization credentials separately following the official procedure; do not store private keys, passwords or tokens in the repository, logs or release artifacts. Tauri supports Apple account or App Store Connect API authentication. Keep hardened runtime enabled. Include successful launch of the embedded media helper in fresh-machine acceptance.

Verify the resulting public package:

```sh
codesign --verify --deep --strict target/release/bundle/macos/Bimb.app
spctl --assess --type execute --verbose=2 target/release/bundle/macos/Bimb.app
xcrun stapler validate target/release/bundle/macos/Bimb.app
hdiutil verify target/release/bundle/dmg/*.dmg
```

Confirm the notarization outcome and stapled ticket for the actual distributed app/image. A valid ad-hoc signature passes `codesign` but is expected to fail the public trust/notarization checks. Do not use `--skip-stapling` for a public package expected to install offline.

## Windows publisher signing

Configure a Windows code-signing identity/service and timestamping before the final installer build. Sign the executable and installer with the same release identity, using [Tauri's Windows signing options](https://v2.tauri.app/distribute/sign/windows/). Signing can require a hardware-backed key or managed signing service; a TLS certificate is not a substitute.

Check the final installer on Windows PowerShell:

```powershell
Get-AuthenticodeSignature .\target\release\bundle\nsis\Bimb_0.1.0_x64-setup.exe | Format-List Status,SignerCertificate
```

Use the actual version in the filename. Record a valid signature and intended publisher. Signing establishes identity/integrity, but new releases can still receive SmartScreen reputation warnings. Smart App Control and managed-device policies make unsigned previews unsuitable for a universal install promise. Do not make “disable security software” part of installation instructions.

## Installer acceptance checklist

Run these against the packaged installer on a supported machine or fresh VM with no Rust, Node, Xcode/Visual Studio, source checkout or Vite server. Record OS/CPU, Bimb version, commit, hash and signing status. Test with synthetic media before personal data.

### macOS

- Download through a browser so quarantine/Gatekeeper behavior is exercised.
- Verify the installer hash, mount the DMG and confirm Bimb and Applications are obvious.
- Drag Bimb to Applications, eject the DMG and open the installed copy.
- Confirm the app icon, name, minimum OS and CPU are correct.
- For a public release, verify Developer ID/notarization and launch while the network is disconnected.
- Add synthetic JPEG/PNG/HEIC and supported video files; verify native helper execution, thumbnail/preview/playback and original reveal.
- Close/reopen, reconnect a source, back up/restore and verify annotations persist.
- Upgrade an installed older version after taking a catalog backup; confirm retained organization.
- Remove the app and confirm originals remain. Test catalog retention separately from deliberate catalog deletion.

### Windows

- Download/extract and run the setup EXE as a normal user; record any elevation/dependency prompts.
- Check publisher/trust behavior, default per-user location, Start menu entry, icons and uninstaller entry.
- With no WebView2 runtime and the network disconnected, confirm the embedded dependency installer succeeds. Repeat with an existing runtime.
- Start from the installed shortcut without a checkout or developer tools; confirm a usable native window.
- Add a Unicode/space-containing path; browse JPEG/PNG, inspect metadata, test video playback and installed optional codecs.
- Verify favorite/caption persistence, original reveal/default-app opening, backup/restore and reconnect after a drive-letter change.
- Test upgrade, downgrade rejection, uninstall and reinstall. Verify what happens to catalog data and that originals remain untouched.
- Repeat on a managed/Smart App Control configuration before promising compatibility with those policies.

### Both platforms

- Confirm application-controlled offline operations and absence of unexpected media/network uploads.
- Check read-only sources and physical NTFS/FAT/exFAT unplug/reconnect behavior.
- Keep metadata/thumbnail/playback codec results separate.
- Test cancellation/restart, disk-full and recovery scenarios before general availability.
- Check all shipped guides and hashes against the final package. A matching hash alone does not prove publisher identity.

## Publish deliberately

1. Finish the product/license decision and platform acceptance checks.
2. Build/sign/notarize from the reviewed revision and collect final files with matching hashes.
3. Review the [draft 0.1.0 release notes](release-notes-0.1.0.md) against the actual packages, tested platforms and formats, and catalog compatibility.
4. Create a draft release and attach only the final packages, manifests and documentation. Check them before publishing.
5. Publish only with the user's authorization. Update the installation page with the real release link and validation evidence.

Do not advertise every OS/CPU or filesystem as certified from a compile check. Until the checklist is complete, label artifacts as development previews.
