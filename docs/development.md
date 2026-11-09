# Build and develop Bimb

[Documentation home](README.md) · [Packaging and release](releasing.md)

These instructions are for contributors. People installing a published package need only the [installation guide](installation.md).

## Prerequisites

| Target | Build machine requirements |
|---|---|
| macOS Apple Silicon | macOS 26+, Xcode command-line tools including Swift, Rust 1.99+, Node 22.12+ (CI uses Node 24), npm and Git |
| Windows x64 | Windows x64, Visual Studio Build Tools with Desktop development with C++ and Windows SDK, Rust MSVC toolchain 1.99+, Node 22.12+, npm, Git and WebView2 for development |

Install prerequisites from their official providers: [Rust](https://rustup.rs/), [Node](https://nodejs.org/), [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Build each platform on its native host. A Windows-target Rust check on a Mac is useful type checking, but does not produce or test a Windows installer. Intel/universal Mac and Windows ARM distribution have separate validation gates.

## Clone and install dependencies

```sh
git clone https://github.com/compulsiveDebugger/bimb.git
cd bimb
npm ci
```

Check out the branch or tag you intend to build. The clone command uses the repository's default branch; a new feature may still be on another branch. Keep `Cargo.lock` and `package-lock.json` with the checkout. Initial dependency installation and installer-tool/WebView2 downloads require internet.

The desktop wrapper uses a project-local `.tools/cargo`/`.tools/rustup` installation if present, otherwise Rust from `PATH`. `.tools`, `.runtime`, `target`, `dist` and `node_modules` are generated/ignored.

## Run with a separate test catalog

On macOS:

```sh
mkdir -p .runtime/dev-library
export BIMB_DATA_DIR="$PWD/.runtime/dev-library"
npm run desktop
```

On Windows PowerShell:

```powershell
New-Item -ItemType Directory -Force .runtime\dev-library | Out-Null
$env:BIMB_DATA_DIR = (Resolve-Path .runtime\dev-library).Path
npm run desktop
```

Keep the catalog outside indexed sources. Use synthetic fixtures before personal media. Unset `BIMB_DATA_DIR` when you intend to use the normal catalog. The override is process-local and inherited by the launched app; it is not a command-line preference saved by Bimb.

`npm run dev` alone opens a browser UI preview without a native library connection. Use `npm run desktop` for filesystem, dialogs, codecs and the catalog bridge.

## Checks

Both platforms:

```sh
npm run docs:check
npm run check
npm run build
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

For a Mac checkout using project-local Rust, replace `cargo` with `sh scripts/cargo.sh`. Add `--offline` to Cargo commands only after their dependencies are cached. Native media tests need codec services and filesystem notifications; a restricted sandbox can cause failures that require a run with OS-service access.

The fixture generator is `scripts/make-fixtures.swift`. Committed JPEG/PNG/HEIC/H.264/HEVC fixtures contain synthetic imagery. `tests/drives.rs` uses a standalone worker host on Windows and the same native implementation as the desktop app.

For macOS mounted-image testing:

```sh
python3 scripts/test-drive-images.py
```

This creates only temporary FAT32/exFAT disk images and tests writable/read-only scans. It must never select a physical disk. Some environments reject image creation; record that as unavailable validation rather than passing the test. FAT32 has passed in the native CI run recorded in [validation](validation.md); exFAT image creation failed there.

## Build installable packages

```sh
npm run desktop:build
npm run installers:collect
```

On a Mac this requests an app bundle and DMG; on Windows it requests an NSIS installer with offline WebView2 setup. Share the collected directory `target/release/installers/`, which contains the native package, checksums, build information and offline guides. It is generated output, not repository source.

If a Mac host cannot create disk images:

```sh
npm run desktop:app
npm run desktop:archive
```

This creates an explicitly labeled `.app.zip` preview after strict bundle-signature verification. It does not prove that DMG packaging works. To rebundle an already compiled release, use `npm run desktop:bundle -- --ci`. Explicit `--bundles` arguments override the wrapper's defaults.

See [release preparation](releasing.md) for trust, signing and clean-machine checks. Do not describe an ad-hoc/unsigned preview as a signed public release.

## Useful locations

| Path | Purpose |
|---|---|
| `crates/bimb-core/src` | Catalog, indexing, drive identity, watcher and media coordination |
| `crates/bimb-core/native/MediaWorker.swift` | macOS ImageIO/AVFoundation subprocess |
| `crates/bimb-core/src/platform/windows` | Windows native media implementation |
| `src-tauri` | Desktop commands, capabilities, bundle configuration and local media protocol |
| `src` | Svelte/TypeScript interface |
| `target/release/bundle/macos/Bimb.app` | Native Mac app |
| `target/release/bundle/dmg` | Native Mac disk image output |
| `target/release/bundle/nsis` | Native Windows installer output |
| `target/release/installers` | Collected package ready for a maintainer to review/share |

The collector supports a custom `CARGO_TARGET_DIR` for native host builds. Explicit cross-compilation target subdirectories are outside its current interface. It records the checkout commit and dirty state; use a clean checkout for reproducible release provenance.
