import {
  Address,
  Contract,
  Keypair,
  Operation,
  SorobanDataBuilder,
  nativeToScVal,
  rpc as SorobanRpc,
  scValToNative,
  TransactionBuilder,
  xdr,
} from '@stellar/stellar-sdk';
import { ContractInvokeOptions } from '../types/common.types';
import { formatSorobanError } from './error-codes';

export type ScVal = xdr.ScVal;

export function address(value: string): ScVal {
  return new Address(value).toScVal();
}

export function u64(value: bigint | number): ScVal {
  return nativeToScVal(BigInt(value), { type: 'u64' });
}

export function u32(value: number): ScVal {
  return nativeToScVal(value, { type: 'u32' });
}

export function i128(value: bigint): ScVal {
  return nativeToScVal(value, { type: 'i128' });
}

export function bool(value: boolean): ScVal {
  return nativeToScVal(value);
}

export function symbol(value: string): ScVal {
  return xdr.ScVal.scvSymbol(value);
}

export function string(value: string): ScVal {
  return nativeToScVal(value);
}

export function bytes(value: Uint8Array): ScVal {
  return xdr.ScVal.scvBytes(Buffer.from(value));
}

export function vec(values: ScVal[]): ScVal {
  return xdr.ScVal.scvVec(values);
}

export function map(fields: Array<[string, ScVal]>): ScVal {
  return xdr.ScVal.scvMap(
    fields.map(([key, value]) =>
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol(key), val: value })
    )
  );
}

export function native(value: ScVal): unknown {
  return scValToNative(value);
}

/**
 * Hook invoked with the authorization entries produced by simulation, before
 * the transaction is assembled. Callers may inspect entries and sign them
 * (e.g. for contract-address based authorization) by returning the entries to
 * attach to the assembled transaction.
 */
export type AuthEntryHook = (
  entries: xdr.SorobanAuthorizationEntry[],
  context: AuthEntryContext
) =>
  | xdr.SorobanAuthorizationEntry[]
  | Promise<xdr.SorobanAuthorizationEntry[]>;

export interface AuthEntryContext {
  /** The unsigned transaction that was simulated. */
  transaction: import('@stellar/stellar-sdk').Transaction;
  /** The simulation result carrying the auth entries. */
  simulation: SorobanRpc.Api.SimulateTransactionSuccessResponse;
  /** Network passphrase used to build the transaction. */
  networkPassphrase: string;
  /** The latest ledger sequence at simulation time, if available. */
  latestLedger?: number;
}

const DEFAULT_TTL_THRESHOLD_LEDGERS = 100_000;
const DEFAULT_TTL_EXTEND_TO_LEDGERS = 518_400;

function canExtendTtl(key: xdr.LedgerKey): boolean {
  return key.type === 'contractCode' ||
    (key.type === 'contractData' && key.contractData.durability.name === 'persistent');
}

export function findExpiringPersistentKeys(
  entries: Array<{ key: xdr.LedgerKey; liveUntilLedgerSeq?: number }>,
  latestLedger: number,
  thresholdLedgers: number
): xdr.LedgerKey[] {
  return entries
    .filter(
      (entry) =>
        canExtendTtl(entry.key) &&
        entry.liveUntilLedgerSeq !== undefined &&
        entry.liveUntilLedgerSeq > latestLedger &&
        entry.liveUntilLedgerSeq - latestLedger <= thresholdLedgers
    )
    .map((entry) => entry.key);
}

function abortError(): Error {
  const error = new Error('Contract invocation was aborted');
  error.name = 'AbortError';
  return error;
}

function delay(milliseconds: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(abortError());
      return;
    }

    const timer = setTimeout(() => {
      signal?.removeEventListener('abort', onAbort);
      resolve();
    }, milliseconds);
    const onAbort = () => {
      clearTimeout(timer);
      signal?.removeEventListener('abort', onAbort);
      reject(abortError());
    };
    signal?.addEventListener('abort', onAbort, { once: true });
  });
}

function awaitPollResult<T>(
  request: Promise<T>,
  timeoutMs: number,
  timeoutMessage: string,
  signal?: AbortSignal
): Promise<T> {
  return new Promise((resolve, reject) => {
    // Attach a rejection handler immediately so a rejected request never
    // surfaces as an unhandled rejection, even when we bail out early below.
    const settled = request.then(
      (result) => ({ ok: true as const, result }),
      (error: unknown) => ({ ok: false as const, error })
    );

    if (signal?.aborted) {
      reject(abortError());
      return;
    }

    const finish = (complete: () => void) => {
      clearTimeout(timer);
      signal?.removeEventListener('abort', onAbort);
      complete();
    };
    const timer = setTimeout(() => finish(() => reject(new Error(timeoutMessage))), timeoutMs);
    const onAbort = () => finish(() => reject(abortError()));
    signal?.addEventListener('abort', onAbort, { once: true });
    settled.then((outcome) =>
      finish(() => (outcome.ok ? resolve(outcome.result) : reject(outcome.error)))
    );
  });
}

