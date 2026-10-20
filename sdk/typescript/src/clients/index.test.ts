import { DeliveryClient } from './index';

describe('DeliveryClient pagination helpers', () => {
  let client: DeliveryClient;
  let invokeSpy: jest.SpyInstance;

  beforeEach(() => {
    client = new DeliveryClient({
      contractId: 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM',
      rpcUrl: 'https://soroban-testnet.stellar.org',
      publicKey: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
      secretKey: 'SAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
    });
    invokeSpy = jest
      .spyOn(client as any, 'invoke')
      .mockResolvedValue(null);
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('getDeliveriesBySender passes kind = 0 with 4 args', async () => {
    await client.getDeliveriesBySender('GSENDER', 5, 20);
    expect(invokeSpy).toHaveBeenCalledTimes(1);
    const [method, args] = invokeSpy.mock.calls[0];
    expect(method).toBe('get_deliveries_page');
    expect(args).toHaveLength(4);
    expect(args[1].u32()).toBe(0);
    expect(args[2].u32()).toBe(5);
    expect(args[3].u32()).toBe(20);
  });

  it('getDeliveriesByRecipient passes kind = 1 with 4 args', async () => {
    await client.getDeliveriesByRecipient('GRECIPIENT', 0, 10);
    expect(invokeSpy).toHaveBeenCalledTimes(1);
    const [method, args] = invokeSpy.mock.calls[0];
    expect(method).toBe('get_deliveries_page');
    expect(args).toHaveLength(4);
    expect(args[1].u32()).toBe(1);
    expect(args[2].u32()).toBe(0);
    expect(args[3].u32()).toBe(10);
  });

  it('getDeliveriesByDriver passes kind = 2 with 4 args', async () => {
    await client.getDeliveriesByDriver('GDRIVER', 2, 15);
    expect(invokeSpy).toHaveBeenCalledTimes(1);
    const [method, args] = invokeSpy.mock.calls[0];
    expect(method).toBe('get_deliveries_page');
    expect(args).toHaveLength(4);
    expect(args[1].u32()).toBe(2);
    expect(args[2].u32()).toBe(2);
    expect(args[3].u32()).toBe(15);
  });
});
