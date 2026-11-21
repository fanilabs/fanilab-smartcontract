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
      return this.waitForResult(server, submitted.hash, config);
    }
    return undefined;
  }

  private require(value: string | undefined, field: string): string {
    if (value === undefined) {
      throw new Error(`Missing required configuration: ${field} is not set`);
    }
    if (value.trim() === '') {
      throw new Error(`Invalid configuration: ${field} must be a non-empty string`);
    }
    return value;
  }

  private server(config: ContractInvokeOptions): SorobanRpc.Server {
    const serverUrl = this.require(config.serverUrl, 'serverUrl');
    return new SorobanRpc.Server(serverUrl);
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

    const candidates = [
      ...transactionData.getReadOnly(),
      ...transactionData.getReadWrite(),
    ].filter(canExtendTtl);
    if (candidates.length === 0) {
      return [];
    }

    const ledgerEntries = await server.getLedgerEntries(...candidates);
    return findExpiringPersistentKeys(ledgerEntries.entries, ledgerEntries.latestLedger, threshold);
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
    const ttlTransaction = new TransactionBuilder(account, {
      fee: config.fee ?? '100000',
      networkPassphrase,
    })
      .setTimeout(config.timeoutSeconds ?? 30)
      .addOperation(Operation.extendFootprintTtl({ extendTo }))
      .setSorobanData(
        new SorobanDataBuilder(transactionData.build())
          .setFootprint(keys, [])
          .build()
      )
      .build();
    ttlTransaction.sign(config.signer as Keypair);
    const submitted = await server.sendTransaction(ttlTransaction);
    if (submitted.status === 'ERROR') {
      throw new Error(
        formatSorobanError(
          'Soroban TTL extension failed',
          submitted.errorResult?.toString() ?? 'unknown error'
        )
      );
    }
    if (submitted.status === 'PENDING') {
      await this.waitForResult(server, submitted.hash, config);
    }
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
      if (config.signal?.aborted) {
        throw abortError();
      }
      const result = await awaitPollResult(
        server.getTransaction(hash),
        Math.max(deadline - Date.now(), 0),
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
}
