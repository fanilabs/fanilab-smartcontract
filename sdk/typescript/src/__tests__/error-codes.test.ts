import fs from 'fs';
import path from 'path';

import { ErrorCodes } from '../types/common.types';
import { DeliveryErrorCodes } from '../types/delivery.types';
import { EscrowErrorCodes } from '../types/escrow.types';
import { FleetErrorCodes } from '../types/fleet_management.types';

const CONTRACTS_ROOT = path.resolve(__dirname, '../../../../contracts');

/**
 * Issue #475 — the shared `ErrorCodes` map must include every variant of the
 * Rust `FaniLabError` enum.
 * Issue #476 — the SDK must expose the contract-specific error enums so
 * integrators need not hardcode raw integers.
 */
function rustVariants(file: string, enumName: string): Record<string, number> {
  const src = fs.readFileSync(path.join(CONTRACTS_ROOT, file), 'utf8');
  const header = src.indexOf(`pub enum ${enumName}`);
  if (header === -1) {
    throw new Error(`Enum \`${enumName}\` not found in ${file}`);
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
  const variants: Record<string, number> = {};
  for (const rawLine of src.slice(open + 1, cursor - 1).split('\n')) {
    const line = rawLine.trim();
    if (!line || line.startsWith('///') || line.startsWith('#')) {
      continue;
    }
    const match = line.match(/^([A-Z][A-Za-z0-9_]*)\s*=\s*(\d+)\s*,?$/);
    if (match) {
      variants[match[1]] = Number(match[2]);
    }
  }
  return variants;
}

describe('SDK error code parity with the Rust enums', () => {
  test('ErrorCodes mirrors every FaniLabError variant (Issue #475)', () => {
    expect({ ...ErrorCodes }).toEqual(
      rustVariants('shared_types/lib.rs', 'FaniLabError')
    );
  });

  test('ErrorCodes exposes LimitExceeded as 12 (Issue #475)', () => {
    expect(ErrorCodes.LimitExceeded).toBe(12);
  });

  test('EscrowErrorCodes mirrors EscrowError (Issue #476)', () => {
    expect({ ...EscrowErrorCodes }).toEqual(
      rustVariants('escrow_contract/lib.rs', 'EscrowError')
    );
    expect(EscrowErrorCodes.InvalidFee).toBe(5);
    expect(EscrowErrorCodes.BatchTooLarge).toBe(12);
  });

  test('DeliveryErrorCodes mirrors DeliveryError (Issue #476)', () => {
    expect({ ...DeliveryErrorCodes }).toEqual(
      rustVariants('delivery_contract/lib.rs', 'DeliveryError')
    );
    expect(DeliveryErrorCodes.BatchTooLarge).toBe(3);
  });

  test('FleetErrorCodes mirrors FleetError (Issue #476)', () => {
    expect({ ...FleetErrorCodes }).toEqual(
      rustVariants('fleet_management_contract/lib.rs', 'FleetError')
    );
    expect(FleetErrorCodes.FleetNotFound).toBe(4);
  });
});
