# Contributing

Thanks for your interest in improving this project. This document covers how to
report problems, propose changes, and get your work merged.

## Reporting security vulnerabilities

**Please do not open public issues for security vulnerabilities.**

This project uses [GitHub private security advisories](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
as the sole supported channel for security reports:

1. Go to the repository's **Security** tab.
2. Click **Report a vulnerability** to open a private advisory.
3. Fill in the affected versions and any reproduction details you can share.

GitHub private advisories are the only supported reporting channel; there is no
separate email or chat address for security reports. If you are unsure whether
something is a vulnerability, report it privately anyway and we will triage it.

### What to expect

- **Acknowledgment:** we aim to acknowledge new advisories within a few business
days.
- **Triage:** we review and confirm (or dismiss) reports as quickly as we can;
  complex reports may take longer.
- **Updates:** we respond in the advisory thread. If you have not heard back and
  want to check in, add a comment to the advisory rather than opening a public
  issue.

Please keep technical details inside the private advisory until a fix has been
released and coordinated disclosure has been agreed.

## Reporting bugs and requesting features

For non-security issues, open a GitHub issue with a clear description, steps to
reproduce, and the expected versus actual behavior.

## Proposing changes

1. Fork the repository and create a topic branch.
2. Keep changes focused; avoid unrelated refactors in the same pull request.
3. Describe the motivation and the change in the pull request body.
4. Make sure existing tests and checks pass before requesting review.

## TypeScript SDK checks

The SDK runs TypeScript in strict mode with `noUnusedLocals` and
`noUnusedParameters` enabled. Run the checks from `sdk/typescript`:

```sh
npm run typecheck
npm test
npm run build
```

When a parameter must stay in a signature for compatibility but is
intentionally unused, prefix it with an underscore (for example `_args`).
TypeScript ignores underscore-prefixed parameters for unused checks, so this
is the supported way to document intent rather than deleting the parameter.

## Questions

If anything here is unclear, open a regular issue and a maintainer will help.
