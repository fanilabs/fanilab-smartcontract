/**
 * Typed SDK client for EscrowContract
 */

import * as EscrowTypes from '../types/escrow.types';
import { nativeToScVal } from '@stellar/stellar-sdk';
import { EscrowRecord, ContractInvokeOptions } from '../types/common.types';
import { ContractInvoker, address, bool, i128, map, u32, u64, vec } from './invoker';

/**
 * Normalize an optional escrow field: treat `null` and `undefined` alike as
 * absent, and reject malformed present values with a field-specific error.
 */
function optionalField<T>(value: T | null | undefined, field: string, isValid: (v: T) => boolean): T | undefined {
  if (value === null || value === undefined) {
    return undefined;
  }
  if (!isValid(value)) {
    throw new Error(`Invalid escrow field \`${field}\`: ${String(value)}`);
  }
  return value;
}

/**
 * Raised when a deployment's protocol version cannot be reconciled with the
 * versions an application supports.
 *
 * `reason` distinguishes a deployment that reported a version the caller does
 * not support (`'unsupported'`) from one whose version could not be read at all
 * (`'unavailable'`).
 */
export class ProtocolCompatibilityError extends Error {
  readonly deployedVersion: number | undefined;
  readonly supportedVersions: readonly number[];
  readonly reason: 'unsupported' | 'unavailable';

  constructor(
    reason: 'unsupported' | 'unavailable',
    deployedVersion: number | undefined,
    supportedVersions: readonly number[]
  ) {
    const message = reason === 'unavailable'
      ? `Unable to determine the deployed protocol version; supported versions: ${formatVersions(supportedVersions)}`
      : `Deployed protocol version ${String(deployedVersion)} is not supported; supported versions: ${formatVersions(supportedVersions)}`;
    super(message);
    this.name = 'ProtocolCompatibilityError';
    this.reason = reason;
    this.deployedVersion = deployedVersion;
    this.supportedVersions = [...supportedVersions];
  }
}

function formatVersions(versions: readonly number[]): string {
  return versions.length === 0 ? '(none)' : versions.join(', ');
}

export class EscrowClient {
  private readonly invoker: ContractInvoker;

  constructor(contractId: string, options: ContractInvokeOptions = {}) {
    this.invoker = new ContractInvoker(contractId, options);
  }

  /**
   * Initialize the escrow contract with admin and platform fee configuration
   */
  async init(params: EscrowTypes.InitParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('init', [address(params.admin), address(params.token), u32(params.platformFeeBps)], options);
  }

  /**
   * Update the platform fee percentage
   */
  async updatePlatformFee(
    params: EscrowTypes.UpdatePlatformFeeParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('update_platform_fee', [address(params.admin), u32(params.newFeeBps)], options);
  }

  /**
   * Get the current platform fee in basis points
   */
  async getPlatformFee(options?: ContractInvokeOptions): Promise<number> {
    return Number(await this.invoker.call('get_platform_fee', [], options));
  }

  /**
   * Assert that the deployed contract's protocol version is one the caller
   * supports, before proceeding with writes.
   *
   * Performs a read-only `get_protocol_version` call (no mutation is
   * submitted) and resolves with the deployed version when it is included in
   * `supportedVersions`. Otherwise — including when the version cannot be read
   * or is unavailable — rejects with a `ProtocolCompatibilityError`.
   */
  async assertProtocolCompatibility(
    supportedVersions: readonly number[],
    options?: ContractInvokeOptions
  ): Promise<number> {
    let deployedVersion: number | undefined;
    try {
      const result = await this.invoker.call('get_protocol_version', [], options);
      if (result !== null && result !== undefined) {
        const parsed = Number(String(result));
        if (Number.isFinite(parsed)) {
          deployedVersion = parsed;
        }
      }
    } catch {
      throw new ProtocolCompatibilityError('unavailable', undefined, supportedVersions);
    }

    if (deployedVersion === undefined) {
      throw new ProtocolCompatibilityError('unavailable', undefined, supportedVersions);
    }
    if (!supportedVersions.includes(deployedVersion)) {
      throw new ProtocolCompatibilityError('unsupported', deployedVersion, supportedVersions);
    }
    return deployedVersion;
  }

