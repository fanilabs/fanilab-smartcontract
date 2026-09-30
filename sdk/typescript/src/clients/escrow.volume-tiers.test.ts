jest.mock('./invoker', () => ({
  ContractInvoker: jest.fn().mockImplementation(() => ({ call: jest.fn() })),
  address: (value: string) => value,
  map: (fields: Array<[string, unknown]>) => Object.fromEntries(fields),
  u32: (value: number) => value,
  vec: (values: unknown[]) => values,
}));
jest.mock('@stellar/stellar-sdk', () => ({
  nativeToScVal: (value: unknown) => value,
}));

import { EscrowClient } from './escrow.client';

function getCall(client: EscrowClient): jest.Mock {
  const invoker = Reflect.get(client, 'invoker') as object;
  return Reflect.get(invoker, 'call') as jest.Mock;
}

describe('EscrowClient volume tiers', () => {
  it('sets volume tiers using the contract field names', async () => {
    const client = new EscrowClient('CESCROW');
    const call = getCall(client).mockResolvedValue(undefined);

    await client.setVolumeTiers({
      admin: 'GADMIN',
      tiers: [
        { volumeThreshold: 1000, discountBps: 250 },
        { volumeThreshold: 5000, discountBps: 500 },
      ],
    });

    expect(call).toHaveBeenCalledWith(
      'set_volume_tiers',
      [
        'GADMIN',
        [
          { volume_threshold: 1000, discount_bps: 250 },
          { volume_threshold: 5000, discount_bps: 500 },
        ],
      ],
      undefined
    );
  });

  it('decodes configured tiers and sender volume', async () => {
    const client = new EscrowClient('CESCROW');
    const call = getCall(client)
      .mockResolvedValueOnce([
        { volume_threshold: 1000, discount_bps: 250 },
        { volume_threshold: 5000, discount_bps: 500 },
      ])
      .mockResolvedValueOnce(1250);

    await expect(client.getVolumeTiers()).resolves.toEqual([
      { volumeThreshold: 1000, discountBps: 250 },
      { volumeThreshold: 5000, discountBps: 500 },
    ]);
    await expect(client.getSenderVolume('GSENDER')).resolves.toBe(1250);
    expect(call).toHaveBeenNthCalledWith(1, 'get_volume_tiers', [], undefined);
    expect(call).toHaveBeenNthCalledWith(2, 'get_sender_volume', [expect.anything()], undefined);
  });
});
