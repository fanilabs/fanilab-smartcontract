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
 * Explicit invocation mode. `simulate` never submits a transaction and returns
 * the simulated return value; `submit` requires a signer and submits the
 * transaction, returning the native return value (or a receipt when requested).
 */
export type InvocationMode = 'simulate' | 'submit';

/**
 * Raised when a mutation is attempted without a signer. Previously this
 * silently degraded into a simulation and resolved `void`, hiding the mistake.
 */
export class MissingSignerError extends Error {
  constructor(functionName: string) {
    super(
      `Cannot submit mutation '${functionName}' without a signer. ` +
        `Pass a signer to submit, or use mode: 'simulate' to simulate explicitly.`
    );
    this.name = 'MissingSignerError';
  }
}

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

const U64_MAX = (1n << 64n) - 1n;

export function u64(value: bigint | number): ScVal {
  let big: bigint;
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value)) {
      throw new RangeError(
        'u64 number input must be a safe integer; use bigint for values outside the safe integer range'
      );
    }
    big = BigInt(value);
  } else {
    big = value;
  }
  if (big < 0n || big > U64_MAX) {
    throw new RangeError('u64 value must be between 0 and 2^64-1');
  }
  return nativeToScVal(big, { type: 'u64' });
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
const LEDGER_ENTRY_BATCH_LIMIT = 200;

const DEFAULT_SUBMISSION_MAX_ATTEMPTS = 5;
const DEFAULT_SUBMISSION_RETRY_DELAY_MS = 1_000;

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

function throwIfAborted(signal?: AbortSignal): void {
  if (signal?.aborted) {
    throw abortError();
  }
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

/**
 * Serializes submissions per source account so concurrent calls from the same
 * account do not race on sequence numbers. Each account has its own tail
 * promise; a new task chains onto the previous one and always releases the
 * chain (even on failure) so later calls can proceed.
 */
export class AccountSequenceCoordinator {
  private readonly tails = new Map<string, Promise<unknown>>();

  run<T>(accountId: string, task: () => Promise<T>): Promise<T> {
    const previous = this.tails.get(accountId) ?? Promise.resolve();
    const next = previous.then(task, task);
    // Keep the chain alive regardless of task outcome, but don't leak rejections.
    const tail = next.then(
      () => undefined,
      () => undefined
    );
    this.tails.set(accountId, tail);
    tail.then(() => {
      if (this.tails.get(accountId) === tail) {
        this.tails.delete(accountId);
      }
    });
    return next;
  }
}

export interface SubmissionErrorDetails {
  status: string;
  hash?: string;
  resultXdr?: string;
  errorResult?: xdr.TransactionResult;
  response: SorobanRpc.Api.SendTransactionResponse;
}

export class SubmissionError extends Error {
  readonly status: string;
  readonly hash?: string;
  readonly resultXdr?: string;
  readonly errorResult?: xdr.TransactionResult;
  readonly response: SorobanRpc.Api.SendTransactionResponse;

  constructor(message: string, details: SubmissionErrorDetails) {
    super(message);
    this.name = 'SubmissionError';
    this.status = details.status;
    this.hash = details.hash;
    this.resultXdr = details.resultXdr;
    this.errorResult = details.errorResult;
    this.response = details.response;
  }
}

export class ContractInvoker {
  private readonly contract: Contract;
  private readonly defaults: ContractInvokeOptions;
  private readonly sequences = new AccountSequenceCoordinator();

  constructor(contractId: string, defaults: ContractInvokeOptions = {}) {
    this.contract = new Contract(contractId);
    this.defaults = defaults;
  }

  /**
   * Resolve the effective invocation mode. An explicit `mode` always wins.
   * Otherwise a signer implies `submit`; without a signer we default to
   * `simulate` for backwards compatibility, but `submit` callers that omit a
   * signer are rejected by `call` rather than silently simulated.
   */
  private resolveMode(config: ContractInvokeOptions): InvocationMode {
    if (config.mode) {
      return config.mode;
    }
    return config.signer ? 'submit' : 'simulate';
  }

  /**
   * Simulate a contract call without submitting a transaction. Always returns
   * the simulated return value and never requires a signer.
   */
  async simulate(
    functionName: string,
    args: ScVal[],
    options: ContractInvokeOptions = {}
  ): Promise<unknown> {
    return this.call(functionName, args, { ...options, mode: 'simulate' });
  }

  /**
   * Submit a contract mutation. Requires a signer; throws `MissingSignerError`
   * when none is provided so accidental unsigned mutations are detectable.
   */
  async submit(
    functionName: string,
    args: ScVal[],
    options: ContractInvokeOptions = {}
  ): Promise<unknown> {
    return this.call(functionName, args, { ...options, mode: 'submit' });
  }

  async call(
    functionName: string,
    args: ScVal[],
    options: ContractInvokeOptions = {}
  ): Promise<unknown> {
    const config = { ...this.defaults, ...options };
    const signal = config.signal;
    throwIfAborted(signal);
    const accountId = this.require(config.sourceAccount, 'sourceAccount');
    return this.sequences.run(accountId, () =>
      this.invoke(functionName, args, config, accountId)
    );
  }

  private async invoke(
    functionName: string,
    args: ScVal[],
    config: ContractInvokeOptions,
    accountId: string
  ): Promise<unknown> {
    const signal = config.signal;
    throwIfAborted(signal);
    const server = this.server(config);
    const networkPassphrase = this.require(config.networkPassphrase, 'networkPassphrase');
    const buildTransaction = async () => {
      throwIfAborted(signal);
      const account = await server.getAccount(accountId);
      throwIfAborted(signal);
      return new TransactionBuilder(account, {
        fee: config.fee ?? '100000',
        networkPassphrase,
      })
        .setTimeout(config.timeoutSeconds ?? 30)
        .addOperation(this.contract.call(functionName, ...args))
        .build();
    };
    let transaction = await buildTransaction();
    throwIfAborted(signal);
    let simulation = await server.simulateTransaction(transaction);
    throwIfAborted(signal);
    if (SorobanRpc.Api.isSimulationError(simulation)) {
      throw new Error(formatSorobanError('Soroban simulation failed', simulation.error));
    }
    if (mode === 'simulate') {
      return simulation.result?.retval ? native(simulation.result.retval) : undefined;
    }

    const expiringKeys = await this.expiringFootprintKeys(
      server,
      simulation.transactionData,
      config
    );
    throwIfAborted(signal);
    if (expiringKeys.length > 0) {
      await this.extendFootprintTtl(
        server,
        simulation.transactionData,
        config
      );
      throwIfAborted(signal);
      transaction = await buildTransaction();
      throwIfAborted(signal);
      simulation = await server.simulateTransaction(transaction);
      throwIfAborted(signal);
      if (SorobanRpc.Api.isSimulationError(simulation)) {
        throw new Error(formatSorobanError('Soroban simulation failed', simulation.error));
      }
    }

    const prepared = SorobanRpc.assembleTransaction(transaction, simulation).build();
    prepared.sign(config.signer);
    throwIfAborted(signal);
    const submitted = await server.sendTransaction(prepared);
    if (submitted.status === 'ERROR') {
      const errorResult = submitted.errorResult;
      const resultXdr = errorResult?.toXDR('base64');
      const decoded = errorResult?.toString() ?? 'unknown error';
      throw new SubmissionError(
        formatSorobanError('Soroban submission failed', decoded),
        {
          status: submitted.status,
          hash: submitted.hash,
          resultXdr,
          errorResult,
          response: submitted,
        }
      );
    }
    if (submitted.status ==

/* … truncated 1086 chars — edit only what you need near the top … */
