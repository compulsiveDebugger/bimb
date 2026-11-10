import { readFile, readdir, stat } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const files = [join(root, 'README.md'), ...(await readdir(join(root, 'docs'))).filter(f => f.endsWith('.md')).map(f => join(root, 'docs', f))];
const errors = [];
for (const file of files) {
  const content = await readFile(file, 'utf8');
  for (const match of content.matchAll(/!?\[[^\]]*\]\(([^)]+)\)/g)) {
    const target = match[1].replace(/^<|>$/g, '').split(/\s+"/)[0];
    if (/^[a-z][a-z\d+.-]*:/i.test(target) || target.startsWith('#')) continue;
    const path = resolve(dirname(file), decodeURIComponent(target.split('#')[0]));
    try { await stat(path); } catch { errors.push(`${file}: missing link target ${target}`); }
  }
}
if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
else console.log(`Checked local links in ${files.length} Markdown documents.`);
