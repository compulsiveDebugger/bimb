import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { copyFile, mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));
const config = JSON.parse(await readFile(join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
const args = process.argv.slice(2);
if (args.some(arg => arg !== '--app-archive')) throw new Error('Usage: node scripts/collect-installers.mjs [--app-archive]');
const archive = args.includes('--app-archive');
if (!['darwin', 'win32'].includes(process.platform)) throw new Error('Create installers on macOS or Windows.');
if (archive && process.platform !== 'darwin') throw new Error('--app-archive requires macOS.');
const os = process.platform === 'darwin' ? 'macos' : 'windows';
const build = resolve(root, process.env.CARGO_TARGET_DIR || 'target', 'release');
const output = join(build, 'installers');
const stem = `Bimb_${config.version}_${os}_${process.arch}`;
const sourceFolder = join(build, 'bundle', os === 'macos' ? 'dmg' : 'nsis');
const suffix = os === 'macos' ? '.dmg' : '-setup.exe';
let source;
if (archive) {
  source = join(build, 'bundle/macos/Bimb.app');
  await stat(join(source, 'Contents/MacOS/bimb'));
  execFileSync('/usr/bin/codesign', ['--verify', '--deep', '--strict', source], { stdio: 'inherit' });
} else {
  const files = await readdir(sourceFolder);
  const arch = os === 'macos' ? { arm64: 'aarch64', x64: 'x64' }[process.arch] : process.arch;
  const expected = `Bimb_${config.version}_${arch}${suffix}`;
  if (!files.includes(expected)) throw new Error(`Missing ${expected} in ${sourceFolder}. Run npm run desktop:build first.`);
  source = join(sourceFolder, expected);
}
// Check inputs before replacing an older collected package.
await rm(output, { recursive: true, force: true });
await mkdir(join(output, 'docs'), { recursive: true });
const name = `${stem}${archive ? '.app.zip' : suffix}`;
const destination = join(output, name);
if (archive) {
  execFileSync('/usr/bin/ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent', source, destination], { stdio: 'inherit' });
} else {
  await copyFile(source, destination);
}
await copyFile(join(root, 'README.md'), join(output, 'README.md'));
for (const file of (await readdir(join(root, 'docs'))).filter(file => /\.(md|png)$/.test(file))) {
  await copyFile(join(root, 'docs', file), join(output, 'docs', file));
}
const hash = createHash('sha256');
for await (const chunk of createReadStream(destination)) hash.update(chunk);
const sha256 = hash.digest('hex');
let commit = null, dirty = null;
try { commit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(); dirty = !!execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).trim(); } catch { /* Git is optional for local packages. */ }
await writeFile(join(output, 'SHA256SUMS.txt'), `${sha256}  ${name}\n`);
await writeFile(join(output, 'BUILDINFO.json'), JSON.stringify({
  product: config.productName, version: config.version, platform: os, arch: process.arch,
  commit, working_tree_dirty: dirty, created_at: new Date().toISOString(), release_channel: 'development-preview',
  trust_status: 'Preview: verify signing/notarization separately before public release',
  file: name, bytes: (await stat(destination)).size, sha256,
}, null, 2) + '\n');
await writeFile(join(output, 'START-HERE.txt'), `Bimb ${config.version} development preview\n\nOpen docs/installation.md for installation and docs/user-guide.md for everyday use.\nNo Rust, Node.js or developer tools are needed to install the packaged app.\n\n${os === 'macos' ? 'Open the .dmg and drag Bimb to Applications. For .app.zip, extract and copy Bimb.app to Applications.' : 'Run the -setup.exe installer, then open Bimb from Start.'}\n\nPreview packages are not approved public releases. macOS preview builds are ad-hoc signed; Windows signing is not configured. OS security warnings or policies can prevent launch. See docs/troubleshooting.md.\n\nThe installer SHA-256 is in SHA256SUMS.txt. BUILDINFO.json records the source commit; local uncommitted changes may also be present.\n`);
console.log(`Collected ${basename(destination)} (${(await stat(destination)).size} bytes)\n${output}\nSHA-256: ${sha256}`);
