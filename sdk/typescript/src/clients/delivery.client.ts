/**
 * Typed SDK client for DeliveryContract
 */

import * as DeliveryTypes from '../types/delivery.types';
import { ContractInvokeOptions } from '../types/common.types';
import { ContractInvoker, address, bool, map, string, symbol, u32, u64 } from './invoker';

export class DeliveryClient {
  private readonly invoker: ContractInvoker;
  private identityInvoker?: ContractInvoker;

  constructor(contractId: string, options: ContractInvokeOptions = {}) {
    this.invoker = new ContractInvoker(contractId, options);
    if (options.identityContractId) {
      this.identityInvoker = new ContractInvoker(options.identityContractId, options);
    }
  }

  /**
   * Initialize the delivery contract with escrow and identity contract addresses
   */
  async init(
    escrowContractId: string,
    identityContractId: string,
    options?: ContractInvokeOptions
  ): Promise<void> {
    const admin = options?.sourceAccount;
    if (!admin) {
      throw new Error('Delivery initialization requires options.sourceAccount as admin');
    }
    this.identityInvoker = new ContractInvoker(identityContractId, options);
    await this.invoker.call('init', [address(admin), address(escrowContractId)], options);
    await this.invoker.call(
      'set_identity_reputation_contract',
      [address(admin), address(identityContractId)],
      options
    );
  }

  /**
   * Create a new delivery
   */
  async createDelivery(
    params: DeliveryTypes.CreateDeliveryParams,
    options?: ContractInvokeOptions
  ): Promise<bigint> {
    const metadata = map([
      ['delivery_id', u64(params.deliveryId)],
      ['origin', string(params.metadata.pickupLocation ?? '')],
      ['destination', string(params.metadata.dropoffLocation ?? '')],
      ['cargo_description', map([
        ['weight_grams', u32(1)],
        ['category', symbol('General')],
        ['fragile', bool(false)],
      ])],
      ['created_at', u64(Math.floor(Date.now() / 1000))],
      ['estimated_delivery', u64(Math.floor(Date.now() / 1000) + (params.metadata.estimatedDistance ?? 0))],
    ]);
    return BigInt(String(await this.invoker.call('create_delivery', [address(params.sender), address(params.recipient), metadata], options)));
  }

  /**
   * Assign a driver to a delivery
   */
  async assignDriver(
    params: DeliveryTypes.AssignDriverParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('assign_driver', [address(params.caller), u64(params.deliveryId), address(params.driver)], options);
  }

  /**
   * Confirm that a delivery has been completed
   */
  async confirmDelivery(
    params: DeliveryTypes.ConfirmDeliveryParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('confirm_delivery', [address(params.caller), u64(params.deliveryId)], options);
  }

  /**
   * Cancel a delivery
   */
  async cancelDelivery(
    params: DeliveryTypes.CancelDeliveryParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('cancel_delivery', [address(params.caller), u64(params.deliveryId)], options);
  }

  /**
   * Mark a delivery as in transit
   */
  async markInTransit(
    params: DeliveryTypes.MarkInTransitParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('mark_in_transit', [address(params.caller), u64(params.deliveryId)], options);
  }

  /**
   * Update the metadata of an existing delivery
   */
  async updateDeliveryMetadata(
    params: DeliveryTypes.UpdateDeliveryMetadataParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    const metadata = map([
      ['origin', string(params.metadata.pickupLocation ?? '')],
      ['destination', string(params.metadata.dropoffLocation ?? '')],
      ['cargo_description', map([
        ['weight_grams', u32(1)],
        ['category', symbol('General')],
        ['fragile', bool(false)],
      ])],
      ['created_at', u64(Math.floor(Date.now() / 1000))],
      ['estimated_delivery', u64(Math.floor(Date.now() / 1000) + (params.metadata.estimatedDistance ?? 0))],
    ]);
    await this.invoker.call('update_delivery_metadata', [address(params.caller), u64(params.deliveryId), metadata], options);
  }

  /**
   * Raise a dispute for a delivery
   */
  async raiseDispute(
    params: DeliveryTypes.RaiseDisputeParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('raise_dispute', [address(params.caller), u64(params.deliveryId), string(params.reason)], options);
  }

  /**
   * Get a delivery record
   */
  async getDelivery(deliveryId: bigint, options?: ContractInvokeOptions): Promise<DeliveryTypes.DeliveryRecord> {
    return decodeDelivery(await this.invoker.call('get_delivery', [u64(deliveryId)], options));
  }

  /**
   * Get the escrow contract address
   */
  async getEscrowContract(options?: ContractInvokeOptions): Promise<string | null> {
    return decodeOptional(await this.invoker.call('get_escrow_contract', [], options));
  }

  /**
   * Get the identity contract address
   */
  async getIdentityContract(options?: ContractInvokeOptions): Promise<string | null> {
    return decodeOptional(await this.invoker.call('get_identity_reputation_contract', [], options));
  }

  /**
   * Check if a driver is registered
   */
  async isDriverRegistered(driver: string): Promise<boolean> {
    return Boolean(await this.identity().call('has_driver_profile', [address(driver)], undefined));
  }

  /**
   * Check if a user is registered
   */
  async isUserRegistered(user: string): Promise<boolean> {
    try {
      await this.identity().call('get_user_profile', [address(user)], undefined);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get all deliveries for a sender
   */
  async getDeliveriesBySender(sender: string): Promise<bigint[]> {
    return decodeIds(await this.invoker.call('get_deliveries_by_sender', [address(sender)], undefined));
  }

  /**
   * Get all deliveries for a recipient
   */
  async getDeliveriesByRecipient(recipient: string): Promise<bigint[]> {
    return decodeIds(await this.invoker.call('get_deliveries_by_recipient', [address(recipient)], undefined));
  }

  /**
   * Get all deliveries for a driver
   */
  async getDeliveriesByDriver(driver: string): Promise<bigint[]> {
    throw new Error('DeliveryContract does not expose get_deliveries_by_driver');
  }

  private identity(): ContractInvoker {
    if (!this.identityInvoker) {
      throw new Error('Identity contract is not configured');
    }
    return this.identityInvoker;
  }
}

function decodeOptional(value: unknown): string | null {
  return value === null || value === undefined ? null : String(value);
}

function decodeIds(value: unknown): bigint[] {
  return (value as unknown[]).map((id) => BigInt(String(id)));
}

function decodeDelivery(value: unknown): DeliveryTypes.DeliveryRecord {
  const record = value as Record<string, unknown>;
  const metadata = record.metadata as Record<string, unknown>;
  return {
    deliveryId: BigInt(String(record.delivery_id)), sender: String(record.sender), recipient: String(record.recipient),
    driver: record.driver === null ? undefined : String(record.driver), status: record.status as DeliveryTypes.DeliveryRecord['status'],
    metadata: {
      pickupLocation: String(metadata.origin), dropoffLocation: String(metadata.destination),
    }, createdAt: Number(record.created_at), deliveredAt: record.delivered_at === null ? undefined : Number(record.delivered_at),
    transitStartedAt: record.transit_started_at === null ? undefined : Number(record.transit_started_at),
  };
}
