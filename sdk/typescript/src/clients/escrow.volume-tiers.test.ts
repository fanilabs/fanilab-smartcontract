import { ContractInvoker } from './invoker';
import { EscrowClient } from './escrow.client';

describe('EscrowClient volume tiers', () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('sets volume tiers using the contract field names', async () => {
    const call = jest.spyOn(ContractInvoker.prototype, 'call').mockResolvedValue(undefined);
    const client = new EscrowClient('CESCROW');

    await client.setVolumeTiers({
      admin: 'GADMIN',
      tiers: [
        { volumeThreshold: 1000, discountBps: 250 },
        { volumeThreshold: 5000, discountBps: 500 },
      ],
    });

    expect(call).toHaveBeenCalledWith(
      'set_volume_tiers',
      [expect.anything(), expect.anything()],
      undefined
    );
  });

  it('decodes configured tiers and sender volume', async () => {
    const call = jest.spyOn(ContractInvoker.prototype, 'call')
      .mockResolvedValueOnce([
        { volume_threshold: 1000, discount_bps: 250 },
        { volume_threshold: 5000, discount_bps: 500 },
      ])
      .mockResolvedValueOnce(1250);
    const client = new EscrowClient('CESCROW');

    await expect(client.getVolumeTiers()).resolves.toEqual([
      { volumeThreshold: 1000, discountBps: 250 },
      { volumeThreshold: 5000, discountBps: 500 },
    ]);
    await expect(client.getSenderVolume('GSENDER')).resolves.toBe(1250);
    expect(call).toHaveBeenNthCalledWith(1, 'get_volume_tiers', [], undefined);
    expect(call).toHaveBeenNthCalledWith(2, 'get_sender_volume', [expect.anything()], undefined);
  });
});
