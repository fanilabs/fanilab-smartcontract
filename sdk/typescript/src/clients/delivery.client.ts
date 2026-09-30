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
    const metadata = encodeDeliveryMetadata(params.deliveryId, params.metadata);
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
      encodeDeliveryMetadata(delivery.deliveryId, delivery.metadata)
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
    const metadata = encodeDeliveryMetadata(params.deliveryId, params.metadata);
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
   * Set the dispute resolution contract address (admin only).
   *
   * This contract is the only caller permitted to drive a delivery into
   * `Disputed`: `raise_dispute` is deliberately not a user entry point, so a
   * delivery party cannot pause the escrow without a recorded dispute case.
   */
  async setDisputeResolutionContract(
    disputeContractId: string,
    options?: ContractInvokeOptions
  ): Promise<void> {
    const admin = options?.sourceAccount;
    if (!admin) {
      throw new Error('setDisputeResolutionContract requires options.sourceAccount as admin');
    }
    await this.invoker.call(
      'set_dispute_resolution_contract',
      [address(admin), address(disputeContractId)],
      options
    );
  }

  /**
   * Get the dispute resolution contract address
   */
  async getDisputeResolutionContract(options?: ContractInvokeOptions): Promise<string | null> {
    return decodeOptional(await this.invoker.call('get_dispute_resolution_contract', [], options));
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
  async getDriverProfile(driver: string, options?: ContractInvokeOptions): Promise<DeliveryTypes.DeliveryDriverProfile> {
    return decodeDriverProfile(await this.invoker.call('get_driver_profile', [address(driver)], options));
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
    const result = await this.invoker.call('get_deliveries_by_driver', [address(driver)], undefined);
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

function decodeDriverProfile(value: unknown): DeliveryTypes.DeliveryDriverProfile {
  const record = value as Record<string, unknown>;
  return {
    address: String(record.address),
    deliveriesCompleted: Number(record.deliveries_completed),
    reputationScore: Number(record.reputation_score),
    registeredAt: Number(record.registered_at),
    kycVerified: Boolean(record.kyc_verified),
    status: String(record.status) as DeliveryTypes.DeliveryDriverProfile['status'],
  };
}

function decodeCombinedState(value: unknown): DeliveryTypes.CombinedDeliveryState {
  if (!Array.isArray(value) || value.length !== 3) {
    throw new TypeError('Invalid combined delivery state returned by contract');
  }
  const [delivery, escrow, isSynchronized] = value;
  return {
    delivery: decodeDelivery(delivery),
    escrow: escrow === null || escrow === undefined ? undefined : decodeDeliveryEscrow(escrow),
    isSynchronized: Boolean(isSynchronized),
  };
}

function decodeDeliveryEscrow(value: unknown): DeliveryTypes.DeliveryEscrowState {
  const record = value as Record<string, unknown>;
  return {
    deliveryId: BigInt(String(record.delivery_id)),
    sender: String(record.sender),
    recipient: String(record.recipient),
    driver: String(record.driver),
    token: String(record.token),
    amount: BigInt(String(record.amount)),
    status: String(record.status) as DeliveryTypes.DeliveryEscrowState['status'],
    createdAt: Number(record.created_at),
    expiresAt: record.expires_at == null ? undefined : Number(record.expires_at),
    disputedBy: record.disputed_by == null ? undefined : String(record.disputed_by),
    disputedAt: record.disputed_at == null ? undefined : Number(record.disputed_at),
    holdbackStartedAt: record.holdback_started_at == null
      ? undefined
      : Number(record.holdback_started_at),
    fleetId: record.fleet_id == null ? undefined : BigInt(String(record.fleet_id)),
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
  const cargo = metadata.cargo_description as Record<string, unknown>;
  return {
    deliveryId: BigInt(String(record.delivery_id)), sender: String(record.sender), recipient: String(record.recipient),
    driver: record.driver === null ? undefined : String(record.driver), status: record.status as DeliveryTypes.DeliveryRecord['status'],
    metadata: {
      pickupLocation: String(metadata.origin),
      dropoffLocation: String(metadata.destination),
      cargoDescription: {
        weightGrams: Number(cargo.weight_grams),
        category: String(cargo.category) as DeliveryTypes.CargoCategory,
        fragile: Boolean(cargo.fragile),
      },
      deliveryId: BigInt(String(metadata.delivery_id)),
      createdAt: Number(metadata.created_at),
      estimatedDelivery: Number(metadata.estimated_delivery),
    }, createdAt: Number(record.created_at), deliveredAt: record.delivered_at === null ? undefined : Number(record.delivered_at),
    transitStartedAt: record.transit_started_at === null ? undefined : Number(record.transit_started_at),
  };
}

function encodeDeliveryMetadata(
  deliveryId: bigint,
  metadata: DeliveryTypes.DeliveryMetadata
) {
  const now = Math.floor(Date.now() / 1000);
  const cargo = metadata.cargoDescription;
  return map([
    ['delivery_id', u64(deliveryId)],
    ['origin', string(metadata.pickupLocation)],
    ['destination', string(metadata.dropoffLocation)],
    ['cargo_description', map([
      ['weight_grams', u32(cargo?.weightGrams ?? 1)],
      ['category', symbol(cargo?.category ?? DeliveryTypes.CargoCategory.General)],
      ['fragile', bool(cargo?.fragile ?? false)],
    ])],
    ['created_at', u64(metadata.createdAt ?? now)],
    ['estimated_delivery', u64(metadata.estimatedDelivery ?? now + (metadata.estimatedDistance ?? 0))],
  ]);
}
