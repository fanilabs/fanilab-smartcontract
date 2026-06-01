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
	PlatformFeeBps,
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

	/// Set the platform cross-border conversion fee in basis points (1 bps = 0.01%).
	/// Only the admin may call this. Maximum value 1000 bps (10%).
	pub fn set_platform_fee_bps(env: Env, admin: Address, fee_bps: u32) {
		admin.require_auth();
		require_admin(&env, &admin);
		if fee_bps > 1000 {
			panic_with_error!(&env, SettlementError::InvalidAmount);
		}
		env.storage().instance().set(&DataKey::PlatformFeeBps, &fee_bps);
	}

	/// Return the current platform fee in basis points (defaults to 0).
	pub fn get_platform_fee_bps(env: Env) -> u32 {
		env.storage()
			.instance()
			.get(&DataKey::PlatformFeeBps)
			.unwrap_or(0)
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

		let fee_bps: u32 = env
			.storage()
			.instance()
			.get(&DataKey::PlatformFeeBps)
			.unwrap_or(0);

		let admin: Address = env
			.storage()
			.instance()
			.get(&DataKey::Admin)
			.unwrap_or_else(|| panic_with_error!(&env, SettlementError::NotInitialized));

		if source_asset == destination_asset {
			if amount_in < min_amount_out {
				panic_with_error!(&env, SettlementError::SlippageExceeded);
			}
			let fee: i128 = (amount_in * fee_bps as i128) / 10000;
			let net = amount_in - fee;
			token::Client::new(&env, &source_asset).transfer(&caller, &recipient, &net);
			if fee > 0 {
				token::Client::new(&env, &source_asset).transfer(&caller, &admin, &fee);
			}
			return net;
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

		let path = pair_path(&env, source_asset.clone(), destination_asset.clone());
		let amount_out: i128 = env.invoke_contract(
			&router,
			&Symbol::new(&env, "swap_exact_tokens_for_tokens"),
			soroban_sdk::vec![
				&env,
				contract_address.into_val(&env),
				contract_address.into_val(&env),
				path.into_val(&env),
				amount_in.into_val(&env),
				min_amount_out.into_val(&env),
			],
		);

		if amount_out < min_amount_out {
			panic_with_error!(&env, SettlementError::SlippageExceeded);
		}

		let fee: i128 = (amount_out * fee_bps as i128) / 10000;
		let net = amount_out - fee;
		token::Client::new(&env, &destination_asset).transfer(&contract_address, &recipient, &net);
		if fee > 0 {
			token::Client::new(&env, &destination_asset).transfer(&contract_address, &admin, &fee);
		}

		env.events().publish(
			(Symbol::new(&env, "settlement_swap_executed"),),
			(caller, recipient, amount_in, net),
		);

		net
	}

}

#[cfg(test)]
mod test {
	use super::*;
	use soroban_sdk::{
		contract as soroban_contract, contractimpl as soroban_contractimpl,
		testutils::Address as _,
		token::{Client as TokenClient, StellarAssetClient},
	};

	// ── Mock router ───────────────────────────────────────────────────────────

	#[soroban_contract]
	pub struct MockRouter;

	#[soroban_contractimpl]
	impl MockRouter {
		/// Store the rate in basis points (10000 = 1:1, 9500 = 0.95x).
		pub fn set_rate(env: Env, rate_bps: i128) {
			env.storage().instance().set(&0u32, &rate_bps);
		}

		pub fn quote_exact_input(env: Env, _path: Vec<Address>, amount_in: i128) -> i128 {
			let rate: i128 = env.storage().instance().get(&0u32).unwrap_or(10000);
			amount_in * rate / 10000
		}

		pub fn swap_exact_tokens_for_tokens(
			env: Env,
			_caller: Address,
			recipient: Address,
			path: Vec<Address>,
			amount_in: i128,
			_min_amount_out: i128,
		) -> i128 {
			let dest_asset = path.get(1).unwrap();
			let rate: i128 = env.storage().instance().get(&0u32).unwrap_or(10000);
			let amount_out = amount_in * rate / 10000;
			TokenClient::new(&env, &dest_asset)
				.transfer(&env.current_contract_address(), &recipient, &amount_out);
			amount_out
		}
	}

