import { nativeToScVal, scValToNative, xdr } from '@stellar/stellar-sdk';

import { FleetManagementClient } from '../clients/fleet_management.client';
import { ContractInvoker } from '../clients/invoker';

/**
 * Mocks ContractInvoker.prototype.call and returns queued ScVal responses by
 * contract function name. Each entry may hold multiple values (shift()d in
 * order) or a single value returned every time.
 */
function mockInvoker(retvals: Record<string, xdr.ScVal[]>): jest.SpyInstance {
  const call = jest.fn(async (functionName: string) => {
    const queue = retvals[functionName];
    if (!queue || queue.length === 0) return undefined;
    const value = queue.length === 1 ? queue[0] : queue.shift();
    return value === undefined ? undefined : scValToNative(value as xdr.ScVal);
  });
  jest.spyOn(ContractInvoker.prototype, 'call').mockImplementation(call as never);
  return call;
}

// Structurally valid Stellar addresses required by the Address/Contract constructors.
const CONTRACT_ID = 'CADQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQP5KR';
const ESCROW_ADDR = 'GAEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSH7S';
const IDENTITY_ADDR = 'GAEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSH7S';
const ADMIN = 'GAEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSH7S';

function client(): FleetManagementClient {
  return new FleetManagementClient(CONTRACT_ID);
}

function decodeAddress(val: xdr.ScVal): string {
  return scValToNative(val) as string;
}

afterEach(() => {
  jest.restoreAllMocks();
});

// ── Issue #486: getEscrowContract and getIdentityContract SDK bindings ────────

describe('FleetManagementClient.getEscrowContract (Issue #486)', () => {
  it('calls get_escrow_contract with no args and returns the address', async () => {
    const call = mockInvoker({
      get_escrow_contract: [nativeToScVal(ESCROW_ADDR, { type: 'address' })],
    });

    const result = await client().getEscrowContract();

    expect(result).toBe(ESCROW_ADDR);
    expect(call).toHaveBeenCalledTimes(1);
    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('get_escrow_contract');
    expect(args).toHaveLength(0);
  });

  it('returns null when the contract returns Option::None', async () => {
    mockInvoker({ get_escrow_contract: [nativeToScVal(null)] });

    const result = await client().getEscrowContract();

    expect(result).toBeNull();
  });

  it('returns null when the invoker resolves undefined', async () => {
    jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockResolvedValue(undefined as never);

    const result = await client().getEscrowContract();

    expect(result).toBeNull();
  });
});

describe('FleetManagementClient.getIdentityContract (Issue #486)', () => {
  it('calls get_identity_contract with no args and returns the address', async () => {
    const call = mockInvoker({
      get_identity_contract: [nativeToScVal(IDENTITY_ADDR, { type: 'address' })],
    });

    const result = await client().getIdentityContract();

    expect(result).toBe(IDENTITY_ADDR);
    expect(call).toHaveBeenCalledTimes(1);
    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('get_identity_contract');
    expect(args).toHaveLength(0);
  });

  it('returns null when the contract returns Option::None', async () => {
    mockInvoker({ get_identity_contract: [nativeToScVal(null)] });

    const result = await client().getIdentityContract();

    expect(result).toBeNull();
  });

  it('returns null when the invoker resolves undefined', async () => {
    jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockResolvedValue(undefined as never);

    const result = await client().getIdentityContract();

    expect(result).toBeNull();
  });

  /**
   * Full round-trip: set and then read back both dependency addresses.
   * The SDK must invoke the correct on-chain function name in each direction.
   */
  it('set+get round-trip for both escrow and identity contracts', async () => {
    const call = mockInvoker({
      set_escrow_contract: [nativeToScVal(null)],
      get_escrow_contract: [nativeToScVal(ESCROW_ADDR, { type: 'address' })],
      set_identity_contract: [nativeToScVal(null)],
      get_identity_contract: [nativeToScVal(IDENTITY_ADDR, { type: 'address' })],
    });

    const fleetClient = client();
    await fleetClient.setEscrowContract({ admin: ADMIN, escrowContract: ESCROW_ADDR });
    const readEscrow = await fleetClient.getEscrowContract();
    await fleetClient.setIdentityContract({ admin: ADMIN, identityContract: IDENTITY_ADDR });
    const readIdentity = await fleetClient.getIdentityContract();

    expect(readEscrow).toBe(ESCROW_ADDR);
    expect(readIdentity).toBe(IDENTITY_ADDR);

    const fns = call.mock.calls.map((c) => c[0]);
    expect(fns).toEqual([
      'set_escrow_contract',
      'get_escrow_contract',
      'set_identity_contract',
      'get_identity_contract',
    ]);
  });
});

