/**
 * Type definitions for delivery contract functions
 */

import { DeliveryStatus } from './common.types';

export interface CreateDeliveryParams {
  sender: string;
  recipient: string;
  deliveryId: bigint;
  metadata: DeliveryMetadata;
}

export interface CargoDescriptor {
  description: string;
  weight?: number;
  dimensions?: string;
  fragile?: boolean;
}

export interface DeliveryMetadata {
  origin: string;
  destination: string;
  cargo_description: string;
  created_at: number;
  estimated_delivery: number;
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
