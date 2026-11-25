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

## Minimum supported Rust version (MSRV)

The workspace declares a minimum supported Rust version (MSRV) via the
`rust-version` field in the root `Cargo.toml`. All workspace crates inherit this
value, so the minimum compiler contract is defined in exactly one place.

- **CI enforcement:** the CI workflow checks the MSRV with the pinned toolchain,
  so a dependency update that silently raises the required compiler will fail
  the build rather than land unnoticed.
- **Raising the MSRV:** raising the minimum is a deliberate, reviewed change.
  Open a dedicated pull request that only bumps `rust-version` (and any
  dependency changes that require it), get maintainer approval, and add a
  changelog entry describing the new minimum and the reason for the bump. Do not
  fold an MSRV bump into an unrelated feature or refactor PR.

## Cargo publication policy

This repository is a WASM workspace built from path dependencies. The contract
crates and `shared_types` are **internal-only** and are marked `publish = false`
in their manifests. They are not published to crates.io.

Releases ship compiled WASM binaries through `.github/workflows/release.yml`.
If you add a new crate, keep it internal (`publish = false`) unless there is an
explicit, reviewed decision to publish it, in which case it must carry the
required publishable metadata and a versioned dependency policy.

## Local lint workflow

`make lint` runs the same target/feature combinations as CI: a host clippy pass
and a `wasm32v1-none` clippy pass. Passing only one of them locally does not
guarantee CI will pass, so run the full command before opening a pull request.

Before running the WASM lint pass for the first time, install the target:

```sh
rustup target add wasm32v1-none
```

The Unix (`Makefile`) and Windows (`Makefile.windows`) lint targets invoke the
same checks so behavior stays aligned across platforms.

## Questions

If anything here is unclear, open a regular issue and a maintainer will help.