  /**
   * Get the admin address
   */
  async getAdmin(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_admin', [], options));
  }

  /**
   * Get the token address used by this contract
   */
  async getToken(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_token', [], options));
  }

  /**
   * Get the total amount of a token currently locked in escrows
   */
  async getTotalLocked(token: string, options?: ContractInvokeOptions): Promise<bigint> {
    return BigInt(String(await this.invoker.call('get_total_locked', [address(token)], options)));
  }

  /**
   * Get the token balance held by the contract but not tracked as escrowed
   */
  async getUntrackedBalance(token: string, options?: ContractInvokeOptions): Promise<bigint> {
    return BigInt(String(await this.invoker.call('get_untracked_balance', [address(token)], options)));
  }

  /**
   * Sweep untracked token balance to a recipient (admin only)
   */
  async sweepUntrackedBalance(
    params: EscrowTypes.SweepUntrackedBalanceParams,
    options?: ContractInvokeOptions
  ): Promise<bigint> {
    return BigInt(String(await this.invoker.call(
      'sweep_untracked_balance',
      [address(params.admin), address(params.token), address(params.recipient)],
      options
    )));
  }

  /**
   * Set volume thresholds and fee discounts for high-volume senders (admin only)
   *
   * Volume thresholds are wide on-chain counters, so they are encoded as
   * `i128` (not `u32`) to preserve large values without rounding.
   */
  async setVolumeTiers(
    params: EscrowTypes.SetVolumeTiersParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    const tiers = vec(params.tiers.map((tier) =>
      map([
        ['volume_threshold', i128(tier.volumeThreshold)],
        ['discount_bps', u32(tier.discountBps)],
      ])
    ));
    await this.invoker.call('set_volume_tiers', [address(params.admin), tiers], options);
  }

  /**
   * Get configured sender volume thresholds and discounts
   *
   * Thresholds are returned as `bigint` so wide on-chain counters are not
   * rounded through `Number`.
   */
  async getVolumeTiers(options?: ContractInvokeOptions): Promise<EscrowTypes.VolumeTier[]> {
    const result = await this.invoker.call('get_volume_tiers', [], options);
    return (result as Array<Record<string, unknown>>).map((tier) => ({
      volumeThreshold: BigInt(String(tier.volume_threshold)),
      discountBps: Number(tier.discount_bps),
    }));
  }

  /**
   * Get a sender's accumulated volume
   *
   * Returned as `bigint` to preserve wide on-chain counters without rounding.
   */
  async getSenderVolume(sender: string, options?: ContractInvokeOptions): Promise<bigint> {
    return BigInt(String(await this.invoker.call('get_sender_volume', [address(sender)], options)));
  }

  /**
   * Create a new escrow for a delivery
   */
  async createEscrow(params: EscrowTypes.CreateEscrowParams, options?: ContractInvokeOptions): Promise<bigint> {
    const fleetId = optionalField(params.fleetId, 'fleetId', (v) => typeof v === 'bigint' || typeof v === 'number' || typeof v === 'string');
    const result = await this.invoker.call('create_escrow', [address(params.sender), address(params.recipient), address(params.driver), u64(params.deliveryId), address(params.token), i128(params.amount), fleetId === undefined ? xdrVoid() : u64(fleetId)], options);
    return BigInt(String(result));
  }

  /**
   * Create multiple escrows in a single transaction
   */
  async createEscrowsBatch(
    params: EscrowTypes.CreateEscrowBatchParams,
    options?: ContractInvokeOptions
  ): Promise<bigint[]> {
    // `create_escrows_batch` takes a `Vec<(u64, Address, i128, Option<u64>)>`,
    // so each entry must be encoded as its own Vec ScVal, not a bare array.
    const entries = params.escrowList.map((entry) => {
      const fleetId = optionalField(entry.fleetId, 'fleetId', (v) => typeof v === 'bigint' || typeof v === 'number' || typeof v === 'string');
      return vec([
        u64(entry.deliveryId),
        address(entry.driver),
        i128(entry.amount),
        fleetId === undefined ? xdrVoid() : u64(fleetId),
      ]);
    });
    const result = await this.invoker.call('create_escrows_batch', [address(params.sender), address(params.recipient), address(params.token), u64(params.fleetId ?? 0), vec(entries)], options);
    return decodeIds(result);
  }

