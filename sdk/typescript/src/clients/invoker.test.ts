import { describe, it, expect, vi, beforeEach } from "vitest";
import { Invoker } from "./invoker";

describe("Invoker cancellation", () => {
  let invoker: Invoker;
  let submitTransaction: ReturnType<typeof vi.fn>;
  let simulateTransaction: ReturnType<typeof vi.fn>;
  let getAccount: ReturnType<typeof vi.fn>;
  let extendTtl: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    submitTransaction = vi.fn().mockResolvedValue({ hash: "0xabc" });
    simulateTransaction = vi.fn().mockResolvedValue({ success: true });
    getAccount = vi.fn().mockResolvedValue({ sequence: 1n });
    extendTtl = vi.fn().mockResolvedValue(undefined);

    invoker = new Invoker({
      submitTransaction,
      simulateTransaction,
      getAccount,
      extendTtl,
    } as any);
  });

  it("sends no transactions when the signal is already aborted", async () => {
    const controller = new AbortController();
    controller.abort();

    await expect(
      invoker.invoke({
        payload: { function: "0x1::coin::transfer", typeArguments: [], functionArguments: [] },
        sender: "0x1",
        signal: controller.signal,
      } as any),
    ).rejects.toMatchObject({ name: "AbortError" });

    expect(getAccount).not.toHaveBeenCalled();
    expect(simulateTransaction).not.toHaveBeenCalled();
    expect(extendTtl).not.toHaveBeenCalled();
    expect(submitTransaction).not.toHaveBeenCalled();
  });

  it("stops before submission when aborted after simulation", async () => {
    const controller = new AbortController();
    simulateTransaction.mockImplementation(async () => {
      controller.abort();
      return { success: true };
    });

    await expect(
      invoker.invoke({
        payload: { function: "0x1::coin::transfer", typeArguments: [], functionArguments: [] },
        sender: "0x1",
        signal: controller.signal,
      } as any),
    ).rejects.toMatchObject({ name: "AbortError" });

    expect(submitTransaction).not.toHaveBeenCalled();
  });
});
