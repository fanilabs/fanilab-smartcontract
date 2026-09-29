import {
  Address,
  Contract,
  Keypair,
  nativeToScVal,
  scValToNative,
  SorobanRpc,
  TransactionBuilder,
  xdr,
} from '@stellar/stellar-sdk';
import { ContractInvokeOptions } from '../types/common.types';

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
    const account = await server.getAccount(accountId);
    const transaction = new TransactionBuilder(account, {
      fee: config.fee ?? '100000',
      networkPassphrase,
    })
      .setTimeout(config.timeoutSeconds ?? 30)
      .addOperation(this.contract.call(functionName, ...args))
      .build();
    const prepared = await server.prepareTransaction(transaction);

    if (!config.signer) {
      const simulation = await server.simulateTransaction(transaction);
      if (SorobanRpc.Api.isSimulationError(simulation)) {
        throw new Error(`Soroban simulation failed: ${simulation.error}`);
      }
      return simulation.result?.retval ? native(simulation.result.retval) : undefined;
    }

    prepared.sign(config.signer);
    const submitted = await server.sendTransaction(prepared);
    if (submitted.status === 'ERROR') {
      throw new Error(`Soroban submission failed: ${submitted.errorResult?.toString() ?? 'unknown error'}`);
    }
    if (submitted.status === 'PENDING') {
      return this.waitForResult(server, submitted.hash);
    }
    return undefined;
  }

  private async waitForResult(server: SorobanRpc.Server, hash: string): Promise<unknown> {
    for (;;) {
      const result = await server.getTransaction(hash);
      if (result.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
        return result.returnValue ? native(result.returnValue) : undefined;
      }
      if (result.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
        throw new Error(`Soroban transaction failed: ${hash}`);
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }

  private server(config: ContractInvokeOptions): SorobanRpc.Server {
    return new SorobanRpc.Server(
      this.require(config.serverUrl, 'serverUrl'),
      { allowHttp: config.serverUrl?.startsWith('http://') }
    );
  }

  private require<T>(value: T | undefined, name: string): T {
    if (value === undefined) {
      throw new Error(`Contract invocation requires ${name}`);
    }
    return value;
  }
}

export type Signer = Keypair;
