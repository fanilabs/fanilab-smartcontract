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
