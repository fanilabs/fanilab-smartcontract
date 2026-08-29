export interface InitParams {
  admin: string;
}

export interface ExecuteSettlementSwapParams {
  caller: string;
  fromToken: string;
  toToken: string;
  recipient: string;
  amount: bigint;
  minAmountOut: bigint;
}