	// ── Helpers ───────────────────────────────────────────────────────────────

	fn setup() -> (Env, SettlementContractClient<'static>, Address) {
		let env = Env::default();
		env.mock_all_auths();
		let contract_id = env.register(SettlementContract, ());
		let client = SettlementContractClient::new(&env, &contract_id);
		let admin = Address::generate(&env);
		client.init(&admin);
		(env, client, admin)
	}

	fn setup_with_router() -> (
		Env,
		SettlementContractClient<'static>,
		Address,   // admin
		Address,   // source_asset
		Address,   // dest_asset
		Address,   // router
	) {
		let env = Env::default();
		env.mock_all_auths();

		let contract_id = env.register(SettlementContract, ());
		let client = SettlementContractClient::new(&env, &contract_id);
		let admin = Address::generate(&env);
		client.init(&admin);

		let source_asset = env.register_stellar_asset_contract_v2(admin.clone()).address();
		let dest_asset = env.register_stellar_asset_contract_v2(admin.clone()).address();
		let router_id = env.register(MockRouter, ());

		client.set_router(&admin, &router_id);
		client.set_supported_asset_pair(&admin, &source_asset, &dest_asset, &true);

		(env, client, admin, source_asset, dest_asset, router_id)
	}

	// ── Basic tests ───────────────────────────────────────────────────────────

