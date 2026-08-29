# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :x:                |

## Reporting a Vulnerability

**Please DO NOT open public GitHub issues for security vulnerabilities.**

### How to Report

Report vulnerabilities privately through GitHub's built-in private vulnerability
reporting, which does not depend on any external mail server:

1. Open the [**Security Advisories**](https://github.com/fanilabs/fanilab-smartcontract/security/advisories/new) page for this repository.
2. Click **Report a vulnerability**.
3. Fill in the advisory form and **Include**:
   - Detailed description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

This opens a private thread visible only to you and the maintainers; no special
repository permissions are required to submit a report.

If you cannot use GitHub's private reporting, open a public issue titled
`[SECURITY] Private contact request` that contains **no vulnerability details**
and a maintainer will follow up with a private channel.

### Response Timeline
- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: 1-3 days
  - High: 1-2 weeks
  - Medium: 2-4 weeks
  - Low: Next release cycle

### Disclosure Policy
- We practice coordinated disclosure
- We will work with you to understand and fix the issue
- Public disclosure after fix is deployed (typically 90 days)
- You will be credited in our security advisories (if desired)

## Bug Bounty Program

### Scope
- All production smart contracts on Mainnet
- Testnet contracts for design flaws only

### Rewards
- **Critical**: $10,000 - $50,000 (funds at risk)
- **High**: $5,000 - $10,000 (significant impact)
- **Medium**: $1,000 - $5,000 (moderate impact)
- **Low**: $100 - $1,000 (minimal impact)

### Out of Scope
- Issues in third-party dependencies
- Known issues already reported
- Theoretical vulnerabilities without proof of concept
- Social engineering attacks
- DoS attacks on public endpoints

### Rules
- Be respectful and professional
- Do not publicly disclose before fix
- Do not exploit vulnerabilities
- Provide clear reproduction steps
- One bounty per unique vulnerability

## Security Best Practices

### For Users
1. Never share your private keys
2. Verify contract addresses before interacting
3. Use hardware wallets for large amounts
4. Monitor your transactions
5. Report suspicious activity

### For Developers
1. Read our [Security Audit Checklist](docs/SECURITY_AUDIT.md)
2. Follow Stellar security guidelines
3. Review all PRs for security implications
4. Keep dependencies updated
5. Use static analysis tools

## Security Features

### Access Control
- Two-step admin transfer
- Per-function authorization checks
- No hidden backdoors
- All privileged operations emit events

### Financial Security
- Checks-effects-interactions pattern
- Balance verification before transfers
- Saturating math to prevent overflow
- Escrow isolation

### State Management
- Comprehensive state transition validation
- TTL management for all storage
- No orphaned state possible
- Atomic operations

### Release & Supply-Chain Integrity
- **Least-privilege CI.** All four GitHub Actions workflows declare explicit
  `permissions` blocks: the release job gets `contents: write` (the minimum
  `softprops/action-gh-release` needs to create a release and upload assets),
  every other job gets `contents: read`. This keeps releases working under a
  read-only default token and bounds what a compromised third-party action can
  reach.
- **Locked dependency resolution.** Every `cargo` invocation across CI, release,
  and testnet deploy uses `--locked`, so published WASM artifacts and their
  recorded SHA256 checksums are built from exactly the dependency set the test
  suite validated, and rebuilding a tag is reproducible.
- **Version-consistency guard.** `release.yml` refuses to build or publish when
  the pushed `vX.Y.Z` tag disagrees with the version declared by the workspace
  crates (or when the crates disagree with each other).

### Version Identifiers
The project uses three distinct version numbers; they are intentionally not the
same thing:

| Identifier | Where | Meaning |
|------------|-------|---------|
| Crate / package version | `contracts/*/Cargo.toml` | Source + published-artifact history. All workspace crates share one value. |
| Release tag (`vX.Y.Z`) | git tags → GitHub Releases | The user-facing name of a published artifact set. Must equal the crate version (CI-enforced, minus the `v`). |
| `PROTOCOL_VERSION` | `escrow_contract::constants` | On-chain data/behaviour contract exposed via `get_protocol_version()`. Bumps only on an on-chain-observable protocol change, independently of crate releases. |

## Audit History

| Date | Auditor | Version | Report | Status |
|------|---------|---------|--------|--------|
| TBD  | TBD     | 1.0.0   | TBD    | Pending |

## Security Contacts

- **Private vulnerability reporting**: [GitHub Security Advisories](https://github.com/fanilabs/fanilab-smartcontract/security/advisories/new)
- **Discord**: FaniLab Official Server

## Additional Resources
- [Security Audit Checklist](docs/SECURITY_AUDIT.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [Testing Guide](docs/TESTING.md)
- [Stellar Security Best Practices](https://developers.stellar.org/docs/learn/security)

---

**Last Updated**: August 2026
