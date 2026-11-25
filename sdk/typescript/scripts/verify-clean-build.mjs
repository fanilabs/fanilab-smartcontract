#!/usr/bin/env node

/**
 * Verify a rebuild removes distributable artifacts left behind by source
 * modules that no longer exist (build-delete-rebuild sequence).
 *
 * The script builds once, plants files in `dist/` as if their source modules
 * had since been deleted, then rebuilds and asserts the stale files are gone.
 */
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const packageDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const distDir = path.join(packageDir, 'dist');
const staleArtifacts = ['stale-module.js', 'stale-module.d.ts'];

function build() {
  const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
  execFileSync(npm, ['run', 'build', '--silent'], {
    cwd: packageDir,
    stdio: 'inherit',
  });
}

build();

mkdirSync(distDir, { recursive: true });
for (const artifact of staleArtifacts) {
  writeFileSync(path.join(distDir, artifact), '// stale generated artifact\n');
}

build();

const leftovers = staleArtifacts.filter((artifact) =>
  existsSync(path.join(distDir, artifact))
);
if (leftovers.length > 0) {
  console.error(
    `❌ Rebuild left stale artifacts in dist/: ${leftovers.join(', ')}`
  );
  process.exit(1);
}

console.log('✅ Rebuild removed stale distributable artifacts.');
