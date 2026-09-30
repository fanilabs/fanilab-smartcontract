/**
 * Type definitions for delivery contract functions
 */

import { DeliveryStatus } from './common.types';

/**
 * Error codes emitted by the Rust `delivery_contract` as contract panics.
 * Maps directly to the `DeliveryError` `#[contracterror]` enum (repr u32).
 * Use these constants to programmatically identify specific protocol errors
 * rather than hard-coding raw integer values.
 *
 * @example
 * ```ts
 * try {
 *   await deliveryClient.assignDriver(params);
 * } catch (err) {
 *   if (extractErrorCode(err) === DeliveryErrorCode.InvalidDriver) {
 *     // handle invalid driver
 *   }
 * }
 * ```
 */
export enum DeliveryErrorCode {
  /** The requested state transition is not permitted by the delivery state machine. */
  InvalidState = 1,
  /** Delivery metadata failed validation (empty origin/destination, invalid weight, etc.). */
  InvalidMetadata = 2,
  /** A batch operation exceeded the maximum allowed batch size. */
  BatchTooLarge = 3,
  /** The driver address is the same as the sender or recipient, which is never valid. */
  InvalidDriver = 4,
  /** Sender and recipient must be different parties. */
  InvalidParties = 5,
  /**
   * The escrow securing this delivery is absent or not in the `Locked` state
   * at a point where a funded escrow is a precondition (e.g. `mark_in_transit`).
   */
  EscrowNotLocked = 6,
}

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
