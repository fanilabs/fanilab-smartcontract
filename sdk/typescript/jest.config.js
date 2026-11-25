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
        // Transpile-only. Type checking is decoupled from test execution and
        // enforced as an independent CI check via `npx tsc --noEmit`.
        diagnostics: false,
      },
    ],
  },
  // Collect coverage for the SDK source. Exclusions are defined explicitly so
  // the published report reflects only executable client/transport code.
  collectCoverage: true,
  collectCoverageFrom: [
    'src/**/*.ts',
    '!src/**/*.test.ts',
    '!src/**/__tests__/**',
    '!src/**/*.d.ts',
  ],
  coveragePathIgnorePatterns: [
    '/node_modules/',
    '/dist/',
    '/coverage/',
    '\\.test\\.ts$',
    '/__tests__/',
  ],
  coverageDirectory: 'coverage',
  coverageReporters: ['lcov', 'text-summary'],
  // @stellar/stellar-sdk requires an ESM-only dependency (uint8array-extras),
  // so tests must run under `node --experimental-vm-modules` (see `npm test`).
};