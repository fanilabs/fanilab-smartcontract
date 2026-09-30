/**
 * Type definitions for delivery contract functions
 */

import { DeliveryStatus, DriverStatus, EscrowStatus } from './common.types';

export interface CreateDeliveryParams {
  sender: string;
  recipient: string;
  deliveryId: bigint;
  metadata: DeliveryMetadata;
}

export enum CargoCategory {
  Documents = 'Documents',
  Electronics = 'Electronics',
  Perishables = 'Perishables',
  Clothing = 'Clothing',
  General = 'General',
}

export interface CargoDescriptor {
  weightGrams: number;
  category: CargoCategory;
  fragile: boolean;
}

export interface DeliveryMetadata {
  pickupLocation: string;
  dropoffLocation: string;
  cargoDescription?: CargoDescriptor;
  deliveryId?: bigint;
  createdAt?: number;
  estimatedDelivery?: number;
  /** @deprecated Use estimatedDelivery, which is an absolute Unix timestamp. */
  estimatedDistance?: number;
  /** Legacy application-only metadata; it is not stored by the contract. */
  items?: string;
  /** Legacy application-only metadata; it is not stored by the contract. */
  notes?: string;
}

export interface AssignDriverParams {
  caller: string;
  deliveryId: bigint;
  driver: string;
}

export interface ConfirmDeliveryParams {
  caller: string;
  deliveryId: bigint;
}

export interface CancelDeliveryParams {
  caller: string;
  deliveryId: bigint;
}

export interface MarkInTransitParams {
  caller: string;
  deliveryId: bigint;
}

export interface UpdateDeliveryMetadataParams {
  sender: string;
  deliveryId: bigint;
  metadata: DeliveryMetadata;
}

export interface GetDeliveryParams {
  deliveryId: bigint;
}

export interface DeliveryRecord {
  deliveryId: bigint;
  sender: string;
  recipient: string;
  driver?: string;
  status: DeliveryStatus;
  metadata: DeliveryMetadata;
  createdAt: number;
  deliveredAt?: number;
  transitStartedAt?: number;
}

export interface DeliveryDriverProfile {
  address: string;
  deliveriesCompleted: number;
  reputationScore: number;
  registeredAt: number;
  kycVerified: boolean;
  status: DriverStatus;
}

export interface DeliveryEscrowState {
  deliveryId: bigint;
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
  holdbackStartedAt?: number;
  fleetId?: bigint;
}

export interface CombinedDeliveryState {
  delivery: DeliveryRecord;
  escrow?: DeliveryEscrowState;
  isSynchronized: boolean;
}

export interface DeliveryCreatedEvent {
  deliveryId: bigint;
  sender: string;
}

export interface DriverAssignedEvent {
  deliveryId: bigint;
  driver: string;
}

export interface DeliveryConfirmedEvent {
  deliveryId: bigint;
  recipient: string;
  timestamp: number;
}

export interface DeliveryInTransitEvent {
  deliveryId: bigint;
  driver: string;
  timestamp: number;
}

export interface DeliveryCancelledEvent {
  deliveryId: bigint;
  reason?: string;
  timestamp: number;
}
