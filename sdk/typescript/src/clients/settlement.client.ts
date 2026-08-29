/**
 * Typed SDK client for SettlementContract
 */

import { ContractInvokeOptions } from '../types/common.types';
import * as SettlementTypes from '../types/settlement.types';
import { ContractInvoker, address, i128, u64 } from './invoker';

export class SettlementClient {
  private readonly invoker: ContractInvoker;

  constructor(contractId: string, options: ContractInvokeOptions = {}) {
    this.invoker = new ContractInvoker(contractId, options);
  }

  async init(params: SettlementTypes.InitParams, options?: ContractInvokeOptions): Promise<void> {
    await this.invoker.call('init', [address(params.admin)], options);
  }

  async getAdmin(options?: ContractInvokeOptions): Promise<string> {
    return String(await this.invoker.call('get_admin', [], options));
  }

  async getDriverPreference(driver: string, options?: ContractInvokeOptions): Promise<string | null> {
    const value = await this.invoker.call('get_driver_preference', [address(driver)], options);
    return value === null || value === undefined ? null : String(value);
  }

  async executeSettlementSwap(
    params: SettlementTypes.ExecuteSettlementSwapParams,
    options?: ContractInvokeOptions
  ): Promise<void> {
    await this.invoker.call(
      'execute_settlement_swap',
      [
        address(params.caller),
        address(params.fromToken),
        address(params.toToken),
        address(params.recipient),
        i128(params.amount),
        i128(params.minAmountOut),
      ],
      options
    );
  }
}
