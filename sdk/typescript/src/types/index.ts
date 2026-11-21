import { xdr } from "@stellar/stellar-base";

export interface AuthorizationEntry {
  credentials: xdr.SorobanCredentials;
  rootInvocation: xdr.SorobanAuthorizedInvocation;
}

export type AuthorizationEntrySigner = (
  entry: AuthorizationEntry,
) => Promise<AuthorizationEntry> | AuthorizationEntry;

export interface InvokeContractArgs {
  contractId: string;
  method: string;
  args?: xdr.ScVal[];
  authorizationEntries?: AuthorizationEntry[];
  signAuthEntries?: AuthorizationEntrySigner;
}
