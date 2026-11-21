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

/**
 * Opt-in receipt describing a successful write invocation. Returned only when
 * `ContractInvokeOptions.returnReceipt` is enabled; otherwise `call` keeps
 * returning the native return value so existing callers are unaffected.
 */
export interface InvocationReceipt {
  /** Transaction hash of the submitted invocation. */
  hash: string;
  /** Ledger sequence in which the transaction was included. */
  ledger: number;
  /** Native return value of the invoked contract function, if any. */
  result: unknown;
}

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

const DEFAULT_TTL_THRESHOLD_LEDGERS = 100_000;
const DEFAULT_TTL_EXTEND_TO_LEDGERS = 518_400;

function canExtendTtl(key: xdr.LedgerKey): boolean {
  return key.type === 'contractCode' ||
    (key.type === 'contractData' && key.contractData.durability.name === 'persistent');
}

function ledgerKeyId(key: xdr.LedgerKey): string {
  return key.toXDR('base64');
}

function dedupeLedgerKeys(keys: xdr.LedgerKey[]): xdr.LedgerKey[] {
  const seen = new Set<string>();
  const unique: xdr.LedgerKey[] = [];
  for (const key of keys) {
    const id = ledgerKeyId(key);
    if (seen.has(id)) {
      continue;
    }
    seen.add(id);
    unique.push(key);
  }
  return unique;
}

export function findExpiringPersistentKeys(
  entries: Array<{ key: xdr.LedgerKey; liveUntilLedgerSeq?: number }>,
  latestLedger: number,
  thresholdLedgers: number
): xdr.LedgerKey[] {
  return dedupeLedgerKeys(
    entries
      .filter(
        (entry) =>
          canExtendTtl(entry.key) &&
          entry.liveUntilLedgerSeq !== undefined &&
          entry.liveUntilLedgerSeq > latestLedger &&
          entry.liveUntilLedgerSeq - latestLedger <= thresholdLedgers
      )
      .map((entry) => entry.key)
  );
}

/**
 * Resolve the effective TTL extension target, honoring the network's maximum
 * entry lifetime. Caller-provided values above the ceiling are clamped (with a
 * warning) so the extension still succeeds; values that cannot be satisfied
 * (e.g. not above the threshold) are rejected with a clear explanation.
 */
export function resolveTtlExtendTo(
  requestedExtendTo: number,
  thresholdLedgers: number,
  networkMaxTtlLedgers: number
): number {
  if (!Number.isSafeInteger(networkMaxTtlLedgers) || networkMaxTtlLedgers <= 0) {
    throw new RangeError('network maximum TTL must be a positive integer');
  }
  if (!Number.isSafeInteger(requestedExtendTo) || requestedExtendTo <= thresholdLedgers) {
    throw new RangeError('ttlExtendToLedgers must be greater than ttlThresholdLedgers');
  }
  if (thresholdLedgers >= networkMaxTtlLedgers) {
    throw new RangeError(
      `ttlThresholdLedgers (${thresholdLedgers}) must be less than the network maximum TTL (${networkMaxTtlLedgers})`
    );
  }
  if (requestedExtendTo > networkMaxTtlLedgers) {
    // eslint-disable-next-line no-console
    console.warn(
      `ttlExtendToLedgers (${requestedExtendTo}) exceeds the network maximum TTL (${networkMaxTtlLedgers}); clamping to the network maximum`
    );
    return networkMaxTtlLedgers;
  }
  return requestedExtendTo;
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
    request.then(
      (result) => finish(() => resolve(result)),
      (error: unknown) => finish(() => reject(error))
    );
  });
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

    const prepared = SorobanRpc.assembleTransaction(transaction, simulation).build();
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
      const result = await this.waitForResult(server, submitted.hash, config);
      if (config.returnReceipt) {
        const receipt = await this.buildReceipt(server, submitted.hash, result, config);
        return receipt;
      }
      return result;
    }
    return undefined;
  }

  /**
   * Build an invocation receipt for a confirmed transaction, resolving the
   * inclusion ledger from the network. Falls back to the latest known ledger
   * when the transaction response does not expose one.
   */
  private async buildReceipt(
    server: SorobanRpc.Server,
    hash: string,
    result: unknown,
    config: ContractInvokeOptions
  ): Promise<InvocationReceipt> {
    let ledger = 0;
    try {
      const response = await server.getTransaction(hash);
      if (response && typeof response.ledger === 'number') {
        ledger = response.ledger;
      }
    } catch {
      // Fall through to latest ledger lookup below.
    }
    if (ledger === 0) {
      try {
        const latest = await server.getLatestLedger();
        ledger = latest.sequence;
      } catch {
        ledger = 0;
      }
    }
    return { hash, ledger, result };
  }

/* … truncated 4716 chars — edit only what you need near the top … */
