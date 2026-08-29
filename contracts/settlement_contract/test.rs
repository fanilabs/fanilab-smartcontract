use super::*;
use soroban_sdk::testutils::Address as _;

#[test]
fn test_init() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(SettlementContract, ());
    let client = SettlementContractClient::new(&env, &contract_id);

    client.init(&admin);
    assert_eq!(client.get_admin(), admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_init_rejects_reinitialization() {
    let env = Env::default();
    env.mock_all_auths();

    let first_admin = Address::generate(&env);
    let second_admin = Address::generate(&env);
    let contract_id = env.register(SettlementContract, ());
    let client = SettlementContractClient::new(&env, &contract_id);

    client.init(&first_admin);
    client.init(&second_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_execute_settlement_swap_panics_when_unimplemented() {
    // This test enforces the production guard: execute_settlement_swap must
    // panic until Phase 3 settlement logic is implemented. This prevents
    // accidental deployment with silent no-op currency swaps.
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let caller = Address::generate(&env);
    let from_token = Address::generate(&env);
    let to_token = Address::generate(&env);
    let recipient = Address::generate(&env);

    let contract_id = env.register(SettlementContract, ());
    let client = SettlementContractClient::new(&env, &contract_id);

    client.init(&admin);

    // This call must return the typed swap-not-implemented error to guard
    // against accidental mainnet deployment with unimplemented settlement logic.
    client.execute_settlement_swap(
        &caller,
        &from_token,
        &to_token,
        &recipient,
        &1000i128,
        &900i128,
    );
}
