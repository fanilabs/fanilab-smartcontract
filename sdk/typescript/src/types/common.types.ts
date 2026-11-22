import { Keypair } from '@stellar/stellar-sdk';

/**
 * Common types shared across FaniLab smart contracts
 */

/**
 * A Rust `u64` timestamp, represented as a `bigint` so values above
 * `Number.MAX_SAFE_INTEGER` (2^53 - 1) are preserved losslessly.
 */
export type U64Timestamp = bigint;

export enum EscrowStatus {
  Locked = 'Locked',
  Holdback = 'Holdback',
  Released = 'Released',
  Refunded = 'Refunded',
  Paused = 'Paused',
  Split = 'Split',
}

export enum DeliveryStatus {
  Pending = 'Pending',
  Active = 'Active',
  InTransit = 'InTransit',
  Delivered = 'Delivered',
  Disputed = 'Disputed',
  Cancelled = 'Cancelled',
}

export enum DisputeStatus {
  Open = 'Open',
  ResolvedRefund = 'ResolvedRefund',
  ResolvedPayout = 'ResolvedPayout',
  Split = 'Split',
}

export enum DriverTier {
  Bronze = 'Bronze',
  Silver = 'Silver',
  Gold = 'Gold',
}

export enum DriverFleetStatus {
  Pending = 'Pending',
  Active = 'Active',
  Removed = 'Removed',
}

/** Mirrors the contract's `DriverStatus` enum in `shared_types`. */
export enum DriverStatus {
  Active = 'Active',
  Suspended = 'Suspended',
}

export interface ProtocolConfig {
  token: string;
  platformFeeBps: number;
  protocolVersion: number;
  slippageToleranceBps: number;
}

export interface EscrowRecord {
  deliveryId: number;
  sender: string;
  recipient: string;
  driver: string;
  token: string;
  amount: bigint;
  status: EscrowStatus;
  createdAt: U64Timestamp;
  expiresAt?: U64Timestamp;
  disputedBy?: string;
  disputedAt?: U64Timestamp;
  fleetId?: bigint;
  /**
   * Timestamp at which the escrow entered the `Holdback` state, used together
   * with the configured holdback window to decide when
   * `release_expired_holdback` becomes available.
   */
  holdbackStartedAt?: U64Timestamp;
}

export interface PartyAddresses {
  sender: string;
  driver: string;
  recipient: string;
}

export interface DriverProfile {
  address: string;
  deliveriesCompleted: number;
  reputationScore: number;
  registeredAt: U64Timestamp;
  kycVerified: boolean;
  /**
   * Lifecycle status of the driver, mirroring the contract's `DriverStatus`.
   * `Active` on registration, `Suspended` after an admin `suspend_driver`.
   */
  status: DriverStatus;
}

export interface UserProfile {
  address: string;
  registeredAt: U64Timestamp;
}

export interface ReputationConfig {
  basePoints: number;
  heavyCargoPoints: number;
  fragilePoints: number;
}

export interface FleetProfile {
  fleetId: bigint;
  owner: string;
  treasury: string;
  totalActiveDrivers: number;
  signers: string[];
  signatureThreshold: number;
  active: boolean;
}

export interface PendingTreasuryChange {
  treasury: string;
  activatesAt: U64Timestamp;
}

export interface EvidenceEntry {
  submitter: string;
  hash: string;
}

export interface DisputeCase {
  deliveryId: bigint;
  status: DisputeStatus;
  raisedAt: U64Timestamp;
  raisedBy: string;
  evidenceHashes: EvidenceEntry[];
  resolvedAt?: U64Timestamp;
  resolvedBy?: string;
}

export interface FaniLabError {
  code: number;
  message: string;
}

export const ErrorCodes = {
  Unauthorized: 1,
  AlreadyInitialized: 2,
  NotInitialized: 3,
  DeliveryNotFound: 4,
  InvalidState: 5,
  InsufficientFunds: 6,
  DuplicateDelivery: 8,
  ProviderNotFound: 9,
  ProtocolPaused: 11,
  LimitExceeded: 12,
  TimelockNotElapsed: 13,
} as const;

export interface ContractInvokeOptions {
  networkPassphrase?: string;
  serverUrl?: string;
  /** RPC request timeout in milliseconds. */
  timeout?: number;
  timeoutSeconds?: number;
  /** Initial delay between transaction status polls, in milliseconds. */
  pollingIntervalMs?: number;
  /** Maximum delay between transaction status polls, in milliseconds. */
  pollingMaxIntervalMs?: number;
  /** Maximum time to wait for a submitted transaction, in milliseconds. */
  pollingTimeoutMs?: number;
  /** Abort an in-progress transaction status wait. */
  signal?: AbortSignal;
  /** Extend persistent storage entries whose remaining TTL is at or below this ledger count. */
  ttlThresholdLedgers?: number;
  /** Minimum TTL, in ledgers, for entries selected for extension. */
  ttlExtendToLedgers?: number;
  fee?: string;
  sourceAccount?: string;
  signer?: Keypair;
  identityContractId?: string;
}
