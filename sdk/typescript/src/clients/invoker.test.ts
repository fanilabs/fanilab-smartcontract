import { describe, it, expect, vi, beforeEach } from "vitest";
import { Invoker } from "./invoker";

describe("Invoker polling validation", () => {
  let sendTransaction: ReturnType<typeof vi.fn>;
  let invoker: Invoker;

  beforeEach(() => {
    sendTransaction = vi.fn().mockResolvedValue({ hash: "0xabc" });
    invoker = new Invoker({
      sendTransaction,
      waitForResult: vi.fn(),
    } as any);
  });

  it("rejects a non-positive polling interval before sending", async () => {
    await expect(
      invoker.invoke({ pollingIntervalMs: 0 } as any),
    ).rejects.toThrow(/polling/i);
    expect(sendTransaction).not.toHaveBeenCalled();
  });

  it("rejects a non-positive polling timeout before sending", async () => {
    await expect(
      invoker.invoke({ pollingTimeoutMs: -1 } as any),
    ).rejects.toThrow(/polling/i);
    expect(sendTransaction).not.toHaveBeenCalled();
  });

  it("rejects a timeout shorter than the interval before sending", async () => {
    await expect(
      invoker.invoke({ pollingIntervalMs: 1000, pollingTimeoutMs: 500 } as any),
    ).rejects.toThrow(/polling/i);
    expect(sendTransaction).not.toHaveBeenCalled();
  });

  it("sends the transaction when polling settings are valid", async () => {
    await invoker.invoke({ pollingIntervalMs: 100, pollingTimeoutMs: 5000 } as any);
    expect(sendTransaction).toHaveBeenCalledTimes(1);
  });
});
