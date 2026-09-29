import { DeliveryClient } from "../clients/delivery.client";
import { Keypair } from "@stellar/stellar-sdk";

describe("DeliveryClient admin configuration bindings", () => {
  const contractId = "CDELIVERYCONTRACTIDXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
  const admin = Keypair.random();

  let client: DeliveryClient;

  beforeEach(() => {
    client = new DeliveryClient({
      contractId,
      networkPassphrase: "Test SDF Network ; September 2015",
      rpcUrl: "https://soroban-testnet.stellar.org",
      publicKey: admin.publicKey(),
    });
  });

  it("exposes setIdentityReputationContract", () => {
    expect(typeof client.setIdentityReputationContract).toBe("function");
  });

  it("exposes setEscrowContract", () => {
    expect(typeof client.setEscrowContract).toBe("function");
  });

  it("builds a set_identity_reputation_contract invocation", async () => {
    const reputationContract = "CREPUTATIONCONTRACTIDXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    const tx = await client.setIdentityReputationContract(reputationContract);

    expect(tx).toBeDefined();
    expect(tx.operations).toHaveLength(1);
    expect(tx.operations[0].type).toBe("invokeHostFunction");
  });

  it("builds a set_escrow_contract invocation", async () => {
    const escrowContract = "CESCROWCONTRACTIDXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    const tx = await client.setEscrowContract(escrowContract);

    expect(tx).toBeDefined();
    expect(tx.operations).toHaveLength(1);
    expect(tx.operations[0].type).toBe("invokeHostFunction");
  });
});
