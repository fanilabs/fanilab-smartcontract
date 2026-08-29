# FaniLab TypeScript SDK

Typed SDK for interacting with FaniLab smart contracts on Stellar Soroban.

> Status: the package is intentionally versioned at `0.2.0` to match the protocol crates and is currently in a partial implementation state. The workspace includes working escrow and delivery clients and the additional contract clients/types added in this update, but the SDK is not yet considered a stable production release.

## Installation

```bash
npm install @fanilab/sdk
```

## SDK Status

This package is currently a typed API preview, not a connected runtime client. The generated method signatures are present, but the underlying Soroban invocation layer is not complete in this repository snapshot. The examples below intentionally fail at runtime with a clear "not implemented" message rather than silently pretending a chain call happened.

### Implemented vs. preview-only

| Client | Status | Notes |
|---|---|---|
| EscrowClient | Preview-only | Type signatures and argument shapes are available; live invocation is not yet implemented in the checked-in example. |
| DeliveryClient | Preview-only | Same as above for delivery methods and metadata payloads. |

## Quick Start

### Initialize Clients

```typescript
import { EscrowClient, DeliveryClient } from '@fanilab/sdk';
import { Keypair } from '@stellar/stellar-sdk';

const keypair = Keypair.fromSecret('SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX');

const invocation = {
  serverUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
  sourceAccount: 'GA123...',
  signer: keypair,
};
const escrowClient = new EscrowClient('CABC...', invocation);
const deliveryClient = new DeliveryClient('CBDE...', invocation);
```

### Create an Escrow

```typescript
import { EscrowClient } from '@fanilab/sdk';

const client = new EscrowClient('CABC...');

const result = await client.createEscrow({
  sender: 'GA123...',
  recipient: 'GB456...',
  driver: 'GC789...',
  deliveryId: BigInt(1),
  token: 'CCDE...',
  amount: BigInt(1_000_000_000), // 100 XLM (assuming 7 decimals)
  fleetId: BigInt(1),
});
```

### Release Escrow

```typescript
await client.releaseEscrow({
  caller: 'GA123...',
  deliveryId: BigInt(1),
});
```

### Get Escrow Details

```typescript
const escrow = await client.getEscrow(BigInt(1));
console.log(escrow.status); // EscrowStatus.Released
console.log(escrow.amount); // BigInt(1_000_000_000)
```

## API Reference

### EscrowClient

#### Methods

##### `init(params: InitParams, options?: ContractInvokeOptions): Promise<void>`
Initialize the escrow contract with admin and fee configuration.

**Parameters:**
- `params.admin`: Admin address
- `params.token`: Token contract address
- `params.platformFeeBps`: Platform fee in basis points (0-1000)

##### `createEscrow(params: CreateEscrowParams, options?: ContractInvokeOptions): Promise<string>`
Create a new escrow for a delivery.

**Parameters:**
- `params.sender`: Address funding the escrow
- `params.recipient`: Address receiving delivery confirmation
- `params.driver`: Address of the delivery driver
- `params.deliveryId`: Unique delivery identifier
- `params.token`: Token contract address
- `params.amount`: Escrow amount in token units
- `params.fleetId`: (Optional) Fleet ID if driver is part of a fleet

##### `releaseEscrow(params: ReleaseEscrowParams, options?: ContractInvokeOptions): Promise<void>`
Release escrowed funds to the driver after delivery confirmation.

**Parameters:**
- `params.caller`: Authorized caller (admin or recipient)
- `params.deliveryId`: Delivery identifier

##### `refundEscrow(params: RefundEscrowParams, options?: ContractInvokeOptions): Promise<void>`
Refund escrowed funds to the sender.

**Parameters:**
- `params.caller`: Authorized caller (admin or sender)
- `params.deliveryId`: Delivery identifier

##### `raiseDispute(params: RaiseDisputeParams, options?: ContractInvokeOptions): Promise<void>`
Raise a dispute on an escrow.

**Parameters:**
- `params.caller`: Dispute raiser (sender, recipient, or driver)
- `params.deliveryId`: Delivery identifier

##### `resolveDispute(params: ResolveDisputeParams, options?: ContractInvokeOptions): Promise<void>`
Resolve a dispute by releasing to driver or refunding sender.

**Parameters:**
- `params.caller`: Admin address (required)
- `params.deliveryId`: Delivery identifier
- `params.releaseToDriver`: `true` to pay driver, `false` to refund sender

##### `resolveDisputeSplit(params: ResolveDisputeSplitParams, options?: ContractInvokeOptions): Promise<void>`
Resolve a dispute by splitting funds between sender and driver.

**Parameters:**
- `params.caller`: Admin address (required)
- `params.deliveryId`: Delivery identifier
- `params.senderShareBps`: Sender's share in basis points (0-10000)

##### `getPlatformFee(): Promise<number>`
Get the current platform fee in basis points.

##### `getAdmin(): Promise<string>`
Get the admin address.

##### `getEscrow(deliveryId: bigint): Promise<EscrowRecord>`
Get an escrow record by delivery ID.

##### `getEscrowsBySender(sender: string): Promise<bigint[]>`
Get all escrow IDs for a sender.

##### `getEscrowsByRecipient(recipient: string): Promise<bigint[]>`
Get all escrow IDs for a recipient.

##### `getEscrowsByDriver(driver: string): Promise<bigint[]>`
Get all escrow IDs for a driver.

### DeliveryClient

#### Methods

##### `init(escrowContractId: string, identityContractId: string, options?: ContractInvokeOptions): Promise<void>`
Initialize the delivery contract.

**Parameters:**
- `escrowContractId`: Address of the escrow contract
- `identityContractId`: Address of the identity contract

##### `createDelivery(params: CreateDeliveryParams, options?: ContractInvokeOptions): Promise<bigint>`
Create a new delivery.

**Parameters:**
- `params.sender`: Address creating the delivery
- `params.recipient`: Address receiving delivery
- `params.deliveryId`: Unique delivery identifier
- `params.metadata`: Delivery metadata (location, items, notes, etc.)

##### `assignDriver(params: AssignDriverParams, options?: ContractInvokeOptions): Promise<void>`
Assign a driver to a delivery.

**Parameters:**
- `params.caller`: Authorized caller
- `params.deliveryId`: Delivery identifier
- `params.driver`: Driver address

##### `confirmDelivery(params: ConfirmDeliveryParams, options?: ContractInvokeOptions): Promise<void>`
Confirm that a delivery has been completed.

**Parameters:**
- `params.caller`: Recipient address
- `params.deliveryId`: Delivery identifier

##### `getDelivery(deliveryId: bigint): Promise<DeliveryRecord>`
Get a delivery record.

## Types

All types are exported from the main package for use in your application:

```typescript
import {
  EscrowStatus,
  DeliveryStatus,
  EscrowRecord,
  DeliveryRecord,
  ProtocolConfig,
} from '@fanilab/sdk';
```

## Error Handling

The SDK propagates contract errors with detailed messages:

```typescript
try {
  await client.releaseEscrow({
    caller: 'GA123...',
    deliveryId: BigInt(999),
  });
} catch (error) {
  console.error('Failed to release escrow:', error.message);
}
```

## Testing

```bash
npm run test
```

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for contribution guidelines.

## License

MIT