	#[test]
	fn init_sets_admin_and_defaults() {
		let (_env, client, admin) = setup();

		assert_eq!(client.get_admin(), admin);
		assert!(!client.is_supported(&SupportedAsset::Xlm));
		assert!(!client.is_supported(&SupportedAsset::Usdc));
		assert!(!client.is_supported(&SupportedAsset::Ngnc));
		assert!(!client.is_supported(&SupportedAsset::Kes));
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

	// ── Issue #92: Slippage protection tests ─────────────────────────────────

	#[test]
	fn same_asset_swap_succeeds_within_slippage() {
		let (env, client, admin) = setup();
		let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
		let caller = Address::generate(&env);
		let recipient = Address::generate(&env);

		StellarAssetClient::new(&env, &token).mint(&caller, &1000);

		let out = client.execute_settlement_swap(
			&caller,
			&token,
			&token,
			&recipient,
			&1000,
			&1000, // min_amount_out == amount_in: exact match
		);
		assert_eq!(out, 1000);
		assert_eq!(TokenClient::new(&env, &token).balance(&recipient), 1000);
	}

	#[test]
	#[should_panic(expected = "Error(Contract, #6)")]
	fn same_asset_swap_rejects_excessive_slippage() {
		let (env, client, admin) = setup();
		let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
		let caller = Address::generate(&env);
		let recipient = Address::generate(&env);

		StellarAssetClient::new(&env, &token).mint(&caller, &900);

		// min_amount_out (1000) > amount_in (900) → SlippageExceeded
		client.execute_settlement_swap(
			&caller,
			&token,
			&token,
			&recipient,
			&900,
			&1000,
		);
	}

	#[test]
	#[should_panic(expected = "Error(Contract, #6)")]
	fn cross_asset_swap_rejects_excessive_slippage() {
		let (env, client, admin, source_asset, dest_asset, router_id) = setup_with_router();
		let caller = Address::generate(&env);
		let recipient = Address::generate(&env);

		// Router will return 0.5x (5000 bps), but min_amount_out is 600
		MockRouterClient::new(&env, &router_id).set_rate(&5000);
		StellarAssetClient::new(&env, &source_asset).mint(&caller, &1000);

		// estimated_out = 500, min = 600 → SlippageExceeded
		client.execute_settlement_swap(
			&caller,
			&source_asset,
			&dest_asset,
			&recipient,
			&1000,
			&600,
		);
	}

	#[test]
	fn cross_asset_swap_succeeds_within_slippage() {
		let (env, client, admin, source_asset, dest_asset, router_id) = setup_with_router();
		let caller = Address::generate(&env);
		let recipient = Address::generate(&env);

		// Router returns 0.95x
		MockRouterClient::new(&env, &router_id).set_rate(&9500);
		StellarAssetClient::new(&env, &source_asset).mint(&caller, &1000);
		// Mint dest tokens to mock router so it can complete the swap
		StellarAssetClient::new(&env, &dest_asset).mint(&router_id, &1000);

		let out = client.execute_settlement_swap(
			&caller,
			&source_asset,
			&dest_asset,
			&recipient,
			&1000,
			&900, // accept up to 5% slippage
		);

		assert_eq!(out, 950);
		assert_eq!(TokenClient::new(&env, &dest_asset).balance(&recipient), 950);
	}

	// ── Issue #93: Platform FX fee tests ─────────────────────────────────────

	#[test]
	fn platform_fee_bps_defaults_to_zero() {
		let (_env, client, _admin) = setup();
		assert_eq!(client.get_platform_fee_bps(), 0);
	}

	#[test]
	fn admin_can_set_platform_fee_bps() {
		let (_env, client, admin) = setup();
		client.set_platform_fee_bps(&admin, &50); // 0.5%
		assert_eq!(client.get_platform_fee_bps(), 50);
	}

	#[test]
	#[should_panic(expected = "Error(Contract, #3)")]
	fn non_admin_cannot_set_platform_fee_bps() {
		let (env, client, _admin) = setup();
		let attacker = Address::generate(&env);
		client.set_platform_fee_bps(&attacker, &50);
	}

	#[test]
	#[should_panic(expected = "Error(Contract, #5)")]
	fn fee_bps_above_1000_is_rejected() {
		let (_env, client, admin) = setup();
		client.set_platform_fee_bps(&admin, &1001);
	}

	#[test]
	fn same_asset_swap_deducts_platform_fee() {
		let (env, client, admin) = setup();
		let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
		let caller = Address::generate(&env);
		let recipient = Address::generate(&env);

		StellarAssetClient::new(&env, &token).mint(&caller, &1000);
		client.set_platform_fee_bps(&admin, &100); // 1% fee

		let out = client.execute_settlement_swap(
			&caller,
			&token,
			&token,
			&recipient,
			&1000,
			&0,
		);

		// 1% fee on 1000 = 10; recipient gets 990
		assert_eq!(out, 990);
		assert_eq!(TokenClient::new(&env, &token).balance(&recipient), 990);
		assert_eq!(TokenClient::new(&env, &token).balance(&admin), 10);
		assert_eq!(TokenClient::new(&env, &token).balance(&caller), 0);
	}

	#[test]
	fn cross_asset_swap_deducts_platform_fee() {
		let (env, client, admin, source_asset, dest_asset, router_id) = setup_with_router();
		let caller = Address::generate(&env);
		let recipient = Address::generate(&env);

		MockRouterClient::new(&env, &router_id).set_rate(&10000); // 1:1 rate
		StellarAssetClient::new(&env, &source_asset).mint(&caller, &1000);
		StellarAssetClient::new(&env, &dest_asset).mint(&router_id, &1000);
		client.set_platform_fee_bps(&admin, &200); // 2% fee

		let out = client.execute_settlement_swap(
			&caller,
			&source_asset,
			&dest_asset,
			&recipient,
			&1000,
			&0,
		);

		// 1:1 swap → 1000 out, 2% fee = 20; recipient gets 980
		assert_eq!(out, 980);
		assert_eq!(TokenClient::new(&env, &dest_asset).balance(&recipient), 980);
		assert_eq!(TokenClient::new(&env, &dest_asset).balance(&admin), 20);
	}

	#[test]
	fn zero_fee_transfers_full_amount() {
		let (env, client, admin) = setup();
		let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
		let caller = Address::generate(&env);
		let recipient = Address::generate(&env);

		StellarAssetClient::new(&env, &token).mint(&caller, &500);
		// fee_bps = 0 (default)

		let out = client.execute_settlement_swap(
			&caller, &token, &token, &recipient, &500, &0,
		);

		assert_eq!(out, 500);
		assert_eq!(TokenClient::new(&env, &token).balance(&recipient), 500);
		assert_eq!(TokenClient::new(&env, &token).balance(&admin), 0);
	}
}
