# Deployment

This document describes how to deploy the project and how the initialization
scripts resolve their configuration.

## Configuration files

The initialization scripts (`scripts/initialize-contract.sh` and any companion
initialization script) load environment configuration from a `.env` file. By
default they use the repository `.env` located at the repository root.

### Supplying an external configuration file

You can point the scripts at an externally supplied configuration file instead
of the repository `.env`. This is useful for isolated environments, CI runners,
or per-environment configs kept outside the checkout.

Provide the path in one of two ways:

- **CLI flag:** `--config <path>` (also accepts `--config=<path>`)
- **Environment variable:** `CONFIG_FILE=<path>`

# 2. Notify stakeholders
# 3. Investigate issue
# 4. Deploy fix or restore from backup
```

### Contract Upgrade
```bash
# Via flag
./scripts/initialize-contract.sh --config "/path/with spaces/my.env"

# Via environment variable
CONFIG_FILE="/path/with spaces/my.env" ./scripts/initialize-contract.sh
```

### Precedence

Configuration path resolution follows this order (highest priority first):

1. Explicit `--config` CLI flag
2. `CONFIG_FILE` environment variable
3. Default repository `.env` (repository root)

If no explicit path is supplied, the scripts retain their original behavior and
use the repository `.env`. All path references are quoted, so paths containing
spaces are supported.

## Deployment artifact retention

The `.github/workflows/deploy-testnet.yml` workflow uploads the deployment
outputs (the `contract-ids-*.json` file and any deployment receipts) as a
workflow artifact. The upload step sets an explicit retention period via
`retention-days` so that deployment records are kept for a period aligned with
the operational recovery horizon rather than the repository default.

### Retention period

- The workflow uploads deployment records with `retention-days: 90`.
- This is the value configured on the artifact upload step in
  `.github/workflows/deploy-testnet.yml`. If you change it, update this document
  in the same change so the documented policy stays accurate.

### GitHub retention is bounded

GitHub artifact retention is capped by the repository/plan maximum and artifacts
are deleted automatically once the retention period elapses. The deployment
artifact is the workflow's only saved deployment output, so relying on GitHub
alone risks losing contract IDs and deployment evidence on a schedule unrelated
to your recovery needs.

### Archiving beyond GitHub retention

Operators must archive deployment records to durable storage **before** the
GitHub retention period expires. Recommended approaches:

- Copy the downloaded `contract-ids-*.json` (and any deployment receipts) to an
  internal object store or bucket managed by the team.
- Attach the records to a GitHub Release as a release asset for the deployed
  version, which is retained independently of workflow artifact expiry.
- Record the archive location (bucket path or release URL) in the deployment
  log or runbook so it can be found during incident recovery.

Download artifacts promptly after a deployment and archive them as part of the
deployment checklist.

### Keep signing credentials out of logs and artifacts

Logs and artifact contents must never contain signing credentials. The deployer
secret is only ever passed via stdin to `stellar keys add`; it is never written
to the workspace, echoed to logs, or included in uploaded artifacts. When
archiving deployment records, verify that the archived files contain only
contract IDs and deployment evidence, not secrets or keys.