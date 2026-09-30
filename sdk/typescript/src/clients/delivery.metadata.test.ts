jest.mock('./invoker', () => ({
  ContractInvoker: jest.fn().mockImplementation(() => ({ call: jest.fn() })),
  address: (value: string) => value,
  bool: (value: boolean) => value,
  map: (fields: Array<[string, unknown]>) => Object.fromEntries(fields),
  string: (value: string) => value,
  symbol: (value: string) => value,
  u32: (value: number) => value,
  u64: (value: bigint | number) => BigInt(value),
}));

import { DeliveryClient } from './delivery.client';
import { CargoCategory } from '../types/delivery.types';

function getCall(client: DeliveryClient): jest.Mock {
  const invoker = Reflect.get(client, 'invoker') as object;
  return Reflect.get(invoker, 'call') as jest.Mock;
}

describe('DeliveryClient metadata', () => {
  it('decodes the complete contract cargo description and metadata timestamps', async () => {
    const client = new DeliveryClient('CDELIVERY');
    getCall(client).mockResolvedValue({
      delivery_id: 42n,
      sender: 'GSENDER',
      recipient: 'GRECIPIENT',
      driver: null,
      status: 'Pending',
      metadata: {
        delivery_id: 42n,
        origin: 'Pickup',
        destination: 'Dropoff',
        cargo_description: {
          weight_grams: 2500,
          category: CargoCategory.Electronics,
          fragile: true,
        },
        created_at: 1700000000n,
        estimated_delivery: 1700003600n,
      },
      created_at: 1700000000n,
      delivered_at: null,
      transit_started_at: null,
    });

    const delivery = await client.getDelivery(42n);

    expect(delivery.metadata).toEqual({
      deliveryId: 42n,
      pickupLocation: 'Pickup',
      dropoffLocation: 'Dropoff',
      cargoDescription: {
        weightGrams: 2500,
        category: CargoCategory.Electronics,
        fragile: true,
      },
      createdAt: 1700000000,
      estimatedDelivery: 1700003600,
    });
  });

  it('encodes submitted cargo fields and timestamps in contract metadata', async () => {
    const client = new DeliveryClient('CDELIVERY');
    const call = getCall(client);
    call.mockResolvedValue(42n);

    await client.createDelivery({
      sender: 'GSENDER',
      recipient: 'GRECIPIENT',
      deliveryId: 42n,
      metadata: {
        pickupLocation: 'Pickup',
        dropoffLocation: 'Dropoff',
        cargoDescription: {
          weightGrams: 2500,
          category: CargoCategory.Electronics,
          fragile: true,
        },
        createdAt: 1700000000,
        estimatedDelivery: 1700003600,
      },
    });

    expect(call).toHaveBeenCalledWith('create_delivery', [
      'GSENDER',
      'GRECIPIENT',
      {
        delivery_id: 42n,
        origin: 'Pickup',
        destination: 'Dropoff',
        cargo_description: {
          weight_grams: 2500,
          category: CargoCategory.Electronics,
          fragile: true,
        },
        created_at: 1700000000n,
        estimated_delivery: 1700003600n,
      },
    ], undefined);
  });
});
