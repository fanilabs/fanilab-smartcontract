# Contract Deployment

This document describes how contracts are deployed and how the deployed
artifacts are recorded so that each on-chain contract ID can be independently
verified against the exact WASM that was deployed.

## Deployment

Contracts are deployed with `scripts/deploy-contract.sh`. The script builds the
WASM artifact, deploys it, and records the resulting contract ID in the
deployment manifest.

## Deployment manifest

For every deployment the manifest records, alongside the contract ID:

- `wasm_sha256` — the SHA-256 hash of the WASM artifact that was deployed.
- `source_revision` — the git commit SHA of the source tree the artifact was
  built from.

These fields are written next to the contract ID so the artifact association is
stored with the deployment itself.

## Verifying a deployment

The artifact association is independently verifiable:

1. Rebuild the WASM from the recorded `source_revision` (check out that commit
   and run the build).
2. Compute the SHA-256 of the rebuilt artifact, e.g.
   `sha256sum path/to/contract.wasm`.
3. Compare it with the `wasm_sha256` recorded in the manifest for the contract
   ID. A match confirms the deployed contract corresponds to that source
   revision.

Because both the hash and the source revision are stored with the contract ID,
anyone can reproduce and check the association without trusting the deployer.
