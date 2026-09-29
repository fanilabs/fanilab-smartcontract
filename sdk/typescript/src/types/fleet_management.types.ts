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
  /** Additional signer authorizations needed to meet the fleet's signature_threshold beyond `owner` alone. Pass [] for a threshold-1 fleet. */
  coSigners?: string[];
}

export interface ConfirmFleetTreasuryUpdateParams {
  fleetId: bigint;
}

export interface AddDriverToFleetParams {
  caller: string;
  fleetId: bigint;
  driver: string;
  /** Additional signer authorizations needed to meet the fleet's signature_threshold beyond `caller` alone. Pass [] for a threshold-1 fleet. */
  coSigners?: string[];
}

export interface CancelInviteParams {
  owner: string;
  fleetId: bigint;
  driver: string;
  /** Additional signer authorizations needed to meet the fleet's signature_threshold beyond `owner` alone. Pass [] for a threshold-1 fleet. */
  coSigners?: string[];
}

export interface AcceptFleetInviteParams {
  fleetId: bigint;
  driver: string;
}

export interface RemoveDriverFromFleetParams {
  fleetId: bigint;
  caller: string;
  driver: string;
  /** Additional signer authorizations needed to meet the fleet's signature_threshold beyond `caller` alone. Pass [] (or omit) when the driver is removing themselves. */
  coSigners?: string[];
}

export interface ConfigureSignersParams {
  owner: string;
  fleetId: bigint;
  signers: string[];
  threshold: number;
}

export { DriverFleetStatus, FleetProfile, PendingTreasuryChange };

export interface GetFleetRosterParams {
  fleetId: bigint;
  /** Roster index to start reading from. Defaults to 0. */
  offset?: number;
  /**
   * Maximum number of drivers to return. Defaults to
   * {@link DEFAULT_ROSTER_PAGE_SIZE} and is clamped by the contract to
   * {@link MAX_ROSTER_PAGE_SIZE}.
   */
  limit?: number;
}
