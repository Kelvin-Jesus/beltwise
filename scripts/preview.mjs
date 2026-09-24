#!/usr/bin/env node
// Serves dist/ the way GitHub Pages will: under a repository sub-path, with nothing at the
// root. If the game loads here, relative-path handling is correct for <user>.github.io/<repo>/.
//
//   npm run build && npm run preview     ->  http://localhost:4173/beltwise/

import { createReadStream, existsSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { networkInterfaces } from 'node:os';
import { dirname, extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(root, 'dist');
const base = `/${process.env.BASE || 'beltwise'}/`;
const port = Number(process.env.PORT) || 4173;
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.webmanifest': 'application/manifest+json',
  '.json': 'application/json',
};

createServer((req, res) => {
  const path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  if (path === base.slice(0, -1)) {
    res.writeHead(301, { location: base }).end(); // Pages redirects /repo -> /repo/
    return;
  }
  if (!path.startsWith(base)) {
    res.writeHead(404).end(`Not found. The site lives under ${base}`);
    return;
  }
  let file = normalize(join(dist, path.slice(base.length)));
  if (!file.startsWith(dist)) return res.writeHead(403).end();
  if (existsSync(file) && statSync(file).isDirectory()) file = join(file, 'index.html');
  if (!existsSync(file)) return res.writeHead(404).end('Not found');
  res.writeHead(200, { 'content-type': types[extname(file)] || 'application/octet-stream', 'cache-control': 'max-age=600' });
  createReadStream(file).pipe(res);
}).listen(port, () => {
  console.log(`GitHub Pages preview: http://localhost:${port}${base}`);
  for (const i of Object.values(networkInterfaces()).flat()) {
    if (i && i.family === 'IPv4' && !i.internal) console.log(`  on your phone (same Wi-Fi): http://${i.address}:${port}${base}`);
  }
});
