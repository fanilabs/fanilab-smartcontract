jest.mock('./invoker', () => ({
  ContractInvoker: jest.fn().mockImplementation(() => ({ call: jest.fn() })),
  address: (value: string) => value,
}));
jest.mock('@stellar/stellar-sdk', () => ({
  nativeToScVal: (value: unknown) => value,
}));

import { EscrowClient } from './escrow.client';

function getCall(client: EscrowClient): jest.Mock {
  const invoker = Reflect.get(client, 'invoker') as object;
  return Reflect.get(invoker, 'call') as jest.Mock;
}

describe('EscrowClient liquidity management', () => {
  it('queries total locked and untracked token balances as bigint values', async () => {
    const client = new EscrowClient('CESCROW');
    const call = getCall(client).mockResolvedValueOnce(900n).mockResolvedValueOnce(25n);

    await expect(client.getTotalLocked('CTOKEN')).resolves.toBe(900n);
    await expect(client.getUntrackedBalance('CTOKEN')).resolves.toBe(25n);
    expect(call).toHaveBeenNthCalledWith(1, 'get_total_locked', [expect.anything()], undefined);
    expect(call).toHaveBeenNthCalledWith(2, 'get_untracked_balance', [expect.anything()], undefined);
  });

  it('sweeps untracked funds using admin, token, and recipient and returns the swept amount', async () => {
    const client = new EscrowClient('CESCROW');
    const call = getCall(client).mockResolvedValue(25n);

    await expect(client.sweepUntrackedBalance({
      admin: 'GADMIN',
      token: 'CTOKEN',
      recipient: 'GTREASURY',
    })).resolves.toBe(25n);
    expect(call).toHaveBeenCalledWith(
      'sweep_untracked_balance',
      [expect.anything(), expect.anything(), expect.anything()],
      undefined
    );
  });
});
