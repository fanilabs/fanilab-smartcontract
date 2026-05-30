#![no_std]

use soroban_sdk::{
	contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env,
};

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SupportedAsset {
	Xlm,
	Usdc,
	Ngnc,
	Kes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
	Admin,
	Xlm,
	Usdc,
	Ngnc,
	Kes,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SettlementError {
	AlreadyInitialized = 1,
	NotInitialized = 2,
	Unauthorized = 3,
}

fn asset_key(asset: SupportedAsset) -> DataKey {
	match asset {
		SupportedAsset::Xlm => DataKey::Xlm,
		SupportedAsset::Usdc => DataKey::Usdc,
		SupportedAsset::Ngnc => DataKey::Ngnc,
		SupportedAsset::Kes => DataKey::Kes,
	}
}

fn require_admin(env: &Env, caller: &Address) {
	let stored_admin: Address = env
		.storage()
		.instance()
		.get(&DataKey::Admin)
		.unwrap_or_else(|| panic_with_error!(env, SettlementError::NotInitialized));
	if stored_admin != *caller {
		panic_with_error!(env, SettlementError::Unauthorized);
	}
}

#[contract]
pub struct SettlementContract;

#[contractimpl]
impl SettlementContract {
	pub fn init(env: Env, admin: Address) {
		if env.storage().instance().has(&DataKey::Admin) {
			panic_with_error!(&env, SettlementError::AlreadyInitialized);
		}

		env.storage().instance().set(&DataKey::Admin, &admin);
		env.storage().instance().set(&DataKey::Xlm, &false);
		env.storage().instance().set(&DataKey::Usdc, &false);
		env.storage().instance().set(&DataKey::Ngnc, &false);
		env.storage().instance().set(&DataKey::Kes, &false);
	}

	pub fn set_supported_asset(env: Env, admin: Address, asset: SupportedAsset, supported: bool) {
		admin.require_auth();
		require_admin(&env, &admin);
		env.storage().instance().set(&asset_key(asset), &supported);
	}

	pub fn is_supported(env: Env, asset: SupportedAsset) -> bool {
		env.storage()
			.instance()
			.get(&asset_key(asset))
			.unwrap_or(false)
	}

	pub fn get_admin(env: Env) -> Address {
		env.storage()
			.instance()
			.get(&DataKey::Admin)
			.unwrap_or_else(|| panic_with_error!(&env, SettlementError::NotInitialized))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use soroban_sdk::testutils::Address as _;

	fn setup() -> (Env, SettlementContractClient<'static>, Address) {
		let env = Env::default();
		env.mock_all_auths();
		let contract_id = env.register(SettlementContract, ());
		let client = SettlementContractClient::new(&env, &contract_id);
		let admin = Address::generate(&env);
		client.init(&admin);
		(env, client, admin)
	}

	#[test]
	fn init_sets_admin_and_defaults() {
		let (env, client, admin) = setup();

		assert_eq!(client.get_admin(), admin);
		assert!(!client.is_supported(&SupportedAsset::Xlm));
		assert!(!client.is_supported(&SupportedAsset::Usdc));
		assert!(!client.is_supported(&SupportedAsset::Ngnc));
		assert!(!client.is_supported(&SupportedAsset::Kes));

		let xlm_key = env.as_contract(&client.address, || asset_key(SupportedAsset::Xlm));
		let usdc_key = env.as_contract(&client.address, || asset_key(SupportedAsset::Usdc));
		let ngnc_key = env.as_contract(&client.address, || asset_key(SupportedAsset::Ngnc));
		let kes_key = env.as_contract(&client.address, || asset_key(SupportedAsset::Kes));

		assert_ne!(xlm_key, usdc_key);
		assert_ne!(xlm_key, ngnc_key);
		assert_ne!(xlm_key, kes_key);
		assert_ne!(usdc_key, ngnc_key);
		assert_ne!(usdc_key, kes_key);
		assert_ne!(ngnc_key, kes_key);
	}

	#[test]
	fn can_toggle_supported_assets() {
		let (_env, client, admin) = setup();

		client.set_supported_asset(&admin, &SupportedAsset::Xlm, &true);
		client.set_supported_asset(&admin, &SupportedAsset::Usdc, &true);

		assert!(client.is_supported(&SupportedAsset::Xlm));
		assert!(client.is_supported(&SupportedAsset::Usdc));
		assert!(!client.is_supported(&SupportedAsset::Ngnc));
		assert!(!client.is_supported(&SupportedAsset::Kes));
	}
}

