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

/**
 * DeliveryClient wraps the on-chain delivery contract and exposes
 * typed helpers for reading and mutating delivery records.
 */
export class DeliveryClient {
  private contract: Contract;
  private server: SorobanRpc.Server;
  private networkPassphrase: string;
  private sourceKeypair?: Keypair;

  constructor(
    contractId: string,
    rpcUrl: string,
    networkPassphrase: string = Networks.TESTNET,
    sourceKeypair?: Keypair,
  ) {
    this.contract = new Contract(contractId);
    this.server = new SorobanRpc.Server(rpcUrl);
    this.networkPassphrase = networkPassphrase;
    this.sourceKeypair = sourceKeypair;
  }

  /**
   * Fetch deliveries sent by a given address.
   *
   * When `pagination` is supplied, the call is routed to the
   * `get_deliveries_page` contract method so integrators can page
   * beyond the default 100-record window.
   */
  async getDeliveriesBySender(
    sender: string,
    pagination?: PaginationOptions,
  ): Promise<any[]> {
    return this.getDeliveries('get_deliveries_by_sender', 'get_deliveries_page', sender, pagination);
  }

  /**
   * Fetch deliveries received by a given address.
   *
   * When `pagination` is supplied, the call is routed to the
   * `get_deliveries_page` contract method.
   */
  async getDeliveriesByRecipient(
    recipient: string,
    pagination?: PaginationOptions,
  ): Promise<any[]> {
    return this.getDeliveries('get_deliveries_by_recipient', 'get_deliveries_page', recipient, pagination);
  }

  /**
   * Get all deliveries for a specific driver.
   * If pagination is provided, it falls back to the index-based
   * `get_deliveries_page` contract method.
   */
  async getDeliveriesByDriver(
    driver: string,
    pagination?: PaginationOptions,
  ): Promise<any[]> {
    return this.getDeliveries('get_deliveries_by_driver', 'get_deliveries_page', driver, pagination);
  }

  /**
   * Shared read path for delivery list methods. Falls back to the
   * non-paginated contract method when no pagination options are given,
   * preserving backward-compatible behavior.
   */
  private async getDeliveries(
    defaultMethod: string,
    pageMethod: string,
    address: string,
    pagination?: PaginationOptions,
  ): Promise<any[]> {
    const usePage = pagination !== undefined && (pagination.offset !== undefined || pagination.limit !== undefined);

    const method = usePage ? pageMethod : defaultMethod;
    const offset = pagination?.offset ?? DEFAULT_OFFSET;
    const limit = pagination?.limit ?? DEFAULT_LIMIT;

    const args = usePage
      ? [
          new Address(address).toScVal(),
          nativeToScVal(offset, { type: 'u32' }),
          nativeToScVal(limit, { type: 'u32' }),
        ]
      : [new Address(address).toScVal()];

    const result = await this.simulateRead(method, args);
    return (result ?? []) as any[];
  }

  /**
   * Simulate a read-only contract invocation and decode the result.
   */
  private async simulateRead(method: string, args: xdr.ScVal[]): Promise<any> {
    const source = this.sourceKeypair?.publicKey();
    if (!source) {
      throw new Error('A source keypair is required to simulate contract reads');
    }

    const account = await this.server.getAccount(source);
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

    const retval = (simulated as SorobanRpc.Api.SimulateTransactionSuccessResponse).result?.retval;
    if (!retval) {
      return undefined;
    }
    return scValToNative(retval);
  }
}
