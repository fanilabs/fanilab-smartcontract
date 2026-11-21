# TypeScript SDK Dependency Policy

The TypeScript SDK keeps `@stellar/stellar-sdk` on the current compatible major
version until the SDK invocation client is migrated and verified against a newer
major. The existing `^11.0.0` range is therefore intentional rather than an
unreviewed stale pin.

The committed `sdk/typescript/package-lock.json` is the reproducibility source for
SDK installs. Dependabot monitors `/sdk/typescript` weekly and can update both the
manifest range and lockfile; CI should install with `npm ci` when SDK CI coverage
is enabled.

## Supported Runtimes

The SDK declares its runtime contract in `sdk/typescript/package.json`:

- **Node.js**: `>=18` (declared via `engines.node`). Node 18 is the minimum
  because the SDK relies on the global `fetch`, `AbortController`, and
  `structuredClone` available from that release onward.
- **Browsers**: modern evergreen browsers (Chrome, Edge, Firefox, Safari) that
  support ES2020 and the Fetch API. The package exposes a browser-safe entry via
  the `browser` and `exports` fields so bundlers resolve a build that does not
  depend on Node-only globals.

## Required Polyfills

The SDK source uses `Buffer` and Node-style globals. When targeting browsers,
consumers must provide the following polyfills (most bundlers such as Vite,
webpack, and esbuild can inject them automatically):

- `Buffer` — via the `buffer` package (`globalThis.Buffer = Buffer`).
- `process` — via the `process` package, at minimum `process.env`.
- `global` — alias to `globalThis` where referenced.

Node.js consumers need no polyfills.

## Verifying the Browser-Safe Entry

After installing the SDK, confirm the browser entry resolves without pulling in
Node-only modules:

```sh
cd sdk/typescript
node -e "const p=require('./package.json'); if(!p.browser||!p.exports) process.exit(1); console.log('browser entry:', p.browser)"
```

Bundler-based smoke checks should import the SDK from the browser entry and
assert that `Buffer`/`process` are only referenced through the documented
polyfills.
