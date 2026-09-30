import { DisputeStatus } from '../types/common.types';

/**
 * Issue #474 — `decodeDispute` must serialize `BytesN<32>` evidence hashes
 * into hex. `scValToNative` returns a byte buffer, so the previous
 * `String(item.hash)` produced "[object Uint8Array]" and destroyed the
 * proof-of-delivery hash.
 *
 * `clients/invoker` is stubbed so the round-trip is exercised without
 * touching @stellar/stellar-sdk, which this jest setup cannot load
 * (pre-existing ESM/transform failure affecting the delivery suites too).
 * The stubs mirror the real argument encoders one-for-one.
 */
jest.mock('../clients/invoker', () => ({
  address: (value: string) => ({ kind: 'address', value }),
  u64: (value: bigint | number) => ({ kind: 'u64', value: BigInt(value) }),
  u32: (value: number) => ({ kind: 'u32', value }),
  bytes: (value: Uint8Array) => ({ kind: 'bytes', value: Buffer.from(value) }),
  ContractInvoker: class {},
}));

import { DisputeResolutionClient } from '../clients/dispute_resolution.client';

describe('DisputeResolutionClient evidence hash decoding', () => {
  const EVIDENCE_HASH_HEX =
    'a3f1c05d9b7e2468ac0d1e3f5a7b9c1d3e5f7a9b1c3d5e7f90a1b2c3d4e5f607';
  const submitter = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';

  let client: DisputeResolutionClient;
  let callMock: jest.Mock;

  beforeEach(() => {
    client = new DisputeResolutionClient('CDISPUTECONTRACTIDXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX');
    callMock = jest.fn();
    (client as unknown as { invoker: { call: jest.Mock } }).invoker = {
      call: callMock,
    };
  });

  test('addEvidenceHash sends a raw 32-byte payload for a hex string', async () => {
    callMock.mockResolvedValue(undefined);

    await client.addEvidenceHash({
      caller: submitter,
      deliveryId: 7n,
      evidenceHash: EVIDENCE_HASH_HEX,
    });

    const [functionName, args] = callMock.mock.calls[0];
    expect(functionName).toBe('add_evidence_hash');
    // 32-byte payload, not the hex string reinterpreted as text.
    expect(args[2].value).toHaveLength(32);
    expect(Buffer.from(args[2].value).toString('hex')).toBe(EVIDENCE_HASH_HEX);
  });

  test('getDispute round-trips an added evidence hash back as hex', async () => {
    callMock.mockResolvedValueOnce(undefined);
    callMock.mockResolvedValueOnce({
      delivery_id: 7n,
      status: DisputeStatus.Open,
      raised_at: 1700000000,
      raised_by: submitter,
      // scValToNative surfaces BytesN<32> as a Buffer/Uint8Array.
      evidence_hashes: [{ submitter, hash: Buffer.from(EVIDENCE_HASH_HEX, 'hex') }],
      resolved_at: null,
      resolved_by: null,
    });

    await client.addEvidenceHash({
      caller: submitter,
      deliveryId: 7n,
      evidenceHash: EVIDENCE_HASH_HEX,
    });
    const dispute = await client.getDispute(7n);

    expect(dispute.evidenceHashes).toHaveLength(1);
    expect(dispute.evidenceHashes[0].hash).toBe(EVIDENCE_HASH_HEX);
    expect(dispute.evidenceHashes[0].hash).toMatch(/^[0-9a-f]{64}$/);
    expect(dispute.evidenceHashes[0].submitter).toBe(submitter);
    expect(dispute.evidenceHashes[0].hash).not.toContain('object');
  });

  test('decodes a dispute with no evidence hashes to an empty list', async () => {
    callMock.mockResolvedValue({
      delivery_id: 9n,
      status: DisputeStatus.Open,
      raised_at: 1700000100,
      raised_by: submitter,
      evidence_hashes: [],
    });

    const dispute = await client.getDispute(9n);

    expect(dispute.evidenceHashes).toEqual([]);
    expect(dispute.deliveryId).toBe(9n);
    expect(dispute.status).toBe(DisputeStatus.Open);
  });
});