  /**
   * Release funds to driver for a completed delivery
   */
  async releaseEscrow(
    params: EscrowTypes.ReleaseEscrowParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('release_escrow', [address(params.caller), u64(params.deliveryId)], options);
  }

  /**
   * Refund escrowed funds to sender
   */
  async refundEscrow(
    params: EscrowTypes.RefundEscrowParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('refund_escrow', [address(params.caller), u64(params.deliveryId)], options);
  }

  /**
   * Raise a dispute on an escrow
   */
  async raiseDispute(
    params: EscrowTypes.RaiseDisputeParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('raise_dispute', [address(params.caller), u64(params.deliveryId)], options);
  }

  /**
   * Resolve a dispute by releasing to driver or refunding sender
   */
  async resolveDispute(
    params: EscrowTypes.ResolveDisputeParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('resolve_dispute', [address(params.caller), u64(params.deliveryId), bool(params.releaseToDriver)], options);
  }

  /**
   * Resolve a dispute by splitting funds between sender and driver
   */
  async resolveDisputeSplit(
    params: EscrowTypes.ResolveDisputeSplitParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('resolve_dispute_split', [address(params.caller), u64(params.deliveryId), u32(params.senderShareBps)], options);
  }

  /**
   * Release escrow funds that are on holdback
   */
  async releaseHoldbackEscrow(
    params: EscrowTypes.ReleaseHoldbackEscrowParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('release_holdback_escrow', [address(params.caller), u64(params.deliveryId)], options);
  }

  /**
   * Permissionless counterpart to `releaseHoldbackEscrow`: settle an escrow
   * that has been in `Holdback` for at least the configured holdback window.
   *
   * This is the driver-protection fallback introduced in Issue #192 — without
   * it a recipient who never calls `releaseHoldbackEscrow` could strand the
   * driver's funds in `Holdback` indefinitely.
   */
  async releaseExpiredHoldback(
    params: EscrowTypes.ReleaseExpiredHoldbackParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('release_expired_holdback', [u64(params.deliveryId)], options);
  }

  /**
   * Decode an on-chain escrow record into a typed `EscrowRecord`.
   *
   * Optional fields are normalized so that `null` and `undefined` are treated
   * identically (both become `undefined`), and malformed present values are
   * rejected with field-specific errors instead of silently becoming `NaN`.
   */
  decodeEscrow(raw: Record<string, unknown>): EscrowRecord {
    const fleetId = optionalField(raw.fleet_id, 'fleet_id', (v) => typeof v === 'bigint' || typeof v === 'number' || typeof v === 'string');
    const disputeReason = optionalField(raw.dispute_reason, 'dispute_reason', (v) => typeof v === 'string');
    const resolvedAt = optionalField(raw.resolved_at, 'resolved_at', (v) => typeof v === 'bigint' || typeof v === 'number' || typeof v === 'string');

    return {
      deliveryId: Number(raw.delivery_id),
      sender: String(raw.sender),
      recipient: String(raw.recipient),
      driver: String(raw.driver),
      token: String(raw.token),
      amount: BigInt(String(raw.amount)),
      status: String(raw.status) as EscrowRecord['status'],
      createdAt: BigInt(String(raw.created_at)),
      fleetId: fleetId === undefined ? undefined : BigInt(String(fleetId)),
      disputeReason,
      resolvedAt: resolvedAt === undefined ? undefined : BigInt(String(resolvedAt)),
    } as EscrowRecord;
  }
}

function decodeIds(result: unknown): bigint[] {
  return (result as Array<unknown>).map((id) => BigInt(String(id)));
}

function xdrVoid(): ReturnType<typeof nativeToScVal> {
  return nativeToScVal(null);
}
