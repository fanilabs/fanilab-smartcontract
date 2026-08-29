import { DisputeCase, DisputeStatus } from './common.types';

export interface InitParams {
  admin: string;
  deliveryContract: string;
  escrowContract: string;
  disputeTimeLimit: bigint;
  disputeResolutionLimit: bigint;
}

export interface AddAdminParams {
  caller: string;
  newAdmin: string;
}

export interface RemoveAdminParams {
  caller: string;
  oldAdmin: string;
}

export interface SetIdentityReputationContractParams {
  caller: string;
  reputationContract: string;
}

export interface SetDisputeReputationPenaltyParams {
  caller: string;
  penalty: number;
}

export interface SetDisputeResolutionLimitParams {
  caller: string;
  newLimit: bigint;
}

export interface UpdateDisputeTimeLimitParams {
  caller: string;
  newLimit: bigint;
}

export interface RaiseDisputeParams {
  caller: string;
  deliveryId: bigint;
}

export interface AddEvidenceHashParams {
  caller: string;
  deliveryId: bigint;
  evidenceHash: string;
}

export interface ResolveDisputeRefundSenderParams {
  caller: string;
  deliveryId: bigint;
}

export interface ResolveDisputeSplitFundsParams {
  caller: string;
  deliveryId: bigint;
  senderShareBps: number;
}

export interface ResolveDisputePayDriverParams {
  caller: string;
  deliveryId: bigint;
}

export interface ForceResolveDisputeParams {
  caller: string;
  deliveryId: bigint;
}

export { DisputeCase, DisputeStatus };
