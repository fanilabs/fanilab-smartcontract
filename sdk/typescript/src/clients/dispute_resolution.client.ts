/**
 * Typed SDK client for DisputeResolutionContract
 */

import { ContractInvokeOptions, DisputeCase, DisputeStatus } from '../types/common.types';
import * as DisputeResolutionTypes from '../types/dispute_resolution.types';
import { ContractInvoker, address, u32, u64 } from './invoker';

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
    await this.invoker.call('add_evidence_hash', [address(params.caller), u64(params.deliveryId), new Uint8Array(Buffer.from(params.evidenceHash, 'hex')) as any], options);
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
}

function decodeDispute(value: unknown): DisputeCase {
  const record = value as Record<string, unknown>;
  return {
    deliveryId: BigInt(String(record.delivery_id)),
    status: record.status as DisputeStatus,
    raisedAt: Number(record.raised_at),
    raisedBy: String(record.raised_by),
    evidenceHashes: ((record.evidence_hashes as unknown[]) ?? []).map((entry) => {
      const item = entry as Record<string, unknown>;
      return {
        submitter: String(item.submitter),
        hash: String(item.hash),
      };
    }),
    resolvedAt: record.resolved_at === null || record.resolved_at === undefined ? undefined : Number(record.resolved_at),
    resolvedBy: record.resolved_by === null || record.resolved_by === undefined ? undefined : String(record.resolved_by),
  };
}
