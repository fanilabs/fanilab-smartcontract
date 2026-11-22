/**
 * Typed SDK client for DisputeResolutionContract
 */

import { ContractInvokeOptions, DisputeCase, DisputeStatus } from '../types/common.types';
import * as DisputeResolutionTypes from '../types/dispute_resolution.types';
import { ContractInvoker, address, bytes, u32, u64 } from './invoker';

export class DisputeResolutionClient {
  private readonly invoker: ContractInvoker;

  constructor(contractId: string, options: ContractInvokeOptions = {}) {
    this.invoker = new ContractInvoker(contractId, options);
  }

  async init(params: DisputeResolutionTypes.InitParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('init', [
      address(params.admin),
      address(params.deliveryContract),
      address(params.escrowContract),
      u64(params.disputeTimeLimit),
      u64(params.disputeResolutionLimit),
    ], options);
  }

  async addAdmin(params: DisputeResolutionTypes.AddAdminParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('add_admin', [address(params.caller), address(params.newAdmin)], options);
  }

  async acceptAdmin(params: DisputeResolutionTypes.AcceptAdminParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('accept_admin', [address(params.newAdmin)], options);
  }

  async removeAdmin(params: DisputeResolutionTypes.RemoveAdminParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('remove_admin', [address(params.caller), address(params.oldAdmin)], options);
  }

  async isAdmin(admin: string, options?: ContractInvokeOptions): Promise<boolean> {
    return Boolean(await this.invoker.call('is_admin', [address(admin)], options));
  }

  async listAdmins(options?: ContractInvokeOptions): Promise<string[]> {
    const result = await this.invoker.call('list_admins', [], options);
    return (result as unknown[]).map((value) => String(value));
  }

  async getDeliveryContract(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_delivery_contract', [], options));
  }

  async getEscrowContract(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_escrow_contract', [], options));
  }

  async setIdentityReputationContract(
    params: DisputeResolutionTypes.SetIdentityReputationContractParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('set_identity_reputation_contract', [address(params.caller), address(params.reputationContract)], options);
  }

  async getIdentityReputationContract(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_identity_reputation_contract', [], options));
  }

  async getDisputeTimeLimit(options?: ContractInvokeOptions): Promise<bigint> {
    return BigInt(String(await this.invoker.call('get_dispute_time_limit', [], options)));
  }

  async setDisputeReputationPenalty(
    params: DisputeResolutionTypes.SetDisputeReputationPenaltyParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('set_dispute_reputation_penalty', [address(params.caller), u32(params.penalty)], options);
  }

  async getDisputeReputationPenalty(options?: ContractInvokeOptions): Promise<number> {
    return Number(await this.invoker.call('get_dispute_reputation_penalty', [], options));
  }

  async getDisputeResolutionLimit(options?: ContractInvokeOptions): Promise<bigint> {
    return BigInt(String(await this.invoker.call('get_dispute_resolution_limit', [], options)));
  }

  async setDisputeResolutionLimit(
    params: DisputeResolutionTypes.SetDisputeResolutionLimitParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('set_dispute_resolution_limit', [address(params.caller), u64(params.newLimit)], options);
  }

  async updateDisputeTimeLimit(
    params: DisputeResolutionTypes.UpdateDisputeTimeLimitParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('update_dispute_time_limit', [address(params.caller), u64(params.newLimit)], options);
  }

  async raiseDispute(params: DisputeResolutionTypes.RaiseDisputeParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('raise_dispute', [address(params.caller), u64(params.deliveryId)], options);
  }

  async addEvidenceHash(
    params: DisputeResolutionTypes.AddEvidenceHashParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    const evidenceHashBytes = decodeEvidenceHash(params.evidenceHash);
    await this.invoker.call('add_evidence_hash', [address(params.caller), u64(params.deliveryId), bytes(evidenceHashBytes)], options);
  }

  async resolveDisputeRefundSender(
    params: DisputeResolutionTypes.ResolveDisputeRefundSenderParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('resolve_dispute_refund_sender', [address(params.caller), u64(params.deliveryId)], options);
  }

  async resolveDisputeSplitFunds(
    params: DisputeResolutionTypes.ResolveDisputeSplitFundsParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('resolve_dispute_split_funds', [address(params.caller), u64(params.deliveryId), u32(params.senderShareBps)], options);
  }

  async resolveDisputePayDriver(
    params: DisputeResolutionTypes.ResolveDisputePayDriverParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('resolve_dispute_pay_driver', [address(params.caller), u64(params.deliveryId)], options);
  }

  async forceResolveDispute(
    params: DisputeResolutionTypes.ForceResolveDisputeParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('force_resolve_dispute', [address(params.caller), u64(params.deliveryId)], options);
  }

  async getDispute(deliveryId: bigint, options?: ContractInvokeOptions): Promise<DisputeCase> {
    return decodeDispute(await this.invoker.call('get_dispute', [u64(deliveryId)], options));
  }

  async hasDispute(deliveryId: bigint, options?: ContractInvokeOptions): Promise<boolean> {
    return Boolean(await this.invoker.call('has_dispute', [u64(deliveryId)], options));
  }

  async getDisputesPage(page: number, pageSize: number, options?: ContractInvokeOptions): Promise<bigint[]> {
    const offset = computeDisputesPageOffset(page, pageSize);
    if (offset === null) {
      // A zero page size yields an empty page rather than an RPC round-trip.
      return [];
    }
    const result = await this.invoker.call('get_disputes_page', [u32(offset), u32(pageSize)], options);
    return (result as unknown[]).map((deliveryId) => BigInt(String(deliveryId)));
  }
}

