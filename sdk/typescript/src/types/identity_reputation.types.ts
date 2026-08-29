import { DriverProfile, DriverTier, ReputationConfig, UserProfile } from './common.types';

export interface InitParams {
  admin: string;
  deliveryContract: string;
  disputeContract: string;
}

export interface SetAuthorizedContractParams {
  admin: string;
  contractAddr: string;
  authorized: boolean;
}

export interface SetReputationConfigParams {
  admin: string;
  config: ReputationConfig;
}

export interface SetDeliveryContractParams {
  admin: string;
  deliveryContract: string;
}

export interface SetDisputeContractParams {
  admin: string;
  disputeContract: string;
}

export interface RegisterDriverParams {
  driver: string;
}

export interface RegisterUserParams {
  user: string;
}

export interface UpdateDriverKycStatusParams {
  admin: string;
  driver: string;
  kycVerified: boolean;
}

export interface IncreaseReputationParams {
  caller: string;
  driver: string;
  deliveryId: bigint;
  weightGrams: number;
  fragile: boolean;
}

export interface DecreaseReputationParams {
  caller: string;
  driver: string;
  points: number;
}

export interface AwardReputationParams {
  caller: string;
  driver: string;
  points: number;
}

export { DriverProfile, DriverTier, ReputationConfig, UserProfile };
