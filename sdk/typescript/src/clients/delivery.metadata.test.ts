import { ContractInvoker } from './invoker';
import { DeliveryClient } from './delivery.client';
import { CargoCategory } from '../types/delivery.types';

describe('DeliveryClient metadata', () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('decodes the complete contract cargo description and metadata timestamps', async () => {
    jest.spyOn(ContractInvoker.prototype, 'call').mockResolvedValue({
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
    const client = new DeliveryClient('CDELIVERY');

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
});
