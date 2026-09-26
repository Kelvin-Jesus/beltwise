#!/usr/bin/env node
// Start-up smoke and mirror check for a built dist/ (CI runs it before publishing):
//   - the engine instantiates, its ABI matches the UI's, and its content JSON parses;
//   - hand-mirrored tables in web/src/constants.ts match their Rust definitions
//     (stats block, sprite ids, ABI and inspector lengths).
// Usage: node scripts/check-build.mjs [dist-dir]

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const dist = process.argv[2] ?? join(root, 'dist');
const errors = [];
const read = (p) => readFileSync(join(root, p), 'utf8');
const constants = read('web/src/constants.ts');

// Rust `pub const NAME: T = value;` inside `pub mod <module> { ... }`.
function rustModule(file, module) {
  const src = read(file);
  const start = src.indexOf(`pub mod ${module} {`);
  if (start < 0) throw new Error(`${file}: no mod ${module}`);
  const body = src.slice(start, src.indexOf('\n}', start));
  return Object.fromEntries([...body.matchAll(/pub const (\w+): \w+ = (\d+);/g)].map((m) => [m[1], Number(m[2])]));
}
function tsObject(name) {
  const m = constants.match(new RegExp(`export const ${name} = \\{([\\s\\S]*?)\\}`));
  if (!m) throw new Error(`constants.ts: no ${name}`);
  return Object.fromEntries([...m[1].matchAll(/(\w+): (\d+)/g)].map((x) => [x[1], Number(x[2])]));
}
const pascal = (s) => s.toLowerCase().replace(/(^|_)([a-z0-9])/g, (_, __, c) => c.toUpperCase());
function mirror(label, rust, ts) {
  for (const [name, value] of Object.entries(rust)) {
    const key = pascal(name);
    if (!(key in ts)) errors.push(`${label}: ${name} (${value}) has no ${key} in constants.ts`);
    else if (ts[key] !== value) errors.push(`${label}: ${name} is ${value} in Rust but ${ts[key]} in constants.ts`);
  }
}
const tsConst = (name) => Number(constants.match(new RegExp(`export const ${name} = (\\d+)`))?.[1]);
const rustConst = (file, name) => Number(read(file).match(new RegExp(`pub const ${name}: \\w+ = (\\d+)`))?.[1]);

mirror('Stat', rustModule('engine/src/world.rs', 'stat'), tsObject('Stat'));
mirror('Sprite', rustModule('engine/src/render.rs', 'sprite'), tsObject('Sprite'));
for (const [name, file] of [['STATS_LEN', 'engine/src/world.rs'], ['INFO_LEN', 'engine/src/lib.rs'], ['ABI_VERSION', 'engine/src/lib.rs']]) {
  if (rustConst(file, name) !== tsConst(name)) errors.push(`${name} is ${rustConst(file, name)} in Rust but ${tsConst(name)} in constants.ts`);
}

// The built engine starts, speaks the UI's ABI, and exports parseable content.
try {
  const { instance } = await WebAssembly.instantiate(readFileSync(join(dist, 'engine.wasm')), {});
  const x = instance.exports;
  if (x.fx_abi() !== tsConst('ABI_VERSION')) errors.push(`engine ABI ${x.fx_abi()} != UI ABI ${tsConst('ABI_VERSION')}`);
  x.fx_init(64, 64, 1);
  const json = new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.fx_content(), x.fx_content_len()));
  const content = JSON.parse(json);
  if (!content.items?.length || !content.buildings?.length) errors.push('content JSON has no items or buildings');
  x.fx_tick(60);
} catch (e) {
  errors.push(`engine start-up failed: ${e.message}`);
}

if (errors.length) {
  console.error(`check-build: ${errors.length} problem(s)\n- ${errors.join('\n- ')}`);
  process.exit(1);
}
console.log('check-build: engine starts, ABI and content OK, mirrored tables match');
