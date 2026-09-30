import { Address, nativeToScVal, scValToNative, xdr } from '@stellar/stellar-sdk';

import { EscrowClient } from '../clients/escrow.client';
import { ContractInvoker } from '../clients/invoker';

/**
 * The escrow client's `ContractInvoker` is the single seam through which every
 * binding reaches the chain, so a test double at that level both exercises the
 * real argument encoding in `clients/invoker.ts` and asserts the exact
 * contract function name each binding targets.
 *
 * Return values are queued per contract function as raw ScVals and decoded the
 * same way `ContractInvoker.call` decodes them, so the client sees the same
 * native shape it would see on-chain.
 */
function mockInvoker(retvals: Record<string, xdr.ScVal[]>): jest.SpyInstance {
  const call = jest.fn(async (functionName: string, args: xdr.ScVal[]) => {
    const queue = retvals[functionName];
    if (!queue || queue.length === 0) {
      return undefined;
    }
    const value = queue.length === 1 ? queue[0] : queue.shift();
    return value === undefined ? undefined : scValToNative(value as xdr.ScVal);
  });
  jest.spyOn(ContractInvoker.prototype, 'call').mockImplementation(call as never);
  return call;
}

// Deterministic placeholder addresses — the invoker is mocked, so only the
// address *encoding* matters, but they must be structurally valid so the
// `Contract`/`Address` constructors in clients/invoker.ts accept them.
const CONTRACT_ID = 'CADQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQP5KR';
const TOKEN = 'GAEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSH7S';
const ADMIN = 'GAEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSH7S';
const TREASURY = 'GAEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSH7S';

function client(): EscrowClient {
  return new EscrowClient(CONTRACT_ID);
}

/** Read a u64 ScVal back as a bigint so encoding can be asserted exactly. */
function decodeU64(value: xdr.ScVal): bigint {
  return scValToNative(value) as bigint;
}

function decodeAddress(value: xdr.ScVal): string {
  return scValToNative(value) as string;
}

afterEach(() => {
  jest.restoreAllMocks();
});

describe('EscrowClient solvency and sweep bindings (issue #450)', () => {
  it('getTotalLocked reads get_total_locked and decodes an i128', async () => {
    const call = mockInvoker({ get_total_locked: [nativeToScVal(1_000_000n, { type: 'i128' })] });

    const total = await client().getTotalLocked(TOKEN);

    expect(total).toBe(1_000_000n);
    expect(call).toHaveBeenCalledTimes(1);
    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('get_total_locked');
    expect(args).toHaveLength(1);
    expect(decodeAddress(args[0] as xdr.ScVal)).toBe(TOKEN);
  });

  it('getTotalLocked surfaces a zero balance as 0n', async () => {
    mockInvoker({ get_total_locked: [nativeToScVal(0n, { type: 'i128' })] });

    await expect(client().getTotalLocked(TOKEN)).resolves.toBe(0n);
  });

  it('getUntrackedBalance reads get_untracked_balance', async () => {
    const call = mockInvoker({ get_untracked_balance: [nativeToScVal(25_000n, { type: 'i128' })] });

    const untracked = await client().getUntrackedBalance(TOKEN);

    expect(untracked).toBe(25_000n);
    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('get_untracked_balance');
    expect(decodeAddress(args[0] as xdr.ScVal)).toBe(TOKEN);
  });

  it('sweepUntrackedBalance targets sweep_untracked_balance with admin, token and recipient', async () => {
    const call = mockInvoker({ sweep_untracked_balance: [nativeToScVal(25_000n, { type: 'i128' })] });

    const swept = await client().sweepUntrackedBalance({
      admin: ADMIN,
      token: TOKEN,
      recipient: TREASURY,
    });

    expect(swept).toBe(25_000n);
    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('sweep_untracked_balance');
    expect(args).toHaveLength(3);
    expect(decodeAddress(args[0] as xdr.ScVal)).toBe(ADMIN);
    expect(decodeAddress(args[1] as xdr.ScVal)).toBe(TOKEN);
    expect(decodeAddress(args[2] as xdr.ScVal)).toBe(TREASURY);
  });

  it('sweepUntrackedBalance returns 0n when there is nothing to sweep', async () => {
    mockInvoker({ sweep_untracked_balance: [nativeToScVal(0n, { type: 'i128' })] });

    const swept = await client().sweepUntrackedBalance({
      admin: ADMIN,
      token: TOKEN,
      recipient: TREASURY,
    });

    expect(swept).toBe(0n);
  });

  /**
   * Full operator cycle: measure the untracked balance, sweep it to treasury,
   * then re-read to confirm the contract is fully accounted for.
   */
  it('completes the measure, sweep and verify fund recovery cycle', async () => {
    const call = mockInvoker({
      get_total_locked: [nativeToScVal(500n, { type: 'i128' })],
      // Balance - total_locked, exactly as the contract computes it.
      get_untracked_balance: [
        nativeToScVal(40n, { type: 'i128' }),
        nativeToScVal(0n, { type: 'i128' }),
      ],
      sweep_untracked_balance: [nativeToScVal(40n, { type: 'i128' })],
    });

    const escrow = client();
    const locked = await escrow.getTotalLocked(TOKEN);
    const before = await escrow.getUntrackedBalance(TOKEN);
    expect(locked).toBe(500n);
    expect(before).toBe(40n);

    const swept = await escrow.sweepUntrackedBalance({
      admin: ADMIN,
      token: TOKEN,
      recipient: TREASURY,
    });
    expect(swept).toBe(before);

    // After the sweep the contract holds nothing beyond its tracked escrows.
    await expect(escrow.getUntrackedBalance(TOKEN)).resolves.toBe(0n);

    expect(call.mock.calls.map((c) => c[0])).toEqual([
      'get_total_locked',
      'get_untracked_balance',
      'sweep_untracked_balance',
      'get_untracked_balance',
    ]);
  });
});

