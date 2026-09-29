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
   * Set the identity reputation contract address (admin only)
   */
  async setIdentityReputationContract(
    identityContractId: string,
    options?: ContractInvokeOptions
  ): Promise<void> {
    const admin = options?.sourceAccount;
    if (!admin) {
      throw new Error('setIdentityReputationContract requires options.sourceAccount as admin');
    }
    this.identityInvoker = new ContractInvoker(identityContractId, options);
    await this.invoker.call(
      'set_identity_reputation_contract',
      [address(admin), address(identityContractId)],
      options
    );
  }

  /**
   * Set the escrow contract address (admin only)
   */
  async setEscrowContract(
    escrowContractId: string,
    options?: ContractInvokeOptions
  ): Promise<void> {
    const admin = options?.sourceAccount;
    if (!admin) {
      throw new Error('setEscrowContract requires options.sourceAccount as admin');
    }
    await this.invoker.call(
      'set_escrow_contract',
      [address(admin), address(escrowContractId)],
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
   * Create multiple deliveries in a single batch
   */
  async createDeliveriesBatch(
    params: DeliveryTypes.CreateDeliveriesBatchParams,
    options?: ContractInvokeOptions
  ): Promise<bigint[]> {
    const metadatas = params.deliveries.map((delivery) =>
      map([
        ['delivery_id', u64(delivery.deliveryId)],
        ['origin', string(delivery.metadata.pickupLocation ?? '')],
        ['destination', string(delivery.metadata.dropoffLocation ?? '')],
        ['cargo_description', map([
          ['weight_grams', u32(1)],
          ['category', symbol('General')],
          ['fragile', bool(false)],
        ])],
        ['created_at', u64(Math.floor(Date.now() / 1000))],
        ['estimated_delivery', u64(Math.floor(Date.now() / 1000) + (delivery.metadata.estimatedDistance ?? 0))],
      ])
    );
    const result = await this.invoker.call(
      'create_deliveries_batch',
      [address(params.sender), address(params.recipient), metadatas],
      options
    );
    return decodeIds(result);
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
   * Update metadata for a Pending delivery. Only the original sender may call
   * this, and only while the delivery is still in the Pending state.
   */
  async updateDeliveryMetadata(
    params: DeliveryTypes.UpdateDeliveryMetadataParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
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
    await this.invoker.call(
      'update_delivery_metadata',
      [address(params.sender), u64(params.deliveryId), metadata],
      options
    );
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
   * Get a driver profile from the identity contract
   */
  async getDriverProfile(driver: string, options?: ContractInvokeOptions): Promise<DeliveryTypes.DriverProfile> {
    return decodeDriverProfile(await this.identity().call('get_driver_profile', [address(driver)], options));
  }

  /**
   * Get the combined delivery state (delivery record plus escrow status)
   */
  async getCombinedState(deliveryId: bigint, options?: ContractInvokeOptions): Promise<DeliveryTypes.CombinedDeliveryState> {
    return decodeCombinedState(await this.invoker.call('get_combined_state', [u64(deliveryId)], options));
  }

  /**
   * Get a paginated list of deliveries
   */
  async getDeliveriesPage(
    offset: number,
    limit: number,
    options?: ContractInvokeOptions
  ): Promise<DeliveryTypes.DeliveryPage> {
    return decodeDeliveryPage(await this.invoker.call('get_deliveries_page', [u32(offset), u32(limit)], options));
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
    const result = await this.invoker.call('get_deliveries_by_driver', [address(driver)], this.options);
    return decodeIds(result);
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

function decodeDriverProfile(value: unknown): DeliveryTypes.DriverProfile {
  const record = value as Record<string, unknown>;
  return {
    driver: String(record.driver),
    name: String(record.name),
    vehicleType: String(record.vehicle_type),
    licenseNumber: String(record.license_number),
    isVerified: Boolean(record.is_verified),
    rating: Number(record.rating),
    completedDeliveries: Number(record.completed_deliveries),
  };
}

function decodeCombinedState(value: unknown): DeliveryTypes.CombinedDeliveryState {
  const record = value as Record<string, unknown>;
  return {
    delivery: decodeDelivery(record.delivery),
    escrowStatus: record.escrow_status === null ? undefined : String(record.escrow_status),
    escrowAmount: record.escrow_amount === null ? undefined : BigInt(String(record.escrow_amount)),
  };
}

function decodeDeliveryPage(value: unknown): DeliveryTypes.DeliveryPage {
  const record = value as Record<string, unknown>;
  const items = (record.items as unknown[]) ?? [];
  return {
    items: items.map((item) => decodeDelivery(item)),
    total: Number(record.total),
    offset: Number(record.offset),
    limit: Number(record.limit),
  };
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
