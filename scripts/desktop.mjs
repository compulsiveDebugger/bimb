import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
const env = { ...process.env };
if (existsSync('.tools/cargo/bin/cargo')) {
  env.CARGO_HOME = resolve('.tools/cargo'); env.RUSTUP_HOME = resolve('.tools/rustup');
  env.PATH = `${env.CARGO_HOME}/bin:${env.PATH}`;
}
const child = spawn(resolve('node_modules/.bin/tauri'), process.argv.slice(2), { env, stdio: 'inherit' });
child.on('exit', code => process.exit(code ?? 1));
