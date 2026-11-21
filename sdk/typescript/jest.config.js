/** @type {import('jest').Config} */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  passWithNoTests: true,
  transform: {
    '^.+\\.tsx?$': [
      'ts-jest',
      {
        tsconfig: 'tsconfig.json',
        // Transpile-only. Type checking is decoupled from test execution and
        // enforced as an independent CI check via `npx tsc --noEmit`.
        diagnostics: false,
      },
    ],
  },
  // @stellar/stellar-sdk requires an ESM-only dependency (uint8array-extras),
  // so tests must run under `node --experimental-vm-modules` (see `npm test`).
};
