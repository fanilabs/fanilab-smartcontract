/**
 * Typed SDK client for EscrowContract
 */

import * as EscrowTypes from '../types/escrow.types';
import { nativeToScVal } from '@stellar/stellar-sdk';
import { EscrowRecord, ContractInvokeOptions } from '../types/common.types';
import { ContractInvoker, address, bool, i128, map, u32, u64, vec } from './invoker';

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
   */
  async setVolumeTiers(
    params: EscrowTypes.SetVolumeTiersParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    const tiers = vec(params.tiers.map((tier) =>
      map([
        ['volume_threshold', u32(tier.volumeThreshold)],
        ['discount_bps', u32(tier.discountBps)],
      ])
    ));
    await this.invoker.call('set_volume_tiers', [address(params.admin), tiers], options);
  }

  /**
   * Get configured sender volume thresholds and discounts
   */
  async getVolumeTiers(options?: ContractInvokeOptions): Promise<EscrowTypes.VolumeTier[]> {
    const result = await this.invoker.call('get_volume_tiers', [], options);
    return (result as Array<Record<string, unknown>>).map((tier) => ({
      volumeThreshold: Number(tier.volume_threshold),
      discountBps: Number(tier.discount_bps),
    }));
  }

  /**
   * Get a sender's accumulated volume
   */
  async getSenderVolume(sender: string, options?: ContractInvokeOptions): Promise<number> {
    return Number(await this.invoker.call('get_sender_volume', [address(sender)], options));
  }

  /**
   * Create a new escrow for a delivery
   */
  async createEscrow(params: EscrowTypes.CreateEscrowParams, options?: ContractInvokeOptions): Promise<bigint> {
    const result = await this.invoker.call('create_escrow', [address(params.sender), address(params.recipient), address(params.driver), u64(params.deliveryId), address(params.token), i128(params.amount), params.fleetId === undefined ? xdrVoid() : u64(params.fleetId)], options);
    return BigInt(String(result));
  }

  /**
   * Create multiple escrows in a single transaction
   */
  async createEscrowsBatch(
    params: EscrowTypes.CreateEscrowBatchParams,
    options?: ContractInvokeOptions
  ): Promise<bigint[]> {
    const entries = params.escrowList.map((entry) => [
      u64(entry.deliveryId),
      address(entry.driver),
      i128(entry.amount),
      entry.fleetId === undefined ? xdrVoid() : u64(entry.fleetId),
    ] as unknown[]);
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
   * Mark an escrow as holdback
   */
  async markHoldbackEscrow(
    params: EscrowTypes.MarkHoldbackEscrowParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('mark_holdback_escrow', [address(params.caller), u64(params.deliveryId)], options);
  }

  /**
   * Get an escrow record by delivery ID
   */
  async getEscrow(deliveryId: bigint, options?: ContractInvokeOptions): Promise<EscrowRecord> {
    return decodeEscrow(await this.invoker.call('get_escrow', [u64(deliveryId)], options));
  }

  /**
   * Get all escrow IDs for a sender
   */
  async getEscrowsBySender(sender: string, options?: ContractInvokeOptions): Promise<bigint[]> {
    return decodeIds(await this.invoker.call('get_escrows_by_sender', [address(sender)], options));
  }

  /**
   * Get all escrow IDs for a recipient
   */
  async getEscrowsByRecipient(recipient: string, options?: ContractInvokeOptions): Promise<bigint[]> {
    return decodeIds(await this.invoker.call('get_escrows_by_recipient', [address(recipient)], options));
  }

  /**
   * Get all escrow IDs for a driver
   */
  async getEscrowsByDriver(driver: string, options?: ContractInvokeOptions): Promise<bigint[]> {
    return decodeIds(await this.invoker.call('get_escrows_by_driver', [address(driver)], options));
  }

  /**
   * Reclaim an escrow that has expired
   */
  async reclaimExpiredEscrow(
    params: EscrowTypes.ReclaimExpiredEscrowParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('reclaim_expired_escrow', [u64(params.deliveryId)], options);
  }

  /**
   * Get the settlement contract address
   */
  async getSettlementContract(options?: ContractInvokeOptions): Promise<string | null> {
    return decodeOptionalAddress(await this.invoker.call('get_settlement_contract', [], options));
  }

  /**
   * Set the settlement contract address
   */
  async setSettlementContract(
    params: EscrowTypes.SetSettlementContractParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('set_settlement_contract', [address(params.admin), address(params.settlementContract)], options);
  }

  /**
   * Get the fleet management contract address
   */
  async getFleetManagementContract(options?: ContractInvokeOptions): Promise<string | null> {
    return decodeOptionalAddress(await this.invoker.call('get_fleet_management_contract', [], options));
  }

  /**
   * Set the fleet management contract address
   */
  async setFleetManagementContract(
    params: EscrowTypes.SetFleetManagementContractParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('set_fleet_management_contract', [address(params.admin), address(params.fleetContract)], options);
  }

  /**
   * Check if the protocol is paused
   */
  async isPaused(options?: ContractInvokeOptions): Promise<boolean> {
    return Boolean(await this.invoker.call('is_paused', [], options));
  }

  /**
   * Pause or unpause the protocol
   */
  async setPaused(admin: string, paused: boolean, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('set_paused', [address(admin), bool(paused)], options);
  }
}

function xdrVoid() {
  return nativeToScVal(null);
}

function decodeIds(value: unknown): bigint[] {
  return (value as unknown[]).map((id) => BigInt(String(id)));
}

function decodeOptionalAddress(value: unknown): string | null {
  return value === null || value === undefined ? null : String(value);
}

function decodeEscrow(value: unknown): EscrowRecord {
  const record = value as Record<string, unknown>;
  return {
    sender: String(record.sender), recipient: String(record.recipient), driver: String(record.driver),
    token: String(record.token), amount: BigInt(String(record.amount)), status: record.status as EscrowRecord['status'],
    createdAt: Number(record.created_at), expiresAt: record.expires_at === null ? undefined : Number(record.expires_at),
    disputedBy: record.disputed_by === null ? undefined : String(record.disputed_by),
    disputedAt: record.disputed_at === null ? undefined : Number(record.disputed_at),
    fleetId: record.fleet_id === null ? undefined : Number(record.fleet_id),
    deliveryId: BigInt(String(record.delivery_id ?? 0)),
    holdbackStartedAt: record.holdback_started_at === null ? undefined : Number(record.holdback_started_at),
  };
}
