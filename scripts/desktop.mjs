import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { delimiter, resolve } from 'node:path';
const env = { ...process.env };
if (existsSync(`.tools/cargo/bin/cargo${process.platform === 'win32' ? '.exe' : ''}`)) {
  env.CARGO_HOME = resolve('.tools/cargo'); env.RUSTUP_HOME = resolve('.tools/rustup');
  env.PATH = `${env.CARGO_HOME}/bin${delimiter}${env.PATH}`;
}
const args = process.argv.slice(2);
if (args[0] === 'build' && !args.includes('--bundles')) {
  args.push('--bundles', process.platform === 'win32' ? 'nsis' : 'app');
}
const child = spawn(process.execPath, [resolve('node_modules/@tauri-apps/cli/tauri.js'), ...args], { env, stdio: 'inherit' });
child.on('error', error => { console.error(error.message); process.exit(1); });
child.on('exit', code => process.exit(code ?? 1));
