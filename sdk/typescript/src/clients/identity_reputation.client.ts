/**
 * Typed SDK client for IdentityReputationContract
 */

import { ContractInvokeOptions, DriverProfile, DriverTier, ReputationConfig, UserProfile } from '../types/common.types';
import * as IdentityReputationTypes from '../types/identity_reputation.types';
import { ContractInvoker, address, bool, u32, u64 } from './invoker';

export class IdentityReputationClient {
  private readonly invoker: ContractInvoker;

  constructor(contractId: string, options: ContractInvokeOptions = {}) {
    this.invoker = new ContractInvoker(contractId, options);
  }

  async init(params: IdentityReputationTypes.InitParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('init', [address(params.admin), address(params.deliveryContract), address(params.disputeContract)], options);
  }

  async getAdmin(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_admin', [], options));
  }

  async setAuthorizedContract(
    params: IdentityReputationTypes.SetAuthorizedContractParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('set_authorized_contract', [address(params.admin), address(params.contractAddr), bool(params.authorized)], options);
  }

  async setReputationConfig(
    params: IdentityReputationTypes.SetReputationConfigParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('set_reputation_config', [address(params.admin), mapConfig(params.config)], options);
  }

  async getReputationConfig(options?: ContractInvokeOptions): Promise<ReputationConfig> {
    return decodeReputationConfig(await this.invoker.call('get_reputation_config', [], options));
  }

  async setDeliveryContract(params: IdentityReputationTypes.SetDeliveryContractParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('set_delivery_contract', [address(params.admin), address(params.deliveryContract)], options);
  }

  async setDisputeContract(params: IdentityReputationTypes.SetDisputeContractParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('set_dispute_contract', [address(params.admin), address(params.disputeContract)], options);
  }

  async getDeliveryContract(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_delivery_contract', [], options));
  }

  async getDisputeContract(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_dispute_contract', [], options));
  }

  async isAuthorizedContract(contractAddr: string, options?: ContractInvokeOptions): Promise<boolean> {
    return Boolean(await this.invoker.call('is_authorized_contract', [address(contractAddr)], options));
  }

  async hasDriverProfile(driver: string, options?: ContractInvokeOptions): Promise<boolean> {
    return Boolean(await this.invoker.call('has_driver_profile', [address(driver)], options));
  }

  async registerDriver(driver: string, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('register_driver', [address(driver)], options);
  }

  async registerUser(user: string, options?: ContractInvokeOptions): Promise<UserProfile> {
    return decodeUserProfile(await this.invoker.call('register_user', [address(user)], options));
  }

  async getUserProfile(user: string, options?: ContractInvokeOptions): Promise<UserProfile> {
    return decodeUserProfile(await this.invoker.call('get_user_profile', [address(user)], options));
  }

  async hasUserProfile(user: string, options?: ContractInvokeOptions): Promise<boolean> {
    return Boolean(await this.invoker.call('has_user_profile', [address(user)], options));
  }

  async getDriverProfile(driver: string, options?: ContractInvokeOptions): Promise<DriverProfile> {
    return decodeDriverProfile(await this.invoker.call('get_driver_profile', [address(driver)], options));
  }

  async updateDriverKycStatus(
    params: IdentityReputationTypes.UpdateDriverKycStatusParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call('update_driver_kyc_status', [address(params.admin), address(params.driver), bool(params.kycVerified)], options);
  }

  async increaseReputation(params: IdentityReputationTypes.IncreaseReputationParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call(
      'increase_reputation',
      [address(params.caller), address(params.driver), u64(params.deliveryId), u32(params.weightGrams), bool(params.fragile)],
      options
    );
  }

  async decreaseReputation(params: IdentityReputationTypes.DecreaseReputationParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('decrease_reputation', [address(params.caller), address(params.driver), u32(params.points)], options);
  }

  async awardReputation(params: IdentityReputationTypes.AwardReputationParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('award_reputation', [address(params.caller), address(params.driver), u32(params.points)], options);
  }

  async getDriverTier(driver: string, options?: ContractInvokeOptions): Promise<DriverTier> {
    return (await this.invoker.call('get_driver_tier', [address(driver)], options)) as DriverTier;
  }

  async isEligibleForEnterprise(driver: string, options?: ContractInvokeOptions): Promise<boolean> {
    return Boolean(await this.invoker.call('is_eligible_for_enterprise', [address(driver)], options));
  }
}

function mapConfig(config: ReputationConfig): unknown {
  return {
    base_points: config.basePoints,
    heavy_cargo_points: config.heavyCargoPoints,
    fragile_points: config.fragilePoints,
  };
}

function decodeReputationConfig(value: unknown): ReputationConfig {
  const config = value as Record<string, unknown>;
  return {
    basePoints: Number(config.base_points ?? 0),
    heavyCargoPoints: Number(config.heavy_cargo_points ?? 0),
    fragilePoints: Number(config.fragile_points ?? 0),
  };
}

function decodeUserProfile(value: unknown): UserProfile {
  const profile = value as Record<string, unknown>;
  return {
    address: String(profile.address),
    registeredAt: Number(profile.registered_at ?? 0),
  };
}

function decodeDriverProfile(value: unknown): DriverProfile {
  const profile = value as Record<string, unknown>;
  return {
    address: String(profile.address),
    deliveriesCompleted: Number(profile.deliveries_completed ?? 0),
    reputationScore: Number(profile.reputation_score ?? 0),
    registeredAt: Number(profile.registered_at ?? 0),
    kycVerified: Boolean(profile.kyc_verified),
  };
}