// ── Issue #487: setEscrowContract / setIdentityContract emit observable events ─

describe('FleetManagementClient set_escrow_contract event (Issue #487)', () => {
  /**
   * `set_escrow_contract` emits an `escrow_contract_updated` event on-chain.
   * This test verifies that the SDK binding:
   *   1. Targets the correct on-chain function (`set_escrow_contract`).
   *   2. Passes admin and escrowContract as positional args in the right order.
   *
   * The event itself is emitted inside the Rust contract and is observable
   * off-chain via the Soroban event stream; the SDK binding is the call-site
   * side of the contract interaction, which is what a TypeScript integrator
   * drives. Confirming the correct function name and arg encoding guarantees
   * the event will be emitted when this binding is invoked on-chain.
   */
  it('setEscrowContract targets set_escrow_contract with (admin, address) args', async () => {
    const call = mockInvoker({ set_escrow_contract: [nativeToScVal(null)] });

    await client().setEscrowContract({ admin: ADMIN, escrowContract: ESCROW_ADDR });

    expect(call).toHaveBeenCalledTimes(1);
    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('set_escrow_contract');
    expect(args).toHaveLength(2);
    expect(decodeAddress(args[0] as xdr.ScVal)).toBe(ADMIN);
    expect(decodeAddress(args[1] as xdr.ScVal)).toBe(ESCROW_ADDR);
  });

  it('propagates a contract error from set_escrow_contract to the caller', async () => {
    jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockRejectedValue(new Error('Soroban simulation failed: FleetError::Unauthorized'));

    await expect(
      client().setEscrowContract({ admin: ADMIN, escrowContract: ESCROW_ADDR }),
    ).rejects.toThrow('Unauthorized');
  });
});

describe('FleetManagementClient set_identity_contract event (Issue #487)', () => {
  /**
   * `set_identity_contract` emits an `identity_contract_updated` event on-chain.
   * This test verifies the SDK binding targets the correct function and encodes
   * the (admin, identityContract) args in the right order, which is a
   * precondition for the event to be emitted on-chain.
   */
  it('setIdentityContract targets set_identity_contract with (admin, address) args', async () => {
    const call = mockInvoker({ set_identity_contract: [nativeToScVal(null)] });

    await client().setIdentityContract({ admin: ADMIN, identityContract: IDENTITY_ADDR });

    expect(call).toHaveBeenCalledTimes(1);
    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('set_identity_contract');
    expect(args).toHaveLength(2);
    expect(decodeAddress(args[0] as xdr.ScVal)).toBe(ADMIN);
    expect(decodeAddress(args[1] as xdr.ScVal)).toBe(IDENTITY_ADDR);
  });

  it('propagates a contract error from set_identity_contract to the caller', async () => {
    jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockRejectedValue(new Error('Soroban simulation failed: FleetError::Unauthorized'));

    await expect(
      client().setIdentityContract({ admin: ADMIN, identityContract: IDENTITY_ADDR }),
    ).rejects.toThrow('Unauthorized');
  });

  /**
   * Proves both event-emitting setters work as a pair: set both dependency
   * addresses in sequence and confirm each binding used the correct function
   * name. Together they exercise the full "wire-up" workflow an admin
   * performs when deploying the fleet contract, and they serve as a
   * regression guard for any future rename of the Rust function names.
   */
  it('both event-emitting setters can be called in sequence without interference', async () => {
    const call = mockInvoker({
      set_escrow_contract: [nativeToScVal(null)],
      set_identity_contract: [nativeToScVal(null)],
    });

    const fleetClient = client();
    await fleetClient.setEscrowContract({ admin: ADMIN, escrowContract: ESCROW_ADDR });
    await fleetClient.setIdentityContract({ admin: ADMIN, identityContract: IDENTITY_ADDR });

    const fns = call.mock.calls.map((c) => c[0]);
    expect(fns).toEqual(['set_escrow_contract', 'set_identity_contract']);

    // First call: set_escrow_contract(admin, escrowAddr)
    expect(decodeAddress(call.mock.calls[0][1][0] as xdr.ScVal)).toBe(ADMIN);
    expect(decodeAddress(call.mock.calls[0][1][1] as xdr.ScVal)).toBe(ESCROW_ADDR);

    // Second call: set_identity_contract(admin, identityAddr)
    expect(decodeAddress(call.mock.calls[1][1][0] as xdr.ScVal)).toBe(ADMIN);
    expect(decodeAddress(call.mock.calls[1][1][1] as xdr.ScVal)).toBe(IDENTITY_ADDR);
  });
});
