jest.mock('./invoker', () => ({
  ContractInvoker: jest.fn().mockImplementation(() => ({ call: jest.fn() })),
  address: (value: string) => value,
  u64: (value: bigint | number) => BigInt(value),
}));

import { DeliveryClient } from './delivery.client';

function getCall(client: DeliveryClient): jest.Mock {
  const invoker = Reflect.get(client, 'invoker') as object;
  return Reflect.get(invoker, 'call') as jest.Mock;
}

describe('DeliveryClient contract aggregations', () => {
  it('reads the driver profile through delivery_contract', async () => {
    const client = new DeliveryClient('CDELIVERY', { identityContractId: 'CIDENTITY' });
    const call = getCall(client);
    call.mockResolvedValue({
      address: 'GDRIVER',
      deliveries_completed: 7,
      reputation_score: 42,
      registered_at: 100,
      kyc_verified: true,
      status: 'Active',
    });

    await expect(client.getDriverProfile('GDRIVER')).resolves.toEqual({
      address: 'GDRIVER',
      deliveriesCompleted: 7,
      reputationScore: 42,
      registeredAt: 100,
      kycVerified: true,
      status: 'Active',
    });
    expect(call).toHaveBeenCalledWith('get_driver_profile', [expect.anything()], undefined);
    expect(call.mock.contexts[0]).toBe(Reflect.get(client, 'invoker'));
  });

  it('decodes the aggregated delivery and optional escrow tuple', async () => {
    const client = new DeliveryClient('CDELIVERY');
    getCall(client).mockResolvedValue([
      {
        delivery_id: 9n,
        sender: 'GSENDER',
        recipient: 'GRECIPIENT',
        driver: null,
        status: 'Pending',
        metadata: {
          delivery_id: 9n,
          origin: 'Pickup',
          destination: 'Dropoff',
          cargo_description: { weight_grams: 1, category: 'General', fragile: false },
          created_at: 100n,
          estimated_delivery: 200n,
        },
        created_at: 100n,
        delivered_at: null,
        transit_started_at: null,
      },
      null,
      false,
    ]);

    await expect(client.getCombinedState(9n)).resolves.toMatchObject({
      delivery: { deliveryId: 9n },
      escrow: undefined,
      isSynchronized: false,
    });
  });
});
