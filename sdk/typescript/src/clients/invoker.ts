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

const DEFAULT_TTL_THRESHOLD_LEDGERS = 100_000;
const DEFAULT_TTL_EXTEND_TO_LEDGERS = 518_400;

const DEFAULT_SUBMISSION_MAX_ATTEMPTS = 5;
const DEFAULT_SUBMISSION_RETRY_DELAY_MS = 1_000;

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
 * Validate polling-related settings up front so that invalid configuration is
 * rejected before any signed transaction is submitted to the network.
 */
function validatePollingSettings(config: ContractInvokeOptions): void {
  const { pollTimeoutMs, pollIntervalMs } = config;
  if (pollTimeoutMs !== undefined) {
    if (!Number.isSafeInteger(pollTimeoutMs) || pollTimeoutMs <= 0) {
      throw new RangeError('pollTimeoutMs must be a positive integer');
    }
  }
  if (pollIntervalMs !== undefined) {
    if (!Number.isSafeInteger(pollIntervalMs) || pollIntervalMs <= 0) {
      throw new RangeError('pollIntervalMs must be a positive integer');
    }
  }
  if (
    pollTimeoutMs !== undefined &&
    pollIntervalMs !== undefined &&
    pollIntervalMs > pollTimeoutMs
  ) {
    throw new RangeError('pollIntervalMs must not exceed pollTimeoutMs');
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
    // Reject invalid polling settings before any transaction is built or sent.
    validatePollingSettings(config);
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
    const submitted = await this.submitWithRetry(server, prepared, config);
    // Any non-ERROR status (PENDING, DUPLICATE, TRY_AGAIN_LATER, etc.) carries a
    // transaction hash. Poll it through finality so duplicate submissions resolve
    // to the same successful or failed result instead of returning undefined.
    return this.waitForResult(server, submitted.hash, config);
  }

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
        throw new Error(
          formatSorobanError(
            'Soroban submission failed',
            submitted.errorResult?.toString() ?? 'unknown error'
          )
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

  private async waitForResult(
    server: SorobanRpc.Server,
    hash: string,
    config: ContractInvokeOptions
  ): Promise<unknown> {
    const timeoutMs = config.pollTimeoutMs ?? 30_000;
    const intervalMs = config.pollIntervalMs ?? 1_000;
    const deadline = Date.now() + timeoutMs;

    while (Date.now() < deadline) {
      const result = await awaitPollResult(
        server.getTransaction(hash),
        Math.max(0, deadline - Date.now()),
        `Timed out waiting for transaction ${hash}`,
        config.signal
      );
      if (result.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
        return result.returnValue ? native(result.returnValue) : undefined;
      }
      if (result.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
        throw new Error(formatSorobanError('Soroban transaction failed', hash));
      }
      await delay(intervalMs, config.signal);
    }

    throw new Error(`Timed out waiting for transaction ${hash}`);
  }

  private server(config: ContractInvokeOptions): SorobanRpc.Server {
    return new SorobanRpc.Server(this.require(config.rpcUrl, 'rpcUrl'), {
      allowHttp: config.allowHttp ?? false,
    });
  }

  private require<T>(value: T | undefined, name: string): T {
    if (value === undefined) {
      throw new Error(`Missing required option: ${name}`);
    }
    return value;
  }

  private async expiringFootprintKeys(
    server: SorobanRpc.Server,
    transactionData: SorobanDataBuilder,
    config: ContractInvokeOptions
  ): Promise<xdr.LedgerKey[]> {
    const threshold = config.ttlThresholdLedgers ?? DEFAULT_TTL_THRESHOLD_LEDGERS;
    const latest = await server.getLatestLedger();
    const entries = transactionData.build().resources().footprint().readOnly().map((key) => ({
      key,
      liveUntilLedgerSeq: undefined,
    }));
    return findExpiringPersistentKeys(entries, latest.sequence, threshold);
  }

  private async extendFootprintTtl(
    server: SorobanRpc.Server,
    accountId: string,
    networkPassphrase: string,
    transactionData: SorobanDataBuilder,
    keys: xdr.LedgerKey[],
    config: ContractInvokeOptions
  ): Promise<void> {
    const extendTo = config.ttlExtendToLedgers ?? DEFAULT_TTL_EXTEND_TO_LEDGERS;
    const account = await server.getAccount(accountId);
    const builder = new TransactionBuilder(account, {
      fee: config.fee ?? '100000',
      networkPassphrase,
    }).setTimeout(config.timeoutSeconds ?? 30);
    for (const key of keys) {
      builder.addOperation(Operation.extendFootprintTtl({ extendTo }));
    }
    const transaction = builder.build();
    const simulation = await server.simulateTransaction(transaction);
    if (SorobanRpc.Api.isSimulationError(simulation)) {
      throw new Error(formatSorobanError('Soroban simulation failed', simulation.error));
    }
    const prepared = SorobanRpc.assembleTransaction(transaction, simulation).build();
    if (config.signer) {
      prepared.sign(config.signer);
    }
    await this.submitWithRetry(server, prepared, config);
  }
}
