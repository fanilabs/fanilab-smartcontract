#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_get_driver_preference() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SettlementContract);
    let client = SettlementContractClient::new(&env, &contract_id);

    let driver = Address::generate(&env);

    // The stub has no stored preference, so it must return None.
    assert_eq!(client.get_driver_preference(&driver), None);
}
