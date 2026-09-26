# 0008. A static, offline-first PWA deployed to GitHub Pages

- Status: Accepted
- Date: 2026-09-24

## Context

The game should be playable from a link, installable on a phone's home screen, and work
without a connection, at no hosting cost.

## Decision

- The build (`scripts/build.mjs`) produces a fully static `dist/`: `engine.wasm` and
  `main.js` with content hashes, `index.html`, a manifest, icons, and a service worker
  generated with the exact asset list of that build.
- Assets are cache-first (their hash changes when they change); the page is network-first
  so a new deploy is picked up when online, and the UI offers a reload.
- Every URL is relative, so the site works at any GitHub Pages sub-path.
- GitHub Actions runs fmt, clippy, tests, typecheck and the build, then deploys `dist/` to
  Pages on every push to `main`.

## Consequences

- Pushing to `main` is a release. Run the CI checks locally first and watch the run.
- No server-side features: saves, settings and blueprints live in localStorage and export
  files.
- The ABI check stops a cached old UI from driving a newer engine.
