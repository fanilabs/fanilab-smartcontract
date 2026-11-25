#!/usr/bin/env node

/**
 * Remove the compiled output directory.
 *
 * `tsc` does not delete JavaScript or declaration files whose source modules
 * were removed, so stale artifacts can survive incremental builds. This script
 * uses Node's built-in filesystem API so the clean step behaves the same on
 * Linux, macOS, and Windows without shelling out to `rm`/`rmdir`.
 */
import { rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const packageDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const distDir = path.join(packageDir, 'dist');

rmSync(distDir, { recursive: true, force: true });
