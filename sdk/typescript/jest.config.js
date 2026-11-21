/** @type {import('jest').Config} */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  // Do not set `passWithNoTests`. Jest defaults to failing when no test
  // suites are discovered, so a broken discovery configuration (e.g. a
  // missing/incorrect test root) surfaces as a red test job instead of a
  // false green. The normal suite under `src/**/*.test.ts` remains
  // discoverable via Jest's default `testMatch`.
  transform: {
    '^.+\\.tsx?$': [
      'ts-jest',
      {
        tsconfig: 'tsconfig.json',
        // Transpile-only. Type checking is enforced separately by
        // `npm run typecheck`; running it here would fail every test suite on
        // the pre-existing `SorobanRpc` drift in clients/invoker.ts.
        diagnostics: false,
      },
    ],
  },
  // @stellar/stellar-sdk requires an ESM-only dependency (uint8array-extras),
  // so tests must run under `node --experimental-vm-modules` (see `npm test`).
};