/**
 * Validate numeric maintenance/TTL options up front so configuration mistakes
 * fail fast, before any RPC (account fetch or simulation) is issued.
 */
function validateMaintenanceOptions(config: ContractInvokeOptions): void {
  const numericOptions: Array<[keyof ContractInvokeOptions, number | undefined]> = [
    ['ttlThresholdLedgers', config.ttlThresholdLedgers],
    ['ttlExtendToLedgers', config.ttlExtendToLedgers],
  ];
  for (const [name, value] of numericOptions) {
    if (value === undefined) {
      continue;
    }
    if (typeof value !== 'number' || !Number.isFinite(value) || !Number.isInteger(value) || value < 0) {
      throw new Error(
        `Invalid ${String(name)}: expected a non-negative integer, received ${String(value)}`
      );
    }
  }
  if (
    config.ttlThresholdLedgers !== undefined &&
    config.ttlExtendToLedgers !== undefined &&
    config.ttlExtendToLedgers < config.ttlThresholdLedgers
  ) {
    throw new Error(
      `Invalid TTL configuration: ttlExtendToLedgers (${config.ttlExtendToLedgers}) must be >= ttlThresholdLedgers (${config.ttlThresholdLedgers})`
    );
  }
}

export class ContractInvoker {
  private readonly contract: Contract;
  private readonly defaults: ContractInvokeOptions;

  constructor(contractId: string, defaults: ContractInvokeOptions = {}) {
    this.contract = new Contract(contractId);
    this.defaults = defaults;
  }

  async call(
    functionName: string,
    args: ScVal[],
    options: ContractInvokeOptions = {}
  ): Promise<unknown> {
    const config = { ...this.defaults, ...options };
    // Validate maintenance/TTL configuration before any RPC is issued.
    validateMaintenanceOptions(config);
    const server = this.server(config);
    const accountId = this.require(config.sourceAccount, 'sourceAccount');
    const networkPassphrase = this.require(config.networkPassphrase, 'networkPassphrase');
    const buildTransaction = async () => {
      const account = await server.getAccount(accountId);
      return new TransactionBuilder(account, {
        fee: config.fee ?? '100000',
        networkPassphrase,
      })
        .setTimeout(config.timeoutSeconds ?? 30)
        .addOperation(this.contract.call(functionName, ...args))
        .build();
    };
    let transaction = await buildTransaction();
    let simulation = await server.simulateTransaction(transaction);
    if (SorobanRpc.Api.isSimulationError(simulation)) {
      throw new Error(formatSorobanError('Soroban simulation failed', simulation.error));
    }
    if (!config.signer) {
      return simulation.result?.retval ? native(simulation.result.retval) : undefined;
    }

    const expiringKeys = await this.expiringFootprintKeys(
      server,
      simulation.transactionData,
      config
    );
    if (expiringKeys.length > 0) {
      await this.extendFootprintTtl(
        server,
        accountId,
        networkPassphrase,
        simulation.transactionData,
        expiringKeys,
        config
      );
      transaction = await buildTransaction();
      simulation = await server.simulateTransaction(transaction);
      if (SorobanRpc.Api.isSimulationError(simulation)) {
        throw new Error(formatSorobanError('Soroban simulation failed', simulation.error));
      }
    }

    const assembled = SorobanRpc.assembleTransaction(transaction, simulation);
    if (config.authEntryHook) {
      const authEntries = assembled.getAuth();
      const signedEntries = await config.authEntryHook(authEntries, {
        transaction,
        simulation,
        networkPassphrase,
        latestLedger: simulation.latestLedger,
      });
      assembled.setSorobanData(
        new SorobanDataBuilder(assembled.build().sorobanData).setAuth(signedEntries).build()
      );
    }
    const prepared = assembled.build();
    prepared.sign(config.signer);
    const submitted = await server.sendTransaction(prepared);
    if (submitted.status === 'ERROR') {
      throw new Error(
        formatSorobanError(
          'Soroban submission failed',
          submitted.errorResult?.toString() ?? 'unknown error'
        )
      );
    }
    if (submitted.status === 'PENDING') {
      return this.waitForResult(server, submitted.hash, config);
    }
    return undefined;
  }

  private async expiringFootprintKeys(
    server: SorobanRpc.Server,
    transactionData: Soro

/* … truncated 4054 chars — edit only what you need near the top … */
