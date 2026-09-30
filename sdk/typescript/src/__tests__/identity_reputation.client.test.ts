import fs from 'fs';
import path from 'path';

const CONTRACT_PATH = path.resolve(
  __dirname,
  '../../../../contracts/identity_reputation_contract/lib.rs',
);
const CLIENT_PATH = path.resolve(__dirname, '../clients/identity_reputation.client.ts');
const TYPES_PATH = path.resolve(__dirname, '../types/identity_reputation.types.ts');

// Issue #454 — the SDK must expose every moderation entry point the contract
// provides, so admin dashboards can ban, unban, and check bans. Assertions are
// made against the sources rather than by importing the client because the
// `ContractInvoker` import chain currently fails to typecheck against the
// installed @stellar/stellar-sdk (unrelated, pre-existing).
describe('IdentityReputationClient moderation bindings', () => {
  const contract = fs.readFileSync(CONTRACT_PATH, 'utf8');
  const client = fs.readFileSync(CLIENT_PATH, 'utf8');
  const types = fs.readFileSync(TYPES_PATH, 'utf8');

  const bindings: Array<{ method: string; fn: string; params?: string }> = [
    { method: 'suspendDriver', fn: 'suspend_driver', params: 'SuspendDriverParams' },
    { method: 'reinstateDriver', fn: 'reinstate_driver', params: 'ReinstateDriverParams' },
    { method: 'isDriverSuspended', fn: 'is_driver_suspended' },
  ];

  test.each(bindings)('$method invokes the contract entry point $fn', ({ method, fn }) => {
    expect(contract).toContain(`pub fn ${fn}(`);

    const signature = `async ${method}(`;
    expect(client).toContain(signature);
    expect(client).toContain(`'${fn}'`);
  });

  test.each(bindings.filter((b) => b.params))(
    '$method has a matching params interface',
    ({ fn, params }) => {
      expect(types).toContain(`export interface ${params} {`);
      expect(client).toContain(`${params},`);
      expect(client).toContain(`'${fn}', [address(params.admin), address(params.driver)]`);
    },
  );

  test('isDriverSuspended sends only the driver address and returns a boolean', () => {
    expect(client).toContain(
      "return Boolean(await this.invoker.call('is_driver_suspended', [address(driver)], options));",
    );
  });
});
