import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DeliveryClient } from './delivery.client';
import type { DeliveryMetadata } from '../types/delivery.types';

describe('DeliveryClient', () => {
  let client: DeliveryClient;
  let invokeContract: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    invokeContract = vi.fn().mockResolvedValue({ txHash: 'tx-1' });
    client = new DeliveryClient({
      contractId: 'CDELIVERY',
      invokeContract,
    } as any);
  });

  describe('createDelivery', () => {
    it('maps a single metadata object to the create_delivery entrypoint', async () => {
      const metadata: DeliveryMetadata = {
        sender: 'GSENDER',
        recipient: 'GRECIPIENT',
        amount: 100n,
        token: 'GTOKEN',
        deadline: 1700000000,
      };

      await client.createDelivery(metadata);

      expect(invokeContract).toHaveBeenCalledWith('create_delivery', [metadata]);
    });
  });

  describe('createDeliveriesBatch', () => {
    it('maps an array of metadata to the create_deliveries_batch entrypoint', async () => {
      const metadata: DeliveryMetadata[] = [
        {
          sender: 'GSENDER1',
          recipient: 'GRECIPIENT1',
          amount: 100n,
          token: 'GTOKEN',
          deadline: 1700000000,
        },
        {
          sender: 'GSENDER2',
          recipient: 'GRECIPIENT2',
          amount: 250n,
          token: 'GTOKEN',
          deadline: 1700000100,
        },
      ];

      await client.createDeliveriesBatch(metadata);

      expect(invokeContract).toHaveBeenCalledWith('create_deliveries_batch', [metadata]);
    });

    it('passes an empty array through to the contract', async () => {
      await client.createDeliveriesBatch([]);

      expect(invokeContract).toHaveBeenCalledWith('create_deliveries_batch', [[]]);
    });

    it('preserves per-item metadata mapping order', async () => {
      const metadata: DeliveryMetadata[] = [
        {
          sender: 'GSENDER1',
          recipient: 'GRECIPIENT1',
          amount: 1n,
          token: 'GTOKEN',
          deadline: 1,
        },
        {
          sender: 'GSENDER2',
          recipient: 'GRECIPIENT2',
          amount: 2n,
          token: 'GTOKEN',
          deadline: 2,
        },
        {
          sender: 'GSENDER3',
          recipient: 'GRECIPIENT3',
          amount: 3n,
          token: 'GTOKEN',
          deadline: 3,
        },
      ];

      await client.createDeliveriesBatch(metadata);

      const [, args] = invokeContract.mock.calls[0];
      expect(args[0]).toEqual(metadata);
      expect(args[0]).toHaveLength(3);
    });
  });

  describe('getDeliveriesBySender', () => {
    it('forwards invocation options to the invoker', async () => {
      const options = { source: 'GSOURCE', fee: 100 };

      await client.getDeliveriesBySender('GSENDER', options as any);

      expect(invokeContract).toHaveBeenCalledWith(
        'get_deliveries_by_sender',
        ['GSENDER'],
        options,
      );
    });

    it('passes undefined options through when omitted', async () => {
      await client.getDeliveriesBySender('GSENDER');

      expect(invokeContract).toHaveBeenCalledWith(
        'get_deliveries_by_sender',
        ['GSENDER'],
        undefined,
      );
    });
  });

  describe('getDeliveriesByRecipient', () => {
    it('forwards invocation options to the invoker', async () => {
      const options = { source: 'GSOURCE', fee: 100 };

      await client.getDeliveriesByRecipient('GRECIPIENT', options as any);

      expect(invokeContract).toHaveBeenCalledWith(
        'get_deliveries_by_recipient',
        ['GRECIPIENT'],
        options,
      );
    });

    it('passes undefined options through when omitted', async () => {
      await client.getDeliveriesByRecipient('GRECIPIENT');

      expect(invokeContract).toHaveBeenCalledWith(
        'get_deliveries_by_recipient',
        ['GRECIPIENT'],
        undefined,
      );
    });
  });
});
