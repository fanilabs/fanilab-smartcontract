/**
 * Type definitions for escrow contract functions
 */

import { EscrowStatus, ProtocolConfig, EscrowRecord } from './common.types';

export interface InitParams {
  admin: string;
  token: string;
  platformFeeBps: number;
}

export interface UpdatePlatformFeeParams {
  admin: string;
  newFeeBps: number;
}

export interface VolumeTier {
  volumeThreshold: number;
  discountBps: number;
}

export interface SetVolumeTiersParams {
  admin: string;
  tiers: VolumeTier[];
}

export interface SweepUntrackedBalanceParams {
  admin: string;
  token: string;
  recipient: string;
}

export interface CreateEscrowParams {
  sender: string;
  recipient: string;
  driver: string;
  deliveryId: bigint;
  token: string;
  amount: bigint;
  fleetId?: bigint;
}

export interface CreateEscrowBatchParams {
  sender: string;
  recipient: string;
  token: string;
  escrowList: Array<{
    deliveryId: bigint;
    driver: string;
    amount: bigint;
    fleetId?: bigint;
  }>;
  fleetId?: bigint;
}

export interface ReleaseEscrowParams {
  caller: string;
  deliveryId: bigint;
}

export interface RefundEscrowParams {
  caller: string;
  deliveryId: bigint;
}

export interface RaiseDisputeParams {
  caller: string;
  deliveryId: bigint;
}

export interface ResolveDisputeParams {
  caller: string;
  deliveryId: bigint;
  releaseToDriver: boolean;
}

export interface ResolveDisputeSplitParams {
  caller: string;
  deliveryId: bigint;
  senderShareBps: number;
}

export interface ReleaseHoldbackEscrowParams {
  caller: string;
  deliveryId: bigint;
}

/**
 * Parameters for the permissionless `release_expired_holdback` fallback
 * (Issue #452). No `caller` is required — anyone may submit it once the
 * holdback window has elapsed.
 */
export interface ReleaseExpiredHoldbackParams {
  deliveryId: bigint;
}

export interface MarkHoldbackEscrowParams {
  caller: string;
  deliveryId: bigint;
}

export interface FreezeFundsParams {
  caller: string;
  deliveryId: bigint;
}

export interface ReclaimExpiredEscrowParams {
  deliveryId: bigint;
}

/**
 * Parameters for the admin-only `sweep_untracked_balance` operation
 * (Issue #450).
 */
export interface SweepParams {
  /** Contract admin authorising the sweep. */
  admin: string;
  /** Token whose untracked balance is swept. */
  token: string;
  /** Treasury wallet the untracked balance is transferred to. */
  recipient: string;
}

export interface SetSettlementContractParams {
  admin: string;
  settlementContract: string;
}

export interface SetFleetManagementContractParams {
  admin: string;
  fleetContract: string;
}

export interface SetDisputeResolutionContractParams {
  admin: string;
  disputeContract: string;
}

export interface EscrowReleasedEvent {
  deliveryId: bigint;
  driver: string;
  amount: bigint;
  platformFee: bigint;
}

export interface EscrowFundedEvent {
  deliveryId: bigint;
  sender: string;
  token: string;
  amount: bigint;
}

export interface EscrowRefundedEvent {
  deliveryId: bigint;
  sender: string;
  amount: bigint;
}

export interface DeliveryDisputedEvent {
  deliveryId: bigint;
  reporter: string;
  timestamp: number;
}

export interface DisputeResolvedEvent {
  deliveryId: bigint;
  resolver: string;
}
