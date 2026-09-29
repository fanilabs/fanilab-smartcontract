import {
  Contract,
  SorobanRpc,
  TransactionBuilder,
  Networks,
  Keypair,
  nativeToScVal,
  scValToNative,
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

/**
 * EscrowClient wraps the on-chain escrow contract and exposes
 * typed helpers for creating and querying escrows and deliveries.
 */
export class EscrowClient {
  private contract: Contract;
  private server: SorobanRpc.Server;
  private networkPassphrase: string;
  private sourceKeypair: Keypair;

  constructor(
    contractId: string,
    server: SorobanRpc.Server,
    networkPassphrase: string = Networks.TESTNET,
    sourceSecret?: string,
  ) {
    this.contract = new Contract(contractId);
    this.server = server;
    this.networkPassphrase = networkPassphrase;
    this.sourceKeypair = sourceSecret
      ? Keypair.fromSecret(sourceSecret)
      : Keypair.random();
  }

  private async simulate(method: string, args: xdr.ScVal[]): Promise<any> {
    const account = await this.server.getAccount(
      this.sourceKeypair.publicKey(),
    );
    const tx = new TransactionBuilder(account, {
      fee: '100',
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(30)
      .build();

    const result = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(result)) {
      throw new Error(`Simulation failed for ${method}: ${result.error}`);
    }
    const retval = (result as SorobanRpc.Api.SimulateTransactionSuccessResponse)
      .result?.retval;
    return retval ? scValToNative(retval) : undefined;
  }

  private paginationArgs(options?: PaginationOptions): xdr.ScVal[] {
    return [
      nativeToScVal(options?.offset ?? DEFAULT_OFFSET, { type: 'u32' }),
      nativeToScVal(options?.limit ?? DEFAULT_LIMIT, { type: 'u32' }),
    ];
  }

  async getDeliveriesBySender(
    sender: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const method = options ? 'get_deliveries_page' : 'get_deliveries_by_sender';
    const args = options
      ? [nativeToScVal(sender, { type: 'address' }), ...this.paginationArgs(options)]
      : [nativeToScVal(sender, { type: 'address' })];
    return this.simulate(method, args);
  }

  async getDeliveriesByRecipient(
    recipient: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const method = options
      ? 'get_deliveries_page'
      : 'get_deliveries_by_recipient';
    const args = options
      ? [
          nativeToScVal(recipient, { type: 'address' }),
          ...this.paginationArgs(options),
        ]
      : [nativeToScVal(recipient, { type: 'address' })];
    return this.simulate(method, args);
  }

  async getEscrowsBySender(
    sender: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const method = options ? 'get_escrows_page' : 'get_escrows_by_sender';
    const args = options
      ? [nativeToScVal(sender, { type: 'address' }), ...this.paginationArgs(options)]
      : [nativeToScVal(sender, { type: 'address' })];
    return this.simulate(method, args);
  }

  async getEscrowsByRecipient(
    recipient: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const method = options ? 'get_escrows_page' : 'get_escrows_by_recipient';
    const args = options
      ? [
          nativeToScVal(recipient, { type: 'address' }),
          ...this.paginationArgs(options),
        ]
      : [nativeToScVal(recipient, { type: 'address' })];
    return this.simulate(method, args);
  }

  async getEscrowsByDriver(
    driver: string,
    options?: PaginationOptions,
  ): Promise<any[]> {
    const method = options ? 'get_escrows_page' : 'get_escrows_by_driver';
    const args = options
      ? [nativeToScVal(driver, { type: 'address' }), ...this.paginationArgs(options)]
      : [nativeToScVal(driver, { type: 'address' })];
    return this.simulate(method, args);
  }
}
