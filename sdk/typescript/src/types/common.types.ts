import { Keypair } from '@stellar/stellar-sdk';

/**
 * Common types shared across FaniLab smart contracts
 */

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

export interface ProtocolConfig {
  token: string;
  platformFeeBps: number;
  protocolVersion: number;
  slippageToleranceBps: number;
}

export interface EscrowRecord {
  sender: string;
  recipient: string;
  driver: string;
  token: string;
  amount: bigint;
  status: EscrowStatus;
  createdAt: number;
  expiresAt?: number;
  disputedBy?: string;
  disputedAt?: number;
  fleetId?: bigint;
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
  registeredAt: number;
  kycVerified: boolean;
}

export interface UserProfile {
  address: string;
  registeredAt: number;
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
  activatesAt: number;
}

export interface EvidenceEntry {
  submitter: string;
  hash: string;
}

export interface DisputeCase {
  deliveryId: bigint;
  status: DisputeStatus;
  raisedAt: number;
  raisedBy: string;
  evidenceHashes: EvidenceEntry[];
  resolvedAt?: number;
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
} as const;

export interface ContractInvokeOptions {
  networkPassphrase?: string;
  serverUrl?: string;
  timeout?: number;
  timeoutSeconds?: number;
  fee?: string;
  sourceAccount?: string;
  signer?: Keypair;
  identityContractId?: string;
}
