import { DriverFleetStatus, FleetProfile, PendingTreasuryChange } from './common.types';

export interface RegisterFleetParams {
  owner: string;
  treasury: string;
}

export interface SetIdentityContractParams {
  admin: string;
  identityContract: string;
}

export interface SetEscrowContractParams {
  admin: string;
  escrowContract: string;
}

export interface FleetIdParams {
  fleetId: bigint;
}

export interface DeactivateFleetParams {
  caller: string;
  fleetId: bigint;
}

export interface AdminReassignFleetOwnerParams {
  admin: string;
  fleetId: bigint;
  newOwner: string;
}

export interface AdminForceUpdateTreasuryParams {
  admin: string;
  fleetId: bigint;
  newTreasury: string;
}

export interface UpdateFleetTreasuryParams {
  owner: string;
  fleetId: bigint;
  treasury: string;
}

export interface ConfirmFleetTreasuryUpdateParams {
  fleetId: bigint;
}

export interface AddDriverToFleetParams {
  caller: string;
  fleetId: bigint;
  driver: string;
}

export interface CancelInviteParams {
  owner: string;
  fleetId: bigint;
  driver: string;
}

export interface AcceptFleetInviteParams {
  fleetId: bigint;
  driver: string;
}

export interface RemoveDriverFromFleetParams {
  fleetId: bigint;
  caller: string;
  driver: string;
}

export interface ConfigureSignersParams {
  owner: string;
  fleetId: bigint;
  signers: string[];
  threshold: number;
}

export { DriverFleetStatus, FleetProfile, PendingTreasuryChange };
