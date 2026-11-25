import * as sdk from '../index';

describe('SDK package entry point', () => {
  test('exports the contract invoker and ScVal conversion helpers', () => {
    for (const name of [
      'ContractInvoker',
      'address',
      'bool',
      'bytes',
      'i128',
      'map',
      'native',
      'string',
      'symbol',
      'u32',
      'u64',
      'vec',
    ]) {
      expect(sdk).toHaveProperty(name);
    }
  });

  test('exports common client option types', () => {
    const options: sdk.ContractInvokeOptions = {
      timeoutSeconds: 20,
      ttlThresholdLedgers: 50_000,
      ttlExtendToLedgers: 200_000,
    };
    const pagination: sdk.PaginationOptions = { offset: 0, limit: 10 };

    expect(options.timeoutSeconds).toBe(20);
    expect(pagination.limit).toBe(10);
  });
});