/**
 * Validate dispute pagination inputs and compute the encoded offset.
 *
 * `page * pageSize` is fed straight into a `u32` argument, so negative,
 * fractional, or overflowing values would silently wrap or truncate before
 * reaching the contract (Issue #727). Require both arguments to be
 * non-negative safe integers, reject a product that exceeds
 * `Number.MAX_SAFE_INTEGER`, and treat a zero `pageSize` as an empty page
 * (returns `null` so the caller can short-circuit without an RPC call).
 */
function computeDisputesPageOffset(page: number, pageSize: number): number | null {
  if (!Number.isInteger(page) || page < 0) {
    throw new RangeError(`Invalid dispute page: expected a non-negative integer, received ${page}`);
  }
  if (!Number.isInteger(pageSize) || pageSize < 0) {
    throw new RangeError(`Invalid dispute page size: expected a non-negative integer, received ${pageSize}`);
  }
  if (pageSize === 0) {
    return null;
  }
  const offset = page * pageSize;
  if (!Number.isSafeInteger(offset)) {
    throw new RangeError(
      `Dispute page offset overflows safe integer range: page ${page} * pageSize ${pageSize}`
    );
  }
  return offset;
}

/**
 * Normalize a Soroban enum value into its canonical variant name.
 *
 * Soroban enums are represented as `ScVal::Vec([Symbol(name)])` (or, in some
 * bindings, as a bare `Symbol`/string or a numeric discriminant). This helper
 * unwraps the tuple representation and returns the variant name so callers can
 * map it to a typed `DisputeStatus` (Issue #728).
 */
function normalizeEnumVariant(value: unknown): string {
  if (Array.isArray(value)) {
    if (value.length !== 1) {
      throw new Error(`Invalid Soroban enum tuple: expected a single variant, received ${value.length}`);
    }
    return normalizeEnumVariant(value[0]);
  }
  if (typeof value === 'string') {
    return value;
  }
  if (typeof value === 'object' && value !== null) {
    const record = value as Record<string, unknown>;
    if (typeof record.name === 'string') {
      return record.name;
    }
    if (typeof record.value === 'string') {
      return record.value;
    }
    if (typeof record.symbol === 'string') {
      return record.symbol;
    }
  }
  throw new Error(`Unsupported Soroban enum representation: ${String(value)}`);
}

/**
 * Decode a native `DisputeStatus` value from a Soroban enum ScVal.
 *
 * Accepts the tuple-wrapped enum form (`[Symbol('Open')]`), a bare symbol
 * name, or a numeric discriminant, and rejects unknown variants instead of
 * returning an invalid status (Issue #728).
 */
function decodeDisputeStatus(value: unknown): DisputeStatus {
  const variant = normalizeEnumVariant(value);
  switch (variant) {
    case 'Open':
      return DisputeStatus.Open;
    case 'ResolvedRefund':
      return DisputeStatus.ResolvedRefund;
    case 'ResolvedPayout':
      return DisputeStatus.ResolvedPayout;
    case 'Split':
      return DisputeStatus.Split;
    default:
      throw new Error(`Unknown DisputeStatus variant: ${variant}`);
  }
}

/**
 * Decode a native dispute case returned by the contract into a typed
 * `DisputeCase`, normalizing the `status` enum representation (Issue #728).
 */
function decodeDispute(value: unknown): DisputeCase {
  const record = value as Record<string, unknown>;
  return {
    deliveryId: BigInt(String(record.delivery_id)),
    status: decodeDisputeStatus(record.status),
    raisedAt: BigInt(String(record.raised_at)),
    raisedBy: String(record.raised_by),
    evidenceHashes: ((record.evidence_hashes as unknown[]) ?? []).map((entry) => {
      const item = entry as Record<string, unknown>;
      return {
        submitter: String(item.submitter),
        hash: hexEncode(item.hash),
      };
    }),
    resolvedAt:
      record.resolved_at === null || record.resolved_at === undefined
        ? undefined
        : BigInt(String(record.resolved_at)),
    resolvedBy:
      record.resolved_by === null || record.resolved_by === undefined
        ? undefined
        : String(record.resolved_by),
  };
}

/**
 * Validate and decode a `BytesN<32>` evidence hash supplied as hex text.
 *
 * `Buffer.from(text, 'hex')` silently truncates at the first invalid pair
 * and accepts odd-length input, so a malformed hash could reach the RPC as
 * a short byte buffer (Issue #726). Require exactly 64 hexadecimal
 * characters and reject anything else before decoding.
 */
function decodeEvidenceHash(evidenceHash: string): Uint8Array {
  if (typeof evidenceHash !== 'string' || !/^[0-9a-fA-F]{64}$/.test(evidenceHash)) {
    throw new TypeError(
      `Invalid evidence hash: expected exactly 64 hexadecimal characters, received ${JSON.stringify(evidenceHash)}`
    );
  }
  return new Uint8Array(Buffer.from(evidenceHash, 'hex'));
}

/**
 * Serialize a `BytesN<32>` evidence hash into its canonical lowercase hex
 * form. `scValToNative` hands back a byte buffer (`Uint8Array`/`Buffer`),
 * so a bare `String(...)` would yield "[object Uint8Array]" and destroy the
 * proof-of-delivery hash (Issue #474). A value that is already a string is
 * passed through unchanged so the decoder tolerates RPCs that surface the
 * hash as text.
 */
function hexEncode(value: unknown): string {
  if (typeof value === 'string') {
    return value;
  }
  if (value instanceof Uint8Array) {
    return Buffer.from(value).toString('hex');
  }
  throw new TypeError(`Expected a byte buffer evidence hash, received ${typeof value}`);
}
