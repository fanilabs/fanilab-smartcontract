#![no_std]

use soroban_sdk::{
	contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
	IntoVal, Symbol, Vec,
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
	Router,
	Xlm,
	Usdc,
	Ngnc,
	Kes,
	AssetPair(Address, Address),
	DriverPreference(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SettlementError {
	AlreadyInitialized = 1,
	NotInitialized = 2,
	Unauthorized = 3,
	UnsupportedAssetPair = 4,
	InvalidAmount = 5,
	SlippageExceeded = 6,
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

fn require_positive_amount(env: &Env, amount: i128) {
	if amount <= 0 {
		panic_with_error!(env, SettlementError::InvalidAmount);
	}
}

fn require_supported_pair(env: &Env, source_asset: &Address, destination_asset: &Address) {
	if source_asset == destination_asset {
		return;
	}

	let key = DataKey::AssetPair(source_asset.clone(), destination_asset.clone());
	let supported = env.storage().persistent().get(&key).unwrap_or(false);
	if !supported {
		panic_with_error!(env, SettlementError::UnsupportedAssetPair);
	}
}

fn get_router(env: &Env) -> Address {
	env.storage()
		.instance()
		.get(&DataKey::Router)
		.unwrap_or_else(|| panic_with_error!(env, SettlementError::NotInitialized))
}

fn pair_path(env: &Env, source_asset: Address, destination_asset: Address) -> Vec<Address> {
	let mut path = Vec::new(env);
	path.push_back(source_asset);
	path.push_back(destination_asset);
	path
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

	pub fn set_router(env: Env, admin: Address, router: Address) {
		admin.require_auth();
		require_admin(&env, &admin);
		env.storage().instance().set(&DataKey::Router, &router);
	}

	pub fn get_router(env: Env) -> Address {
		get_router(&env)
	}

	pub fn set_supported_asset_pair(
		env: Env,
		admin: Address,
		source_asset: Address,
		destination_asset: Address,
		supported: bool,
	) {
		admin.require_auth();
		require_admin(&env, &admin);

		if source_asset == destination_asset {
			panic_with_error!(&env, SettlementError::UnsupportedAssetPair);
		}

		let key = DataKey::AssetPair(source_asset, destination_asset);
		if supported {
			env.storage().persistent().set(&key, &true);
		} else {
			env.storage().persistent().remove(&key);
		}
	}

	pub fn is_supported_asset_pair(
		env: Env,
		source_asset: Address,
		destination_asset: Address,
	) -> bool {
		if source_asset == destination_asset {
			return true;
		}

		env.storage()
			.persistent()
			.get(&DataKey::AssetPair(source_asset, destination_asset))
			.unwrap_or(false)
	}

	pub fn register_driver_preference(env: Env, driver: Address, asset: Address) {
		driver.require_auth();
		env.storage()
			.persistent()
			.set(&DataKey::DriverPreference(driver), &asset);
	}

	pub fn get_driver_preference(env: Env, driver: Address) -> Option<Address> {
		env.storage()
			.persistent()
			.get(&DataKey::DriverPreference(driver))
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

	pub fn calculate_swap_estimate(
		env: Env,
		source_asset: Address,
		destination_asset: Address,
		amount_in: i128,
	) -> i128 {
		require_positive_amount(&env, amount_in);

		if source_asset == destination_asset {
			return amount_in;
		}

		require_supported_pair(&env, &source_asset, &destination_asset);

		let router = get_router(&env);
		let path = pair_path(&env, source_asset, destination_asset);
		env.invoke_contract(
			&router,
			&Symbol::new(&env, "quote_exact_input"),
			soroban_sdk::vec![&env, path.into_val(&env), amount_in.into_val(&env)],
		)
	}

	pub fn execute_settlement_swap(
		env: Env,
		caller: Address,
		source_asset: Address,
		destination_asset: Address,
		recipient: Address,
		amount_in: i128,
		min_amount_out: i128,
	) -> i128 {
		caller.require_auth();
		require_positive_amount(&env, amount_in);

		if min_amount_out < 0 {
			panic_with_error!(&env, SettlementError::InvalidAmount);
		}

		if source_asset == destination_asset {
			if amount_in < min_amount_out {
				panic_with_error!(&env, SettlementError::SlippageExceeded);
			}
			token::Client::new(&env, &source_asset).transfer(&caller, &recipient, &amount_in);
			return amount_in;
		}

		require_supported_pair(&env, &source_asset, &destination_asset);

		let estimated_amount_out = Self::calculate_swap_estimate(
			env.clone(),
			source_asset.clone(),
			destination_asset.clone(),
			amount_in,
		);
		if estimated_amount_out < min_amount_out {
			panic_with_error!(&env, SettlementError::SlippageExceeded);
		}

		let router = get_router(&env);
		let contract_address = env.current_contract_address();
		token::Client::new(&env, &source_asset).transfer(&caller, &contract_address, &amount_in);
		token::Client::new(&env, &source_asset).approve(
			&contract_address,
			&router,
			&amount_in,
			&(env.ledger().sequence() + 100),
		);

		let path = pair_path(&env, source_asset, destination_asset);
		let amount_out: i128 = env.invoke_contract(
			&router,
			&Symbol::new(&env, "swap_exact_tokens_for_tokens"),
			soroban_sdk::vec![
				&env,
				contract_address.into_val(&env),
				recipient.clone().into_val(&env),
				path.into_val(&env),
				amount_in.into_val(&env),
				min_amount_out.into_val(&env),
			],
		);

		if amount_out < min_amount_out {
			panic_with_error!(&env, SettlementError::SlippageExceeded);
		}

		env.events().publish(
			(Symbol::new(&env, "settlement_swap_executed"),),
			(caller, recipient, amount_in, amount_out),
		);

		amount_out
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
