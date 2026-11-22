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

  async confirmEscrowContract(options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('confirm_escrow_contract', [], options);
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
   * Get deliveries by sender index
   */
  async getDeliveriesBySender(
    sender: string,
    options?: ContractInvokeOptions
  ): Promise<DeliveryTypes.DeliveryPage> {
    return decodeDeliveryPage(await this.invoker.call('get_deliveries_by_sender', [address(sender)], options));
  }

  /**
   * Get deliveries by recipient index
   */
  async getDeliveriesByRecipient(
    recipient: string,
    options?: ContractInvokeOptions
  ): Promise<DeliveryTypes.DeliveryPage> {
    return decodeDeliveryPage(await this.invoker.call('get_deliveries_by_recipient', [address(recipient)], options));
  }

  /**
   * Get deliveries by driver index
   */
  async getDeliveriesByDriver(
    driver: string,
    options?: ContractInvokeOptions
  ): Promise<DeliveryTypes.DeliveryPage> {
    return decodeDeliveryPage(await this.invoker.call('get_deliveries_by_driver', [address(driver)], options));
  }

  /**
   * Get the total number of deliveries
   */
  async getDeliveryCount(options?: ContractInvokeOptions): Promise<bigint> {
    return BigInt(String(await this.invoker.call('get_delivery_count', [], options)));
  }
}

const DELIVERY_STATUSES: ReadonlyArray<DeliveryTypes.DeliveryStatus> = [
  'Pending',
  'InTransit',
  'Delivered',
  'Cancelled',
  'Disputed',
];

function decodeDelivery(raw: unknown): DeliveryTypes.DeliveryRecord {
  const record = requireRecord(raw, 'delivery');
  const status = requireField(record, 'status', 'delivery');
  if (typeof status !== 'string' || !DELIVERY_STATUSES.includes(status as DeliveryTypes.DeliveryStatus)) {
    throw new Error(
      `Invalid delivery response: field 'status' has unexpected value ${JSON.stringify(status)}`
    );
  }
  return {
    deliveryId: requireField(record, 'deliveryId', 'delivery') as bigint,
    sender: requireField(record, 'sender', 'delivery') as string,
    recipient: requireField(record, 'recipient', 'delivery') as string,
    driver: requireField(record, 'driver', 'delivery') as string,
    status: status as DeliveryTypes.DeliveryStatus,
    createdAt: requireField(record, 'createdAt', 'delivery') as bigint,
    updatedAt: requireField(record, 'updatedAt', 'delivery') as bigint,
  };
}

function requireRecord(raw: unknown, context: string): Record<string, unknown> {
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) {
    throw new Error(`Invalid ${context} response: expected an object, got ${typeof raw}`);
  }
  return raw as Record<string, unknown>;
}

function requireField(record: Record<string, unknown>, field: string, context: string): unknown {
  if (!(field in record) || record[field] === undefined || record[field] === null) {
    throw new Error(`Invalid ${context} response: missing required field '${field}'`);
  }
  return record[field];
}

function decodeOptional(raw: unknown): string | null {
  if (raw === null || raw === undefined) {
    return null;
  }
  return String(raw);
}

function decodeIds(raw: unknown): bigint[] {
  if (!Array.isArray(raw)) {
    throw new Error('Invalid batch response: expected an array of ids');
  }
  return raw.map((id) => BigInt(String(id)));
}

function decodeDriverProfile(raw: unknown): DeliveryTypes.DeliveryDriverProfile {
  const record = requireRecord(raw, 'driver profile');
  return {
    driver: requireField(record, 'driver', 'driver profile') as string,
    reputation: requireField(record, 'reputation', 'driver profile') as bigint,
    completedDeliveries: requireField(record, 'completedDeliveries', 'driver profile') as bigint,
  };
}

function decodeCombinedState(raw: unknown): DeliveryTypes.CombinedDeliveryState {
  const record = requireRecord(raw, 'combined state');
  return {
    delivery: decodeDelivery(requireField(record, 'delivery', 'combined state')),
    escrowStatus: requireField(record, 'escrowStatus', 'combined state') as string,
  };
}

function decodeDeliveryPage(raw: unknown): DeliveryTypes.DeliveryPage {
  const record = requireRecord(raw, 'delivery page');
  const items = requireField(record, 'items', 'delivery page');
  if (!Array.isArray(items)) {
    throw new Error("Invalid delivery page response: field 'items' must be an array");
  }
  return {
    items: items.map((item) => decodeDelivery(item)),
    total: requireField(record, 'total', 'delivery page') as bigint,
  };
}

function encodeDeliveryMetadata(deliveryId: bigint, metadata: DeliveryTypes.DeliveryMetadata): unknown {
  return map([
    [symbol('delivery_id'), u64(deliveryId)],
    [symbol('description'), string(metadata.description)],
    [symbol('weight'), u32(metadata.weight)],
    [symbol('is_fragile'), bool(metadata.isFragile)],
  ]);
}