describe('EscrowClient releaseExpiredHoldback binding (issue #452)', () => {
  it('targets release_expired_holdback with a u64 deliveryId', async () => {
    const call = mockInvoker({ release_expired_holdback: [undefined] });

    await client().releaseExpiredHoldback({ deliveryId: 4242n });

    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('release_expired_holdback');
    expect(args).toHaveLength(1);
    expect(decodeU64(args[0] as xdr.ScVal)).toBe(4242n);
  });

  it('propagates a reverted holdback release to the caller', async () => {
    const call = jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockRejectedValue(new Error('Soroban simulation failed: EscrowError::TimelockNotElapsed'));

    await expect(client().releaseExpiredHoldback({ deliveryId: 7n })).rejects.toThrow(
      'TimelockNotElapsed',
    );
    expect(call).toHaveBeenCalledWith('release_expired_holdback', expect.anything(), undefined);
  });
});

// ── Issue #485: getPendingSettlementContract and confirmSettlementContract ────

describe('EscrowClient settlement contract timelock lifecycle (Issue #485)', () => {
  const SETTLEMENT = 'GAEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSH7S';
  const ACTIVATES_AT = 1_800_000_000n; // a plausible future Unix timestamp

  /**
   * Step 1 — proposal:
   * `setSettlementContract` encodes and dispatches the on-chain proposal that
   * starts the three-day timelock. Confirms the correct function name and args.
   */
  it('setSettlementContract calls set_settlement_contract with (admin, address) args', async () => {
    const call = mockInvoker({ set_settlement_contract: [nativeToScVal(null)] });

    await client().setSettlementContract({ admin: ADMIN, settlementContract: SETTLEMENT });

    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('set_settlement_contract');
    expect(args).toHaveLength(2);
    expect(decodeAddress(args[0] as xdr.ScVal)).toBe(ADMIN);
    expect(decodeAddress(args[1] as xdr.ScVal)).toBe(SETTLEMENT);
  });

  /**
   * Step 2 — inspection during the timelock window:
   * `getPendingSettlementContract` reads the pending proposal so frontends can
   * display the upcoming routing change and its activation timestamp.
   * Returns `null` when no change is pending.
   */
  it('getPendingSettlementContract reads the pending proposal from the contract', async () => {
    const pending = nativeToScVal({
      settlement_contract: SETTLEMENT,
      activates_at: ACTIVATES_AT,
    });
    const call = mockInvoker({ get_pending_settlement_contract: [pending] });

    const result = await client().getPendingSettlementContract();

    expect(result).not.toBeNull();
    expect(result!.settlementContract).toBe(SETTLEMENT);
    expect(result!.activatesAt).toBe(ACTIVATES_AT);
    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('get_pending_settlement_contract');
    expect(args).toHaveLength(0);
  });

  it('getPendingSettlementContract returns null when no proposal is in-flight', async () => {
    mockInvoker({ get_pending_settlement_contract: [nativeToScVal(null)] });

    const result = await client().getPendingSettlementContract();

    expect(result).toBeNull();
  });

  it('getPendingSettlementContract returns null when the invoker resolves undefined', async () => {
    jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockResolvedValue(undefined as never);

    const result = await client().getPendingSettlementContract();

    expect(result).toBeNull();
  });

  /**
   * Step 3 — confirmation after the timelock elapses:
   * `confirmSettlementContract` applies the pending change. Only an admin may
   * call it; the contract will reject the call if the timelock has not yet
   * elapsed. This test confirms the SDK binding targets the right function and
   * passes only the admin address (the contract derives the rest from storage).
   */
  it('confirmSettlementContract calls confirm_settlement_contract with only the admin arg', async () => {
    const call = mockInvoker({ confirm_settlement_contract: [nativeToScVal(null)] });

    await client().confirmSettlementContract({ admin: ADMIN });

    const [fn, args] = call.mock.calls[0];
    expect(fn).toBe('confirm_settlement_contract');
    expect(args).toHaveLength(1);
    expect(decodeAddress(args[0] as xdr.ScVal)).toBe(ADMIN);
  });

  it('confirmSettlementContract rejects TimelockNotElapsed from the contract', async () => {
    jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockRejectedValue(
        new Error('Soroban simulation failed: EscrowError::TimelockNotElapsed'),
      );

    await expect(client().confirmSettlementContract({ admin: ADMIN })).rejects.toThrow(
      'TimelockNotElapsed',
    );
  });

  it('confirmSettlementContract rejects NoPendingSettlementChange from the contract', async () => {
    jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockRejectedValue(
        new Error('Soroban simulation failed: EscrowError::NoPendingSettlementChange'),
      );

    await expect(client().confirmSettlementContract({ admin: ADMIN })).rejects.toThrow(
      'NoPendingSettlementChange',
    );
  });

  /**
   * Full two-step lifecycle — the primary acceptance criterion for Issue #485.
   *
   * Sequence:
   *   1. Admin proposes a new settlement contract → timelock starts.
   *   2. Pending state is visible via `getPendingSettlementContract`.
   *   3. After the timelock, admin confirms → change applied on-chain.
   *   4. `getPendingSettlementContract` returns null (proposal consumed).
   *   5. `getSettlementContract` returns the new address.
   *
   * The test is pure SDK / mock — no live node needed — but it exercises the
   * full call sequence an administrator performs, verifying that every binding
   * targets the correct contract function and produces the correct arg encoding.
   */
  it('full two-step timelock lifecycle: propose → inspect → confirm → verify', async () => {
    const call = mockInvoker({
      set_settlement_contract: [nativeToScVal(null)],
      get_pending_settlement_contract: [
        // First read: proposal in-flight
        nativeToScVal({ settlement_contract: SETTLEMENT, activates_at: ACTIVATES_AT }),
        // Second read: proposal consumed after confirmation
        nativeToScVal(null),
      ],
      confirm_settlement_contract: [nativeToScVal(null)],
      get_settlement_contract: [nativeToScVal(SETTLEMENT, { type: 'address' })],
    });

    const escrow = client();

    // Step 1: propose
    await escrow.setSettlementContract({ admin: ADMIN, settlementContract: SETTLEMENT });

    // Step 2: inspect — change is pending, not yet active
    const pending = await escrow.getPendingSettlementContract();
    expect(pending).not.toBeNull();
    expect(pending!.settlementContract).toBe(SETTLEMENT);
    expect(pending!.activatesAt).toBe(ACTIVATES_AT);

    // Step 3: confirm (simulates time advancing past the timelock on-chain)
    await escrow.confirmSettlementContract({ admin: ADMIN });

    // Step 4: pending entry is now consumed — no further proposal in-flight
    const pendingAfter = await escrow.getPendingSettlementContract();
    expect(pendingAfter).toBeNull();

    // Step 5: the active settlement contract now resolves to the new address
    const active = await escrow.getSettlementContract();
    expect(active).toBe(SETTLEMENT);

    // Confirm the exact sequence of on-chain calls
    expect(call.mock.calls.map((c) => c[0])).toEqual([
      'set_settlement_contract',
      'get_pending_settlement_contract',
      'confirm_settlement_contract',
      'get_pending_settlement_contract',
      'get_settlement_contract',
    ]);
  });
});
