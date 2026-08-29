# FaniLab-SmartContract 📦🔗

> **FaniLab** ni Jukwaa la Uendeshaji na Uhifadhi wa Pesa Powered by Blockchain, inayoundwa kuunganisha watu na biashara zinazohitaji kusafirisha bidhaa na wafanyabizanishi wa usafiri walay vailable. Hazina hii ina Soroban Smart Contracts za Stellar ambazo zina nguvu za mfumo wa uhifadhi wa Pesa Blockchain unaotumiwa na jukwaa la uendeshaji.

---

## 📖 Muhtasari wa Maudhui
- [Muhtasari](#-muhtasari)
- [Muundo wa hazina](#-muundo-wa-hazina)
- [Anza haraka](#-anza-haraka)
- [Mchango](#-mchango)

---

## 🌍 Muhtasari

Hazina hii ina mikataba ya Soroban ya FaniLab ya escrow, utoaji, mzozo, sifa na utawala. Inafuatia muundo wa sasa wa workspace na kusawazishwa na README ya Kiingereza.

Mradi umewekwa katika mikataba sita ya Rust na maktaba moja ya kawaida:
- `escrow_contract`
- `delivery_contract`
- `dispute_resolution_contract`
- `fleet_management_contract`
- `identity_reputation_contract`
- `settlement_contract`
- `shared_types`

## 📂 Muundo wa hazina

```text
fanilab-smartcontract/
├── Cargo.toml
├── CHANGELOG.md
├── README.md
├── contracts/
│   ├── delivery_contract/
│   │   ├── Cargo.toml
│   │   ├── lib.rs
│   │   └── test.rs
│   ├── dispute_resolution_contract/
│   │   ├── Cargo.toml
│   │   ├── lib.rs
│   │   └── test.rs
│   ├── escrow_contract/
│   │   ├── Cargo.toml
│   │   ├── lib.rs
│   │   └── test.rs
│   ├── fleet_management_contract/
│   │   ├── Cargo.toml
│   │   ├── lib.rs
│   │   └── test.rs
│   ├── identity_reputation_contract/
│   │   ├── Cargo.toml
│   │   ├── lib.rs
│   │   └── test.rs
│   ├── settlement_contract/
│   │   ├── Cargo.toml
│   │   └── lib.rs
│   └── shared_types/
│       ├── Cargo.toml
│       └── lib.rs
├── docs/
├── scripts/
├── sdk/
└── .github/
```

## ⚙️ Anza haraka

1. Sakinisha Rust na target ya Soroban inayofaa.
2. Kagua workspace ya Rust kutoka kwenye mzizi wa mradi: `cargo test`.
3. Compile mikataba kwa kutumia scripts na Makefile ya hazina.
4. Soma nyaraka za `docs/` kwa API, deployment na usalama.

## 🤝 Mchango

Hati hii ni tafsiri ya README ya Kiingereza na inapaswa kubaki inayofuata utekelezaji wa kweli wa mradi. Mabadiliko ya msimbo yanapaswa kuonyeshwa pia kwenye toleo la Kiingereza na hapa.

## ⚙️ Maagizo ya Kusakinisha

1. **Sakinisha Rust na huduma za kaida:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add wasm32v1-none
   ```

2. **Sakinisha Stellar CLI:**
   ```bash
   cargo install --locked stellar-cli
   ```

3. **Nakili hazina:**
   ```bash
   git clone https://github.com/your-org/FaniLab-SmartContract.git
   cd FaniLab-SmartContract
   ```

4. **Kuundwa kwa mkataba:**

   **Kwa Watumiaji wa Linux / macOS (kutumia Make):**
   
   Kuundwa kwa mkataba wote:
   ```bash
   make build
   ```
   
   Kuundwa kwa mkataba mahususi:
   ```bash
   make build-escrow
   make build-delivery
   make build-dispute
   ```
   
   Kueneza mjihano:
   ```bash
   make test
   ```
   
   **Kwa Watumiaji wa Windows (au bila Make):**
   
   Unaweza kueneza amri za `cargo` msingi moja kwa moja kutoka muundo wa mizizi:
   
   Kuundwa kwa mkataba wote:
   ```bash
   cargo build --target wasm32v1-none --release
   ```
   
   Kuundwa kwa mkataba mahususi:
   ```bash
   cargo build -p escrow_contract --target wasm32v1-none --release
   cargo build -p delivery_contract --target wasm32v1-none --release
   cargo build -p dispute_resolution_contract --target wasm32v1-none --release
   ```
   
   Kueneza mjihano:
   ```bash
   cargo test
   ```

## 🚢 Maagizo ya Kukamatia Mkataba

1. **Kusambaza utambulisho wako wa mtandao wa Stellar:**
   ```bash
   stellar keys generate deployer
   ```

2. **Fedha utambulisho kwenye Testnet:**
   ```bash
   stellar keys fund deployer --network testnet
   ```

3. **Kamatia mkataba wa Uhifadhi:**
   ```bash
   ./scripts/deploy-contract.sh escrow_contract
   ```

4. **Simu mkataba:**
   ```bash
   ./scripts/initialize-contract.sh escrow_contract
   ```

## 🔑 Tofauti za Mazingira

Nakili faili la `.env.example` hadi `.env` na jaza tofauti zako:

```env
STELLAR_NETWORK=testnet
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
CONTRACT_DEPLOYER_KEY=S...
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
```

## 🔄 Bomba la CI/CD

Mradi huu unatumia GitHub Actions kwa CI/CD. Bomba `.github/workflows/ci.yml` linasambaza kwa kiotomatiki:
- Kueneza uchingaji wa muundo wa Rust (`cargo fmt`).
- Kueneza kuchimba Rust (`cargo clippy`).
- Kukamatia mkataba wa Soroban.
- Kuthibitisha kuundwa kwa WASM.
- Kueneza mtihani wote wa uzamili na ujumuishaji.

## 📊 Hali ya Mradi

![CI Status](https://github.com/fanilab/FaniLab-SmartContract/workflows/Rust%20CI/badge.svg)
![Security Audit](https://github.com/fanilab/FaniLab-SmartContract/workflows/Security%20Audit/badge.svg)
[![codecov](https://codecov.io/gh/fanilab/FaniLab-SmartContract/branch/main/graph/badge.svg)](https://codecov.io/gh/fanilab/FaniLab-SmartContract)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

- **Toleo la Sasa**: 0.2.0
- **Hali ya Uchunguzi**: Inasubiri
- **Kumbatia Mtihani**: > 80%
- **Mtandao**: Testnet (Mainnet inakuja karibuni)

## 📚 Nyaraka

- [Kumbatia API](../API.md)
- [Mwongozo wa Kamata](../DEPLOYMENT.md)
- [Kuorodhesha Uchunguzi wa Usalama](../SECURITY_AUDIT.md)
- [Mwongozo wa Mtihani](../TESTING.md)
- [Mfano wa Kukamatia](../GOVERNANCE.md)
- [Kuandika Uamuzi wa Usanifu](../ARCHITECTURE_DECISION_RECORDS.md)

## 🤝 Mwongozo wa Michango

Tafadhali angalia faili letu la `CONTRIBUTING.md` kwa taarifa kuhusu kanuni yetu ya tabia na mfumo wa kuwasilisha maombi ya kuvuta.

## 🔒 Usalama

Usalama ni kipaumbele chetu kuu. Tafadhali angalia [SECURITY.md](../SECURITY.md) kwa sera yetu ya usalama na mfumo wa kuripoti miundombinu.

**Bug Bounty**: Tunatoa zawadi hadi $50,000 kwa utunzaji muhimu wa usalama.

## 📜 Leseni

Mradi huu unatolewa chini ya Leseni ya MIT - angalia faili la `LICENSE` kwa taarifa.

## 🌟 Shukurani

- Stellar Development Foundation kwa Soroban
- Jamii za Rust na Stellar
- Wote wangu wanaohitaji na wanaotumia

## 📞 Mawasiliano & Jamii

- **Tovuti**: https://fanilab.com
- **Barua pepe**: contact@fanilab.com
- **Twitter**: [@FaniLabHQ](https://twitter.com/FaniLabHQ)
- **Discord**: [Jibu jamii yetu](https://discord.gg/fanilab)
- **GitHub**: [Shirika la FaniLab](https://github.com/fanilab)

---

Kuundwa kwa ❤️ na Wanakioski wa FaniLab | Inapatia nguvu na Stellar Soroban
