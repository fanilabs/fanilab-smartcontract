import { scValToNative, xdr } from '@stellar/stellar-sdk';

import { DeliveryClient } from '../clients/delivery.client';
import { ContractInvoker } from '../clients/invoker';
import { DeliveryIndexKind } from '../types/delivery.types';

const CONTRACT_ID = 'CADQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQP5KR';
const OWNER = 'GAEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSCIJBEEQSH7S';

afterEach(() => {
  jest.restoreAllMocks();
});

describe('DeliveryClient getDeliveriesPage binding', () => {
  it('sends the four contract arguments and decodes a flat list of IDs', async () => {
    const call = jest
      .spyOn(ContractInvoker.prototype, 'call')
      .mockResolvedValue([10n, 11n]);
    const client = new DeliveryClient(CONTRACT_ID);

    await expect(
      client.getDeliveriesPage(OWNER, DeliveryIndexKind.Recipient, 5, 20)
    ).resolves.toEqual([10n, 11n]);

    expect(call).toHaveBeenCalledTimes(1);
    const [functionName, args] = call.mock.calls[0];
    expect(functionName).toBe('get_deliveries_page');
    expect(args).toHaveLength(4);
    expect(scValToNative(args[0] as xdr.ScVal)).toBe(OWNER);
    expect(scValToNative(args[1] as xdr.ScVal)).toBe(DeliveryIndexKind.Recipient);
    expect(scValToNative(args[2] as xdr.ScVal)).toBe(5);
    expect(scValToNative(args[3] as xdr.ScVal)).toBe(20);
  });
});
