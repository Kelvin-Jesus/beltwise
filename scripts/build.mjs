#!/usr/bin/env node
// Builds the static site into dist/:
//   engine.wasm  cargo (wasm32-unknown-unknown, release) -> wasm-opt -O3
//   main.js      esbuild bundle of web/src (ESM, minified)
//   index.html   web/index.html with a content-hash query on main.js
//   sw.js        service worker precaching exactly this build (offline play)
//   + web/public/* (icons, manifest)
// Everything is referenced with relative URLs, so dist/ works from any sub-path.
//
//   node scripts/build.mjs          production build
//   node scripts/build.mjs --dev    dev server on http://localhost:8000, rebuilds on change
//
// Env: WASM_OPT=/path/to/wasm-opt to use a native Binaryen instead of the npm one.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, watch, writeFileSync } from 'node:fs';
import { networkInterfaces } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as esbuild from 'esbuild';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const dev = process.argv.includes('--dev');
// Dev builds go to .dev/ so a production dist/ (e.g. served to a phone) is never clobbered.
const dist = join(root, dev ? '.dev' : 'dist');
const port = Number(process.env.PORT) || 8000;

// Post-MVP features Rust >= 1.82 enables by default for wasm32-unknown-unknown. wasm-opt
// refuses to validate the module unless they are declared. All ship in every browser
// with WebGL2 support (Safari 15+, Chrome 91+, Firefox 90+).
const WASM_FEATURES = [
  '--enable-bulk-memory',
  '--enable-nontrapping-float-to-int',
  '--enable-sign-ext',
  '--enable-mutable-globals',
  '--enable-multivalue',
  '--enable-reference-types',
];

const lanAddress = () =>
  Object.values(networkInterfaces())
    .flat()
    .find((i) => i && i.family === 'IPv4' && !i.internal)?.address ?? 'localhost';
const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, stdio: 'inherit' });
const hash = (file) => createHash('sha256').update(readFileSync(file)).digest('hex').slice(0, 10);
const kb = (file) => `${(statSync(file).size / 1024).toFixed(1)} KB`;

function buildWasm() {
  run('cargo', ['build', '-p', 'engine', '--release', '--target', 'wasm32-unknown-unknown']);
  const src = join(root, 'target/wasm32-unknown-unknown/release/engine.wasm');
  const out = join(dist, 'engine.wasm');
  if (dev) {
    cpSync(src, out);
  } else {
    const bin = process.env.WASM_OPT || join(root, 'node_modules/.bin', process.platform === 'win32' ? 'wasm-opt.cmd' : 'wasm-opt');
    run(bin, ['-O3', ...WASM_FEATURES, '--strip-debug', '--strip-producers', src, '-o', out]);
    console.log(`engine.wasm  ${kb(src)} -> ${kb(out)} (wasm-opt -O3)`);
  }
  return hash(out);
}

function esbuildOptions(wasmHash) {
  return {
    entryPoints: [join(root, 'web/src/main.ts')],
    outfile: join(dist, 'main.js'),
    bundle: true,
    format: 'esm',
    target: ['es2022', 'chrome91', 'safari15', 'firefox90'],
    minify: !dev,
    sourcemap: dev ? 'inline' : false,
    legalComments: 'none',
    define: {
      __WASM_FILE__: JSON.stringify(dev ? 'engine.wasm' : `engine.wasm?v=${wasmHash}`),
      __DEV__: String(dev),
    },
    logLevel: 'warning',
  };
}

function writeHtml(jsHash) {
  const html = readFileSync(join(root, 'web/index.html'), 'utf8');
  const src = dev ? 'main.js' : `main.js?v=${jsHash}`;
  writeFileSync(join(dist, 'index.html'), html.replace('src="main.js"', `src="${src}"`));
}

/** Service worker with this build's exact asset list, so the game works offline. */
function writeServiceWorker(version, hashed) {
  const pub = join(root, 'web/public');
  const statics = existsSync(pub) ? readdirSync(pub).filter((f) => !f.startsWith('.')) : [];
  const assets = ['./', ...hashed, ...statics];
  const sw = readFileSync(join(root, 'web/sw.js'), 'utf8')
    .replaceAll('__VERSION__', version)
    .replaceAll('__ASSETS__', JSON.stringify(assets));
  writeFileSync(join(dist, 'sw.js'), sw);
}

function copyPublic() {
  const pub = join(root, 'web/public');
  if (existsSync(pub)) cpSync(pub, dist, { recursive: true });
  // Harmless for Actions-based Pages deploys; keeps branch-based deploys from running Jekyll.
  writeFileSync(join(dist, '.nojekyll'), '');
}

rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });
copyPublic();
const wasmHash = buildWasm();

if (!dev) {
  await esbuild.build(esbuildOptions(wasmHash));
  const jsFile = join(dist, 'main.js');
  const jsHash = hash(jsFile);
  writeHtml(jsHash);
  writeServiceWorker(`${wasmHash}-${jsHash}`, [`main.js?v=${jsHash}`, `engine.wasm?v=${wasmHash}`]);
  console.log(`main.js      ${kb(jsFile)}`);
  console.log(`dist/ ready  (wasm ${wasmHash}, js ${jsHash})`);
} else {
  writeHtml('');
  const ctx = await esbuild.context(esbuildOptions(wasmHash));
  await ctx.watch();
  // HOST=0.0.0.0 npm run dev  -> reachable from a phone on the same Wi-Fi.
  const host = process.env.HOST || '127.0.0.1';
  await ctx.serve({ servedir: dist, port, host });
  console.log(`dev server   http://${host === '0.0.0.0' ? lanAddress() : 'localhost'}:${port}/`);

  // One debounce timer per task, so a burst of edits across files rebuilds everything.
  const timers = new Map();
  const rebuild = (what, fn) => {
    clearTimeout(timers.get(what));
    timers.set(
      what,
      setTimeout(() => {
        try {
          fn();
          console.log(`rebuilt ${what}`);
        } catch (e) {
          console.error(`${what} build failed: ${e.message}`);
        }
      }, 100),
    );
  };
  watch(join(root, 'engine/src'), { recursive: true }, () => rebuild('engine.wasm', buildWasm));
  // Watch the directory, not the file: editors that save atomically replace the inode.
  watch(join(root, 'web'), (_, file) => file === 'index.html' && rebuild('index.html', () => writeHtml('')));
  watch(join(root, 'web/public'), { recursive: true }, () => rebuild('public/', copyPublic));
}
