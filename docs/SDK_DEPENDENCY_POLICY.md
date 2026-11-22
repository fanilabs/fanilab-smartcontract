# TypeScript SDK Dependency Policy

The TypeScript SDK keeps `@stellar/stellar-sdk` on the current compatible major
version until the SDK invocation client is migrated and verified against a newer
major. The existing `^17.0.0` range is therefore intentional rather than an
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

## Supported Module Consumption Modes

The SDK supports both CommonJS and ES module consumers. `sdk/typescript/package.json`
declares this contract explicitly:

- **CommonJS** (`require`) — resolved through the `require` condition of the
  `exports` map, falling back to the top-level `main` field for consumers that do
  not honor `exports`.
- **ESM** (`import`) — resolved through the `import` condition of the `exports`
  map.
- **TypeScript types** — resolved through the `types` condition of the `exports`
  map, with the top-level `types` field retained for legacy resolution.

The top-level `main` and `types` fields are preserved alongside `exports` so that
older tooling and consumers that ignore `exports` continue to resolve the package.

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

## Verifying Module Consumption Modes from a Packed Consumer

To confirm both supported modes resolve from the published artifact, pack the SDK
and import it from a throwaway consumer in each mode. These commands are meant to
be run manually (or in a dedicated smoke job) — they are documented here rather
than executed as part of this change:

```sh
# 1. Produce the packed tarball
cd sdk/typescript
npm pack --pack-destination /tmp

# 2. CommonJS consumer
mkdir -p /tmp/cjs-consumer && cd /tmp/cjs-consumer
npm init -y
npm install /tmp/stellar-sdk-*.tgz
node -e "const sdk = require('@stellar/stellar-sdk'); if (!sdk) process.exit(1); console.log('CJS ok')"

# 3. ESM consumer
mkdir -p /tmp/esm-consumer && cd /tmp/esm-consumer
npm init -y
npm pkg set type=module
npm install /tmp/stellar-sdk-*.tgz
node --input-type=module -e "import * as sdk from '@stellar/stellar-sdk'; if (!sdk) process.exit(1); console.log('ESM ok')"
```

Both commands must exit successfully for the module contract to be considered
satisfied. If either mode fails, the `exports` map in `sdk/typescript/package.json`
must be corrected before release.
