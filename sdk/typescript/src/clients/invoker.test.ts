import { Address, Contract, Keypair, Networks, Operation, SorobanRpc, TransactionBuilder, xdr } from "@stellar/stellar-sdk";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Client } from "./client.js";
import { Invoker } from "./invoker.js";

const RPC_URL = "http://localhost:8000/rpc";
const CONTRACT_ID = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM";
const CONTRACT_ADDRESS = Address.fromString(CONTRACT_ID);

function makeAuthEntry(contract: Address): xdr.SorobanAuthorizationEntry {
  return new xdr.SorobanAuthorizationEntry({
    credentials: xdr.SorobanCredentials.sorobanCredentialsAddress(
      new xdr.SorobanAddressCredentials({
        address: contract.toScAddress(),
        nonce: xdr.Int64.fromString("1"),
        signatureExpirationLedger: 0,
        signature: xdr.ScVal.scvVoid(),
      }),
    ),
    rootInvocation: new xdr.SorobanAuthorizedInvocation({
      function: xdr.SorobanAuthorizedFunction.sorobanAuthorizedFunctionTypeContractFn(
        new xdr.InvokeContractArgs({
          contractAddress: contract.toScAddress(),
          functionName: "transfer",
          args: [],
        }),
      ),
      subInvocations: [],
    }),
  });
}

describe("Invoker", () => {
  let server: SorobanRpc.Server;
  let client: Client;
  let invoker: Invoker;

  beforeEach(() => {
    server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });
    client = new Client({
      contractId: CONTRACT_ID,
      networkPassphrase: Networks.TESTNET,
      rpcUrl: RPC_URL,
      allowHttp: true,
    });
    invoker = new Invoker(client, server);
  });

  it("signs the transaction envelope when no auth hook is provided", async () => {
    const source = Keypair.random();
    const tx = new TransactionBuilder(await server.getAccount(source.publicKey()), {
      fee: "100",
      networkPassphrase: Networks.TESTNET,
    })
      .addOperation(Operation.invokeContractFunction({
        contract: CONTRACT_ID,
        function: "transfer",
        args: [],
      }))
      .setTimeout(30)
      .build();

    const signSpy = vi.spyOn(server, "prepareTransaction").mockResolvedValue(tx);
    const signed = await invoker.invoke(source, tx);

    expect(signSpy).toHaveBeenCalled();
    expect(signed.signatures.length).toBeGreaterThan(0);
  });

  it("invokes the auth hook with entries before assembly", async () => {
    const source = Keypair.random();
    const entry = makeAuthEntry(CONTRACT_ADDRESS);
    const tx = new TransactionBuilder(await server.getAccount(source.publicKey()), {
      fee: "100",
      networkPassphrase: Networks.TESTNET,
    })
      .addOperation(Operation.invokeContractFunction({
        contract: CONTRACT_ID,
        function: "transfer",
        args: [],
      }))
      .setTimeout(30)
      .build();

    vi.spyOn(server, "prepareTransaction").mockResolvedValue(tx);

    const hook = vi.fn(async (entries: xdr.SorobanAuthorizationEntry[]) => entries);
    await invoker.invoke(source, tx, { signAuthEntries: hook });

    expect(hook).toHaveBeenCalledTimes(1);
    expect(hook.mock.calls[0][0]).toEqual([entry]);
  });

  it("supports contract address authorization entries", async () => {
    const source = Keypair.random();
    const entry = makeAuthEntry(CONTRACT_ADDRESS);
    const tx = new TransactionBuilder(await server.getAccount(source.publicKey()), {
      fee: "100",
      networkPassphrase: Networks.TESTNET,
    })
      .addOperation(Operation.invokeContractFunction({
        contract: CONTRACT_ID,
        function: "transfer",
        args: [],
      }))
      .setTimeout(30)
      .build();

    vi.spyOn(server, "prepareTransaction").mockResolvedValue(tx);

    const hook = vi.fn(async (entries: xdr.SorobanAuthorizationEntry[]) => {
      return entries.map((e) => {
        const creds = e.credentials().address();
        expect(creds.address().switch()).toBe(xdr.ScAddressType.scAddressTypeContract());
        return e;
      });
    });

    await invoker.invoke(source, tx, { signAuthEntries: hook });
    expect(hook).toHaveBeenCalledTimes(1);
  });
});
