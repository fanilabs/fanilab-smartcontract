# TypeScript SDK Dependency Policy

The TypeScript SDK keeps `@stellar/stellar-sdk` on the current compatible major
version until the SDK invocation client is migrated and verified against a newer
major. The existing `^11.0.0` range is therefore intentional rather than an
unreviewed stale pin.

The committed `sdk/typescript/package-lock.json` is the reproducibility source for
SDK installs. Dependabot monitors `/sdk/typescript` weekly and can update both the
manifest range and lockfile; CI should install with `npm ci` when SDK CI coverage
is enabled.