import fs from 'fs';
import path from 'path';

import { DeliveryStatus, EscrowStatus } from '../types/common.types';

const CONTRACT_TYPES_PATH = path.resolve(
  __dirname,
  '../../../../contracts/shared_types/lib.rs',
);

function readSource(): string {
  return fs.readFileSync(CONTRACT_TYPES_PATH, 'utf8');
}

function extractBlock(src: string, keyword: 'enum' | 'struct', name: string): string {
  const header = src.indexOf(`pub ${keyword} ${name}`);
  if (header === -1) {
    throw new Error(`Contract type \`${name}\` not found in shared_types/lib.rs`);
  }
  const open = src.indexOf('{', header);
  let depth = 1;
  let cursor = open + 1;
  while (depth > 0 && cursor < src.length) {
    if (src[cursor] === '{') {
      depth += 1;
    } else if (src[cursor] === '}') {
      depth -= 1;
    }
    cursor += 1;
  }
  return src.slice(open + 1, cursor - 1);
}

function unitEnumVariants(src: string, name: string): string[] {
  return extractBlock(src, 'enum', name)
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith('///') && !line.startsWith('#'))
    .map((line) => line.replace(/[,].*$/, '').trim())
    .map((line) => line.split(/\s+/)[0])
    .filter((variant) => /^[A-Z][A-Za-z0-9_]*$/.test(variant));
}

function expectExactParity(sdkVariants: string[], contractVariants: string[]): void {
  expect([...new Set(sdkVariants)].sort()).toEqual([...new Set(contractVariants)].sort());
}

describe('SDK type parity with contracts/shared_types', () => {
  const src = readSource();

  test('EscrowStatus matches every EscrowState variant defined by the contract', () => {
    const contractVariants = unitEnumVariants(src, 'EscrowState');
    expect(contractVariants).toEqual([
      'Locked',
      'Holdback',
      'Released',
      'Refunded',
      'Paused',
      'Split',
    ]);
    expectExactParity(Object.values(EscrowStatus), contractVariants);
  });

  test('EscrowStatus exposes Paused so disputed escrows are representable', () => {
    expect(Object.values(EscrowStatus)).toContain('Paused');
  });

  test('DeliveryStatus matches every DeliveryStatus variant defined by the contract', () => {
    const contractVariants = unitEnumVariants(src, 'DeliveryStatus');
    expect(contractVariants).toEqual([
      'Pending',
      'Active',
      'InTransit',
      'Delivered',
      'Disputed',
      'Cancelled',
    ]);
    expectExactParity(Object.values(DeliveryStatus), contractVariants);
  });

  test('parity test itself fails when a variant is dropped from the SDK enum', () => {
    const sdkVariants = Object.values(EscrowStatus).filter((v) => v !== 'Paused');
    expect(() => expectExactParity(sdkVariants, unitEnumVariants(src, 'EscrowState'))).toThrow();
  });
});