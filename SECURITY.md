# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :x:                |

## Reporting a Vulnerability

**Please DO NOT open public GitHub issues for security vulnerabilities.**

### For Critical Vulnerabilities
If you discover a critical security vulnerability:

1. **Email**: security@fanilab.com
2. **Subject**: [SECURITY] Brief description
3. **Include**:
   - Detailed description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

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

## Audit History

| Date | Auditor | Version | Report | Status |
|------|---------|---------|--------|--------|
| TBD  | TBD     | 1.0.0   | TBD    | Pending |

## Security Contacts

- **Email**: security@fanilab.com
- **PGP Key**: [Link to PGP key]
- **Discord**: FaniLab Official Server

## Additional Resources
- [Security Audit Checklist](docs/SECURITY_AUDIT.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [Testing Guide](docs/TESTING.md)
- [Stellar Security Best Practices](https://developers.stellar.org/docs/learn/security)

---

**Last Updated**: January 2026
