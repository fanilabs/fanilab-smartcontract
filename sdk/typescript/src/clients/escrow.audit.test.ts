import { ContractInvoker } from './invoker';
import { EscrowClient } from './escrow.client';

describe('EscrowClient liquidity management', () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('queries total locked and untracked token balances as bigint values', async () => {
    const call = jest.spyOn(ContractInvoker.prototype, 'call')
      .mockResolvedValueOnce(900n)
      .mockResolvedValueOnce(25n);
    const client = new EscrowClient('CESCROW');

    await expect(client.getTotalLocked('CTOKEN')).resolves.toBe(900n);
    await expect(client.getUntrackedBalance('CTOKEN')).resolves.toBe(25n);
    expect(call).toHaveBeenNthCalledWith(1, 'get_total_locked', [expect.anything()], undefined);
    expect(call).toHaveBeenNthCalledWith(2, 'get_untracked_balance', [expect.anything()], undefined);
  });

  it('sweeps untracked funds using admin, token, and recipient and returns the swept amount', async () => {
    const call = jest.spyOn(ContractInvoker.prototype, 'call').mockResolvedValue(25n);
    const client = new EscrowClient('CESCROW');

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
