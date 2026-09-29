/**
 * Typed SDK client for FleetManagementContract
 */

import { ContractInvokeOptions, DriverFleetStatus, FleetProfile, PendingTreasuryChange } from '../types/common.types';
import * as FleetManagementTypes from '../types/fleet_management.types';
import { ContractInvoker, address, u32, u64, vec } from './invoker';

/**
 * Maximum roster entries the contract returns for a single `get_fleet_roster`
 * call, mirroring `MAX_ROSTER_PAGE_SIZE` in fleet_management_contract. The
 * contract clamps any larger `limit`, so this is a client-side convenience
 * default for a single full-page read.
 */
const DEFAULT_ROSTER_PAGE_SIZE = 100;

export class FleetManagementClient {
  private readonly invoker: ContractInvoker;

  constructor(contractId: string, options: ContractInvokeOptions = {}) {
    this.invoker = new ContractInvoker(contractId, options);
  }

  async init(admin: string, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('init', [address(admin)], options);
  }

  async setIdentityContract(params: FleetManagementTypes.SetIdentityContractParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('set_identity_contract', [address(params.admin), address(params.identityContract)], options);
  }

  async setEscrowContract(params: FleetManagementTypes.SetEscrowContractParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('set_escrow_contract', [address(params.admin), address(params.escrowContract)], options);
  }

  async registerFleet(params: FleetManagementTypes.RegisterFleetParams, options?: ContractInvokeOptions): Promise<bigint> {
    return BigInt(String(await this.invoker.call('register_fleet', [address(params.owner), address(params.treasury)], options)));
  }

  async getFleet(fleetId: bigint, options?: ContractInvokeOptions): Promise<FleetProfile> {
    return decodeFleet(await this.invoker.call('get_fleet', [u64(fleetId)], options));
  }

  async deactivateFleet(params: FleetManagementTypes.DeactivateFleetParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('deactivate_fleet', [address(params.caller), u64(params.fleetId)], options);
  }

  async adminReassignFleetOwner(params: FleetManagementTypes.AdminReassignFleetOwnerParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('admin_reassign_fleet_owner', [address(params.admin), u64(params.fleetId), address(params.newOwner)], options);
  }

  async adminForceUpdateTreasury(params: FleetManagementTypes.AdminForceUpdateTreasuryParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('admin_force_update_treasury', [address(params.admin), u64(params.fleetId), address(params.newTreasury)], options);
  }

  async updateFleetTreasury(params: FleetManagementTypes.UpdateFleetTreasuryParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('update_fleet_treasury', [address(params.owner), u64(params.fleetId), address(params.treasury), vec((params.coSigners ?? []).map(address))], options);
  }

  async confirmFleetTreasuryUpdate(params: FleetManagementTypes.ConfirmFleetTreasuryUpdateParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('confirm_fleet_treasury_update', [u64(params.fleetId)], options);
  }

  async getPendingTreasuryUpdate(fleetId: bigint, options?: ContractInvokeOptions): Promise<PendingTreasuryChange | null> {
    const value = await this.invoker.call('get_pending_treasury_update', [u64(fleetId)], options);
    return value == null ? null : decodePendingTreasury(value);
  }

  async addDriverToFleet(params: FleetManagementTypes.AddDriverToFleetParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('add_driver_to_fleet', [address(params.caller), u64(params.fleetId), address(params.driver), vec((params.coSigners ?? []).map(address))], options);
  }

  async cancelInvite(params: FleetManagementTypes.CancelInviteParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('cancel_invite', [address(params.owner), u64(params.fleetId), address(params.driver), vec((params.coSigners ?? []).map(address))], options);
  }

  async acceptFleetInvite(params: FleetManagementTypes.AcceptFleetInviteParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('accept_fleet_invite', [u64(params.fleetId), address(params.driver)], options);
  }

  async removeDriverFromFleet(params: FleetManagementTypes.RemoveDriverFromFleetParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('remove_driver_from_fleet', [u64(params.fleetId), address(params.caller), address(params.driver), vec((params.coSigners ?? []).map(address))], options);
  }

  async getPayoutAddress(driver: string, fleetId: bigint, options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_payout_address', [address(driver), u64(fleetId)], options));
  }

  async getDriverFleetStatus(fleetId: bigint, driver: string, options?: ContractInvokeOptions): Promise<DriverFleetStatus | null> {
    const result = await this.invoker.call('get_driver_fleet_status', [u64(fleetId), address(driver)], options);
    return result == null ? null : (result as DriverFleetStatus);
  }

  async getFleetRoster(params: FleetManagementTypes.GetFleetRosterParams, options?: ContractInvokeOptions): Promise<string[]> {
    const offset = params.offset ?? 0;
    const limit = params.limit ?? DEFAULT_ROSTER_PAGE_SIZE;
    const result = await this.invoker.call(
      'get_fleet_roster',
      [u64(params.fleetId), u32(offset), u32(limit)],
      options
    );
    return (result as unknown[]).map((value) => String(value));
  }

  async configureSigners(params: FleetManagementTypes.ConfigureSignersParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('configure_signers', [address(params.owner), u64(params.fleetId), params.signers.map(address), u32(params.threshold)], options);
  }

  async getFleetSigners(fleetId: bigint, options?: ContractInvokeOptions): Promise<{ signers: string[]; threshold: number }> {
    const result = await this.invoker.call('get_fleet_signers', [u64(fleetId)], options) as unknown[];
    const signers = (result[0] as unknown[]).map((value) => String(value));
    return { signers, threshold: Number(result[1]) };
  }
}

function decodeFleet(value: unknown): FleetProfile {
  const profile = value as Record<string, unknown>;
  return {
    fleetId: BigInt(String(profile.fleet_id)),
    owner: String(profile.owner),
    treasury: String(profile.treasury),
    totalActiveDrivers: Number(profile.total_active_drivers),
    signers: ((profile.signers as unknown[]) ?? []).map((value) => String(value)),
    signatureThreshold: Number(profile.signature_threshold),
    active: Boolean(profile.active),
  };
}

function decodePendingTreasury(value: unknown): PendingTreasuryChange {
  const change = value as Record<string, unknown>;
  return {
    treasury: String(change.treasury),
    activatesAt: Number(change.activates_at),
  };
}
