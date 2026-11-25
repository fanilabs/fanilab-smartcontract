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
const LEDGER_ENTRY_BATCH_LIMIT = 200;

const DEFAULT_SUBMISSION_MAX_ATTEMPTS = 5;
const DEFAULT_SUBMISSION_RETRY_DELAY_MS = 1_000;

function canExtendTtl(key: xdr.LedgerKey): boolean {
  return key.type === 'contractCode' ||
    (key.type === 'contractData' && key.contractData.durability.name === 'persistent');
}

function ledgerKeyId(key: xdr.LedgerKey): string {
  // Real ledger keys serialize themselves; plain-object test doubles do not,
  // so fall back to a structural key for deduplication.
  const serializable = key as { toXDR?: (encoding: string) => string };
  return typeof serializable.toXDR === 'function'
    ? serializable.toXDR('base64')
    : JSON.stringify(key);
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

/**
 * Returns true when the given hostname refers to the local machine
 * (loopback). Only these hosts are allowed to use plain HTTP automatically.
 */
export function isLoopbackHost(hostname: string): boolean {
  const host = hostname.toLowerCase().replace(/^\[|\]$/g, '');
  if (host === 'localhost' || host === '::1' || host === '0:0:0:0:0:0:0:1') {
    return true;
  }
  // IPv4 loopback range 127.0.0.0/8
  return /^127(?:\.\d{1,3}){3}$/.test(host);
}

/**
 * Decide whether plain HTTP is permitted for the given RPC URL.
 *
 * HTTP is opt-in: it is only allowed when the caller explicitly sets
 * `allowInsecureHttp`, or when the URL points at a loopback host. Remote
 * endpoints must use HTTPS unless the insecure option is set.
 */
export function assertHttpTransportAllowed(url: string, allowInsecureHttp?: boolean): void {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`Invalid RPC URL: ${url}`);
  }
  if (parsed.protocol !== 'http:') {
    return;
  }
  if (allowInsecureHttp === true) {
    return;
  }
  if (isLoopbackHost(parsed.hostname)) {
    return;
  }
  throw new Error(
    `Refusing insecure HTTP RPC transport for remote endpoint ${parsed.host}. ` +
      'Use HTTPS, or set allowInsecureHttp: true to opt in explicitly.'
  );
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
    this.defaults = { ...defaults };
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
    const mode = this.resolveMode(config);
    if (mode === 'submit' && config.signer === undefined) {
      throw new MissingSignerError(functionName);
    }
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
        accountId,
        networkPassphrase,
        simulation.transactionData,
        expiringKeys,
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
    prepared.sign(this.require(config.signer, 'signer'));
    throwIfAborted(signal);
    const submitted = await this.submitWithRetry(server, prepared, config);
    const result = await this.waitForResult(server, submitted.hash, config);
    if (config.returnReceipt) {
      return {
        hash: submitted.hash,
        ledger: simulation.latestLedger ?? 0,
        result,
      } satisfies InvocationReceipt;
    }
    return result;
  }

  /**
   * Submit a prepared transaction, retrying transient `TRY_AGAIN_LATER`
   * responses with linear backoff. Terminal `ERROR` responses surface the
   * decoded result XDR through `SubmissionError` so callers keep the diagnostic
   * context instead of parsing a formatted string.
   */
  private async submitWithRetry(
    server: SorobanRpc.Server,
    prepared: ReturnType<typeof SorobanRpc.assembleTransaction>['build'] extends () => infer T
      ? T
      : never,
    config: ContractInvokeOptions
  ): Promise<SorobanRpc.Api.SendTransactionResponse> {
    const maxAttempts = config.submissionMaxAttempts ?? DEFAULT_SUBMISSION_MAX_ATTEMPTS;
    const baseDelay = config.submissionRetryDelayMs ?? DEFAULT_SUBMISSION_RETRY_DELAY_MS;
    if (!Number.isSafeInteger(maxAttempts) || maxAttempts <= 0) {
      throw new RangeError('submissionMaxAttempts must be a positive integer');
    }
    if (!Number.isSafeInteger(baseDelay) || baseDelay < 0) {
      throw new RangeError('submissionRetryDelayMs must be a non-negative integer');
    }

    let lastStatus: string | undefined;
    for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
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
      if (submitted.status !== 'TRY_AGAIN_LATER') {
        return submitted;
      }

      lastStatus = submitted.status;
      if (attempt < maxAttempts) {
        await delay(baseDelay * attempt, config.signal);
      }
    }

    throw new Error(
      formatSorobanError(
        'Soroban submission failed after retries',
        `transaction ${prepared.hash.toString()} was not accepted after ${maxAttempts} attempts ` +
          `(last status: ${lastStatus ?? 'unknown'}); the signed transaction was preserved and ` +
          'can be resubmitted'
      )
    );
  }

  private async expiringFootprintKeys(
    server: SorobanRpc.Server,
    transactionData: SorobanDataBuilder,
    config: ContractInvokeOptions
  ): Promise<xdr.LedgerKey[]> {
    const threshold = config.ttlThresholdLedgers ?? DEFAULT_TTL_THRESHOLD_LEDGERS;
    const extendTo = config.ttlExtendToLedgers ?? DEFAULT_TTL_EXTEND_TO_LEDGERS;
    if (!Number.isSafeInteger(threshold) || threshold <= 0) {
      throw new RangeError('ttlThresholdLedgers must be a positive integer');
    }
    if (!Number.isSafeInteger(extendTo) || extendTo <= threshold) {
      throw new RangeError('ttlExtendToLedgers must be greater than ttlThresholdLedgers');
    }

    const candidates = dedupeLedgerKeys(
      [
        ...transactionData.getReadOnly(),
        ...transactionData.getReadWrite(),
      ].filter(canExtendTtl)
    );
    if (candidates.length === 0) {
      return [];
    }

    // Chunk large footprints so a single RPC request stays within the node's
    // ledger-entry limit instead of being rejected wholesale.
    const expiring: xdr.LedgerKey[] = [];
    for (let start = 0; start < candidates.length; start += LEDGER_ENTRY_BATCH_LIMIT) {
      const batch = candidates.slice(start, start + LEDGER_ENTRY_BATCH_LIMIT);
      const ledgerEntries = await server.getLedgerEntries(...batch);
      expiring.push(
        ...findExpiringPersistentKeys(
          ledgerEntries.entries,
          ledgerEntries.latestLedger,
          threshold
        )
      );
    }
    return dedupeLedgerKeys(expiring);
  }

  private async extendFootprintTtl(
    server: SorobanRpc.Server,
    accountId: string,
    networkPassphrase: string,
    transactionData: SorobanDataBuilder,
    keys: xdr.LedgerKey[],
    config: ContractInvokeOptions
  ): Promise<void> {
    const threshold = config.ttlThresholdLedgers ?? DEFAULT_TTL_THRESHOLD_LEDGERS;
    const requestedExtendTo = config.ttlExtendToLedgers ?? DEFAULT_TTL_EXTEND_TO_LEDGERS;
    const networkMaxTtlLedgers = await this.networkMaxTtlLedgers(server, config);
    const extendTo = resolveTtlExtendTo(requestedExtendTo, threshold, networkMaxTtlLedgers);

    const account = await server.getAccount(accountId);
    const builder = new TransactionBuilder(account, {
      fee: config.fee ?? '100000',
      networkPassphrase,
    })
      .setTimeout(config.timeoutSeconds ?? 30)
      .setSorobanData(transactionData.build());

    for (let index = 0; index < keys.length; index += 1) {
      builder.addOperation(Operation.extendFootprintTtl({ extendTo }));
    }

    const transaction = builder.build();
    const simulation = await server.simulateTransaction(transaction);
    if (SorobanRpc.Api.isSimulationError(simulation)) {
      throw new Error(
        formatSorobanError('Soroban TTL extension simulation failed', simulation.error)
      );
    }
    const prepared = SorobanRpc.assembleTransaction(transaction, simulation).build();
    prepared.sign(this.require(config.signer, 'signer'));
    const submitted = await this.submitWithRetry(server, prepared, config);
    await this.waitForResult(server, submitted.hash, config);
  }

  /**
   * Resolve the network maximum entry TTL. A caller-provided value wins; RPCs
   * that do not expose `maxEntryTtl` fall back to the configured extend target
   * so TTL maintenance keeps working against older nodes and test doubles.
   */
  private async networkMaxTtlLedgers(
    server: SorobanRpc.Server,
    config: ContractInvokeOptions
  ): Promise<number> {
    if (config.networkMaxTtlLedgers !== undefined) {
      if (!Number.isSafeInteger(config.networkMaxTtlLedgers) || config.networkMaxTtlLedgers <= 0) {
        throw new RangeError('networkMaxTtlLedgers must be a positive integer');
      }
      return config.networkMaxTtlLedgers;
    }

    const getNetwork = (server as { getNetwork?: () => Promise<{ maxEntryTtl?: number }> }).getNetwork;
    if (typeof getNetwork === 'function') {
      const network = await getNetwork.call(server);
      if (typeof network.maxEntryTtl === 'number' && network.maxEntryTtl > 0) {
        return network.maxEntryTtl;
      }
    }
    return config.ttlExtendToLedgers ?? DEFAULT_TTL_EXTEND_TO_LEDGERS;
  }

  private async waitForResult(
    server: SorobanRpc.Server,
    hash: string,
    config: ContractInvokeOptions
  ): Promise<unknown> {
    const initialDelay = config.pollingIntervalMs ?? 1000;
    const maxDelay = config.pollingMaxIntervalMs ?? 10_000;
    const timeout = config.pollingTimeoutMs ?? 300_000;
    if (!Number.isFinite(initialDelay) || initialDelay <= 0) {
      throw new RangeError('pollingIntervalMs must be a positive number');
    }
    if (!Number.isFinite(maxDelay) || maxDelay < initialDelay) {
      throw new RangeError('pollingMaxIntervalMs must be greater than or equal to pollingIntervalMs');
    }
    if (!Number.isFinite(timeout) || timeout <= 0) {
      throw new RangeError('pollingTimeoutMs must be a positive number');
    }

    const deadline = Date.now() + timeout;
    let interval = initialDelay;
    for (;;) {
      if (config.signal?.aborted) {
        throw abortError();
      }
      if (Date.now() >= deadline) {
        throw new Error(`Timed out waiting for Soroban transaction: ${hash}`);
      }
      const remaining = deadline - Date.now();
      const result = await awaitPollResult(
        server.getTransaction(hash),
        remaining,
        `Timed out waiting for Soroban transaction: ${hash}`,
        config.signal
      );
      if (result.status === 'SUCCESS') {
        return result.returnValue ? native(result.returnValue) : undefined;
      }
      if (result.status === 'FAILED') {
        const errorCodes = result.diagnosticEventsXdr?.flatMap(({ event }) => {
          const value = event.body.value.data;
          return value.type === 'scvError' && value.error.type === 'sceContract'
            ? [value.error.contractCode]
            : [];
        });
        const failure = errorCodes?.length
          ? `Error(Contract, #${errorCodes.join(', #')})`
          : hash;
        throw new Error(
          formatSorobanError('Soroban transaction failed', failure, errorCodes ?? [])
        );
      }
      const remainingAfterPoll = deadline - Date.now();
      if (remainingAfterPoll <= 0) {
        throw new Error(`Timed out waiting for Soroban transaction: ${hash}`);
      }
      await delay(Math.min(interval, remainingAfterPoll), config.signal);
      interval = Math.min(interval * 2, maxDelay);
    }
  }

  private server(config: ContractInvokeOptions): SorobanRpc.Server {
    const serverUrl = this.require(config.serverUrl, 'serverUrl');
    assertHttpTransportAllowed(serverUrl, config.allowInsecureHttp);
    return new SorobanRpc.Server(serverUrl, {
      allowHttp: config.serverUrl?.startsWith('http://') ?? false,
      ...(config.timeout === undefined ? {} : { timeout: config.timeout }),
    });
  }

  private require<T>(value: T | undefined, name: string): T {
    if (value === undefined) {
      throw new Error(`Contract invocation requires ${name}`);
    }
    return value;
  }
}

export type Signer = Keypair;
