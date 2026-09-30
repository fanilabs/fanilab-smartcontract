import {
  Contract,
  SorobanRpc,
  TransactionBuilder,
  Networks,
  Keypair,
  nativeToScVal,
  scValToNative,
  Address,
  xdr,
} from '@stellar/stellar-sdk';

/**
 * Optional pagination options for list-returning SDK methods.
 * When provided, the client routes to the corresponding `_page`
 * contract method instead of the default (offset: 0, limit: 100) call.
 */
export interface PaginationOptions {
  offset?: number;
  limit?: number;
}

const DEFAULT_OFFSET = 0;
const DEFAULT_LIMIT = 100;

function resolvePagination(options?: PaginationOptions): { offset: number; limit: number } {
  return {
    offset: options?.offset ?? DEFAULT_OFFSET,
    limit: options?.limit ?? DEFAULT_LIMIT,
  };
}

function hasPagination(options?: PaginationOptions): boolean {
  return options?.offset !== undefined || options?.limit !== undefined;
}

export class DeliveryClient {
  private contract: Contract;
  private server: SorobanRpc.Server;
  private networkPassphrase: string;

  constructor(contractId: string, rpcUrl: string, networkPassphrase: string = Networks.TESTNET) {
    this.contract = new Contract(contractId);
    this.server = new SorobanRpc.Server(rpcUrl);
    this.networkPassphrase = networkPassphrase;
  }

  async getDeliveriesBySender(
    sender: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const { offset, limit } = resolvePagination(options);
    const method = hasPagination(options) ? 'get_deliveries_page' : 'get_deliveries_by_sender';
    const args = hasPagination(options)
      ? [
          new Address(sender).toScVal(),
          nativeToScVal(offset, { type: 'u32' }),
          nativeToScVal(limit, { type: 'u32' }),
        ]
      : [new Address(sender).toScVal()];
    return this.invokeRead(method, args);
  }

  async getDeliveriesByRecipient(
    recipient: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const { offset, limit } = resolvePagination(options);
    const method = hasPagination(options) ? 'get_deliveries_page' : 'get_deliveries_by_recipient';
    const args = hasPagination(options)
      ? [
          new Address(recipient).toScVal(),
          nativeToScVal(offset, { type: 'u32' }),
          nativeToScVal(limit, { type: 'u32' }),
        ]
      : [new Address(recipient).toScVal()];
    return this.invokeRead(method, args);
  }

  async getDeliveriesByDriver(
    driver: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const { offset, limit } = resolvePagination(options);
    const method = hasPagination(options) ? "get_deliveries_page" : "get_deliveries_by_driver";
    const args = hasPagination(options)
      ? [
          new Address(driver).toScVal(),
          nativeToScVal(offset, { type: "u32" }),
          nativeToScVal(limit, { type: "u32" }),
        ]
      : [new Address(driver).toScVal()];
    return this.invokeRead(method, args);
  }


  private async invokeRead(method: string, args: xdr.ScVal[]): Promise<any[]> {
    const source = Keypair.random();
    const account = await this.server.getAccount(source.publicKey());
    const tx = new TransactionBuilder(account, {
      fee: '100',
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(30)
      .build();
    const simulated = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simulated)) {
      throw new Error(simulated.error);
    }
    const result = (simulated as SorobanRpc.Api.SimulateTransactionSuccessResponse).result;
    if (!result) {
      return [];
    }
    return scValToNative(result.retval) as any[];
  }
}

export class EscrowClient {
  private contract: Contract;
  private server: SorobanRpc.Server;
  private networkPassphrase: string;

  constructor(contractId: string, rpcUrl: string, networkPassphrase: string = Networks.TESTNET) {
    this.contract = new Contract(contractId);
    this.server = new SorobanRpc.Server(rpcUrl);
    this.networkPassphrase = networkPassphrase;
  }

  async getEscrowsBySender(
    sender: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const { offset, limit } = resolvePagination(options);
    const method = hasPagination(options) ? 'get_escrows_page' : 'get_escrows_by_sender';
    const args = hasPagination(options)
      ? [
          new Address(sender).toScVal(),
          nativeToScVal(offset, { type: 'u32' }),
          nativeToScVal(limit, { type: 'u32' }),
        ]
      : [new Address(sender).toScVal()];
    return this.invokeRead(method, args);
  }

  async getEscrowsByRecipient(
    recipient: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const { offset, limit } = resolvePagination(options);
    const method = hasPagination(options) ? 'get_escrows_page' : 'get_escrows_by_recipient';
    const args = hasPagination(options)
      ? [
          new Address(recipient).toScVal(),
          nativeToScVal(offset, { type: 'u32' }),
          nativeToScVal(limit, { type: 'u32' }),
        ]
      : [new Address(recipient).toScVal()];
    return this.invokeRead(method, args);
  }

  async getEscrowsByDriver(
    driver: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const { offset, limit } = resolvePagination(options);
    const method = hasPagination(options) ? 'get_escrows_page' : 'get_escrows_by_driver';
    const args = hasPagination(options)
      ? [
          new Address(driver).toScVal(),
          nativeToScVal(offset, { type: 'u32' }),
          nativeToScVal(limit, { type: 'u32' }),
        ]
      : [new Address(driver).toScVal()];
    return this.invokeRead(method, args);
  }

  private async invokeRead(method: string, args: xdr.ScVal[]): Promise<any[]> {
    const source = Keypair.random();
    const account = await this.server.getAccount(source.publicKey());
    const tx = new TransactionBuilder(account, {
      fee: '100',
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(30)
      .build();
    const simulated = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simulated)) {
      throw new Error(simulated.error);
    }
    const result = (simulated as SorobanRpc.Api.SimulateTransactionSuccessResponse).result;
    if (!result) {
      return [];
    }
    return scValToNative(result.retval) as any[];
  }
}
