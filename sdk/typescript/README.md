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
| DisputeResolutionClient | Preview-only | Dispute lifecycle, voting, and resolution methods are typed; live invocation is not yet implemented. |
| IdentityReputationClient | Preview-only | Identity registration and reputation scoring methods are typed; live invocation is not yet implemented. |
| FleetManagementClient | Preview-only | Fleet creation, membership, and driver management methods are typed; live invocation is not yet implemented. |
| SettlementClient | Preview-only | Settlement batching and payout methods are typed; live invocation is not yet implemented. |

## Quick Start

### Initialize Clients

```typescript
import {
  EscrowClient,
  DeliveryClient,
  DisputeResolutionClient,
  IdentityReputationClient,
  FleetManagementClient,
  SettlementClient,
} from '@fanilab/sdk';
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
const disputeClient = new DisputeResolutionClient('CDIS...', invocation);
const identityClient = new IdentityReputationClient('CIDN...', invocation);
const fleetClient = new FleetManagementClient('CFLT...', invocation);
const settlementClient = new SettlementClient('CSTL...', invocation);
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

### Raise and Resolve a Dispute

```typescript
import { DisputeResolutionClient } from '@fanilab/sdk';

const disputeClient = new DisputeResolutionClient('CDIS...', invocation);

// Open a dispute for a delivery
await disputeClient.raiseDispute({
  caller: 'GA123...',
  deliveryId: BigInt(1),
});

// Resolve the dispute in favour of the driver
await disputeClient.resolveDispute({
  caller: 'GADMIN...',
  deliveryId: BigInt(1),
  releaseToDriver: true,
});

const dispute = await disputeClient.getDispute(BigInt(1));
console.log(dispute.status); // DisputeStatus.Resolved
```

### Manage a Fleet

```typescript
import { FleetManagementClient } from '@fanilab/sdk';

const fleetClient = new FleetManagementClient('CFLT...', invocation);

// Create a fleet owned by the caller
const fleetId = await fleetClient.createFleet({
  owner: 'GA123...',
  name: 'Downtown Couriers',
});

// Add a driver to the fleet
await fleetClient.addDriver({
  caller: 'GA123...',
  fleetId,
  driver: 'GC789...',
});

const fleet = await fleetClient.getFleet(fleetId);
console.log(fleet.drivers); // ['GC789...']
```

## Error Handling

The SDK surfaces contract failures as typed error enums so callers can branch on
specific failure modes instead of parsing strings. Each client exports its own
error enum (for example `EscrowError`, `DeliveryError`, `DisputeResolutionError`,
`FleetManagementError`, and `SettlementError`).

```typescript
import { EscrowClient, EscrowError } from '@fanilab/sdk';

try {
  await client.releaseEscrow({ caller: 'GA123...', deliveryId: BigInt(1) });
} catch (err) {
  if (err instanceof EscrowError) {
    switch (err.code) {
      case EscrowErrorCode.Unauthorized:
        console.error('Caller is not allowed to release this escrow');
        break;
      case EscrowErrorCode.EscrowNotFound:
        console.error('No escrow exists for this delivery');
        break;
      case EscrowErrorCode.InvalidState:
        console.error('Escrow is not in a releasable state');
        break;
      default:
        throw err;
    }
  } else {
    throw err;
  }
}
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

##### `getTotalLocked(token: string): Promise<bigint>`
Get the total amount of the token currently locked by escrows.

##### `getUntrackedBalance(token: string): Promise<bigint>`
Get the contract's token balance that is not tracked as locked escrow funds.

##### `sweepUntrackedBalance(params: SweepUntrackedBalanceParams): Promise<bigint>`
Sweep the untracked balance to a recipient (admin only) and return the amount
transferred.

##### `setVolumeTiers(params: SetVolumeTiersParams): Promise<void>`
Configure the volume thresholds and fee discounts for senders (admin only).

##### `getVolumeTiers(): Promise<VolumeTier[]>`
Read the configured volume thresholds and discounts.

##### `getSenderVolume(sender: string): Promise<number>`
Read a sender's accumulated volume.

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

Delivery metadata accepts `pickupLocation`, `dropoffLocation`, and an optional
`cargoDescription` with `weightGrams`, `category`, and `fragile`. The cargo
category is one of `Documents`, `Electronics`, `Perishables`, `Clothing`, or
`General`. Optional `createdAt` and `estimatedDelivery` values are Unix
timestamps in seconds. If omitted, the SDK supplies the current creation time
and estimates delivery from the legacy `estimatedDistance` value. Reads return
the complete cargo description and all contract metadata fields.

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

##### `getDriverProfile(driver: string): Promise<DeliveryDriverProfile>`
Get the dri

/* … truncated 1736 chars — edit only what you need near the top … */
