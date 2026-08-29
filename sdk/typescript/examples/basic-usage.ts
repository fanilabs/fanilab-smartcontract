/**
 * SDK preview example for the FaniLab TypeScript client layer.
 *
 * These client methods are intentionally non-functional in the current SDK
 * snapshot; they document the expected call shapes but are not yet wired to a
 * live Soroban invocation layer. Running this file should fail loudly instead of
 * silently appearing successful.
 */

import { EscrowClient, DeliveryClient } from '../src/index';

const deliveryClient = new DeliveryClient('CBDE...');
const escrowClient = new EscrowClient('CABC...');

async function main() {
  console.warn('Warning: this TypeScript SDK example is a shape preview only. The current client layer is intentionally not connected to a live contract invocation backend.');

  try {
    await deliveryClient.createDelivery({
      sender: 'GA7VQKQ...',
      recipient: 'GB3UWF4...',
      deliveryId: BigInt(1),
      metadata: {
        pickupLocation: '123 Main St, City',
        dropoffLocation: '456 Oak Ave, City',
        items: 'Package containing books',
        notes: 'Deliver between 9 AM - 5 PM',
        estimatedDistance: 25,
      },
    });
  } catch (error) {
    console.error('Expected preview-only failure:', error instanceof Error ? error.message : error);
  }
}

main();
