# FaniLab-SmartContract 📦🔗

> **FaniLab** is a Blockchain-Powered Logistics & Escrow Delivery Platform designed to connect individuals and businesses who need to transport goods with available transport providers. This repository contains the Stellar Soroban smart contracts powering the blockchain escrow system used by the logistics platform.

---

## 📖 Table of Contents
- [Project Overview](#-project-overview)
- [The Core Problem](#-the-core-problem)
- [The Solution](#-the-solution)
- [How It Works (Escrow Payment Model)](#-how-it-works-escrow-payment-model)
- [Financial Inclusion Benefits](#-financial-inclusion-benefits)
- [Target Market](#-target-market)
- [Revenue Model](#-revenue-model)
- [Smart Contract Architecture](#-smart-contract-architecture)
- [Technology Stack](#-technology-stack)
- [Platform Features](#-platform-features)
- [Development Phases](#-development-phases)
- [Repository Structure](#-repository-structure)
- [Installation Instructions](#-installation-instructions)
- [Contract Deployment Instructions](#-contract-deployment-instructions)
- [Environment Variables](#-environment-variables)
- [CI/CD Pipeline](#-cicd-pipeline)
- [Contribution Guidelines](#-contribution-guidelines)
- [License](#-license)

---

## 🌍 Project Overview

FaniLab is composed of three main repositories:
1. **FaniLab-Frontend**: Stack: Next.js + TypeScript + TailwindCSS
2. **FaniLab-Backend**: Stack: Node.js + Express.js + TypeScript + MongoDB
3. **FaniLab-SmartContract** *(This Repository)*: Stack: Stellar Soroban + Rust

The smart contract repository powers the **blockchain escrow system** used by the logistics platform.

## ⚠️ The Core Problem

Traditional logistics and delivery networks often suffer from:
- Lack of trust between senders and independent delivery drivers.
- High fees and delayed settlements for drivers.
- Inefficient utilization of existing transportation assets.
- Difficulties for small operators to access global or cross-border logistics economies.

## 💡 The Solution

FaniLab creates a **shared decentralized logistics economy** by allowing existing transportation assets to participate securely in delivery operations. Transport providers include:
- Motorcycle delivery riders
- Courier agents
- Van drivers
- Truck operators
- Independent transport owners

By leveraging **blockchain escrow smart contracts**, FaniLab protects both senders and delivery agents, ensuring goods are transported securely and payments are settled instantly upon delivery confirmation.

## 🔄 How It Works (Escrow Payment Model)

FaniLab ensures **trustless logistics transactions** through the following workflow:

1. **Customer creates delivery request**: Sender initiates a delivery order.
2. **Payment is locked in escrow**: The smart contract securely holds the payment.
3. **Driver accepts delivery**: A driver is assigned to the task.
4. **Goods are transported**: The driver fulfills the logistics process.
5. **Recipient confirms delivery**: The recipient verifies the arrival of the goods.
6. **Escrow contract releases payment to driver**: Instant settlement on the Stellar network.

## 🤝 Financial Inclusion Benefits

The platform is built to empower individuals and small businesses, enabling:
- **Independent delivery drivers**
- **Small logistics businesses**
- **Rural transport operators**
- **Cross-border merchants**

to seamlessly participate in a global logistics economy powered by the Stellar blockchain.

## 🎯 Target Market

Our primary target audience includes:
- African logistics networks
- SME merchants
- E-commerce sellers
- Courier startups
- Transport unions
- Cross-border trade operators

## 💰 Revenue Model

FaniLab generates revenue through the following streams:
- Escrow service fees
- Delivery commission fees
- Cross-border settlement fees
- Enterprise logistics integrations
- Premium logistics analytics

## 🏗️ Smart Contract Architecture

The FaniLab smart contracts are the backbone of the trustless logistics protocol. They are responsible for:
- Escrow payment locking
- Escrow payment release
- Delivery verification
- Transaction settlement
- Delivery state validation
- On-chain logistics metadata

### Functional Requirements

The contract architecture supports:
- **Escrow Creation**: Lock payment when a delivery is created.
- **Driver Acceptance**: Driver accepts the delivery assignment.
- **Delivery Confirmation**: Recipient confirms package arrival.
- **Escrow Release**: Payment is released to the driver after confirmation.
- **Dispute Handling**: Escrow can be paused for dispute resolution.

### Event Emission
The contracts emit critical events for off-chain indexing:
- `delivery_created`
- `escrow_funded`
- `driver_assigned`
- `delivery_confirmed`
- `escrow_released`

## 🛠️ Technology Stack

- **Stellar Blockchain**
- **Soroban Smart Contracts**
- **Rust**
- **Soroban SDK**
- **Stellar CLI**
- **Soroban CLI**
- **WASM smart contract compilation**

## ✨ Platform Features

- Decentralized Escrow Management
- Instant Driver Settlement
- Verifiable Delivery States
- Immutable Logistics Metadata
- Trustless Dispute Resolution

## 🚀 Development Phases

### Phase 1 — Escrow MVP Smart Contract
**Focus:** Minimal smart contract to support escrow-based delivery payments.
- Escrow payment locking
- Delivery ID registration
- Escrow storage state
- Payment release mechanism

### Phase 2 — Logistics Smart Contract Expansion
**Focus:** Advanced tracking and shipment metadata.
- Driver assignment tracking
- Delivery status updates
- Delivery confirmation events
- Shipment metadata storage

### Phase 3 — Full Blockchain Logistics Protocol
**Focus:** Decentralized governance and cross-border capabilities.
- Dispute resolution mechanism
- Reputation scoring for drivers
- Decentralized delivery verification
- Cross-border payment settlement

## 📂 Repository Structure

```text
FaniLab-SmartContract/
├── contracts/
│   ├── escrow_contract/
│   │   └── lib.rs
│   ├── delivery_contract/
│   │   └── lib.rs
│   └── shared_types/
│       └── lib.rs
├── src/
│   ├── events/
│   ├── errors/
│   ├── storage/
│   └── interfaces/
├── tests/
│   ├── integration_tests/
│   └── contract_tests/
├── scripts/
│   ├── deployment/
│   ├── build/
│   ├── initialize/
│   ├── deploy-contract.sh
│   └── initialize-contract.sh
├── docs/
│   ├── architecture/
│   │   ├── smart-contract-architecture.md
│   │   └── event-system.md
│   ├── contract-design/
│   │   └── escrow-design.md
│   └── protocol/
│       └── delivery-protocol.md
├── deploy/
│   ├── testnet/
│   └── mainnet/
├── .github/
│   └── workflows/
│       └── ci.yml
├── Cargo.toml
├── Cargo.lock
├── Makefile
├── .env.example
├── README.md
├── LICENSE
├── CONTRIBUTING.md
└── SECURITY.md
```

## ⚙️ Installation Instructions

1. **Install Rust and standard utilities:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add wasm32-unknown-unknown
   ```

2. **Install Stellar CLI:**
   ```bash
   cargo install --locked stellar-cli
   ```

3. **Clone the repository:**
   ```bash
   git clone https://github.com/your-org/FaniLab-SmartContract.git
   cd FaniLab-SmartContract
   ```

4. **Build the contracts:**

   **For Linux / macOS Users (using Make):**
   
   To build all contracts:
   ```bash
   make build
   ```
   
   To build specific contracts:
   ```bash
   make build-escrow
   make build-delivery
   make build-dispute
   ```
   
   To run tests:
   ```bash
   make test
   ```
   
   **For Windows Users (or users without Make):**
   
   You can run the underlying `cargo` commands directly from the root directory:
   
   To build all contracts:
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```
   
   To build specific contracts:
   ```bash
   cargo build -p escrow_contract --target wasm32-unknown-unknown --release
   cargo build -p delivery_contract --target wasm32-unknown-unknown --release
   cargo build -p dispute_resolution_contract --target wasm32-unknown-unknown --release
   ```
   
   To run tests:
   ```bash
   cargo test
   ```

## 🚢 Contract Deployment Instructions

1. **Configure your Stellar network identity:**
   ```bash
   stellar keys generate deployer
   ```

2. **Fund the identity on Testnet:**
   ```bash
   stellar keys fund deployer --network testnet
   ```

3. **Deploy the Escrow contract:**
   ```bash
   ./scripts/deploy-contract.sh escrow_contract
   ```

4. **Initialize the contract:**
   ```bash
   ./scripts/initialize-contract.sh <CONTRACT_ID>
   ```

## 🔑 Environment Variables

Copy the `.env.example` file to `.env` and fill in your variables:

```env
STELLAR_NETWORK=testnet
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
CONTRACT_DEPLOYER_KEY=S...
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
```

## 🔄 CI/CD Pipeline

This project uses GitHub Actions for CI/CD. The pipeline `.github/workflows/ci.yml` is configured to automatically:
- Run Rust formatting checks (`cargo fmt`).
- Run Rust linting (`cargo clippy`).
- Compile the Soroban contracts.
- Verify the WASM build.
- Execute all unit and integration tests.

## 📊 Project Status

![CI Status](https://github.com/fanilab/FaniLab-SmartContract/workflows/Rust%20CI/badge.svg)
![Security Audit](https://github.com/fanilab/FaniLab-SmartContract/workflows/Security%20Audit/badge.svg)
[![codecov](https://codecov.io/gh/fanilab/FaniLab-SmartContract/branch/main/graph/badge.svg)](https://codecov.io/gh/fanilab/FaniLab-SmartContract)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

- **Current Version**: 0.2.0
- **Audit Status**: Pending
- **Test Coverage**: > 80%
- **Network**: Testnet (Mainnet coming soon)

## 📚 Documentation

- [API Reference](docs/API.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [Security Audit Checklist](docs/SECURITY_AUDIT.md)
- [Testing Guide](docs/TESTING.md)
- [Governance Model](docs/GOVERNANCE.md)
- [Architecture Decision Records](docs/ARCHITECTURE_DECISION_RECORDS.md)

## 🤝 Contribution Guidelines

Please review our `CONTRIBUTING.md` file for details on our code of conduct, and the process for submitting pull requests to us.

## 🔒 Security

Security is our top priority. Please see [SECURITY.md](SECURITY.md) for our security policy and vulnerability reporting process.

**Bug Bounty**: We offer rewards up to $50,000 for critical security findings.

## 📜 License

This project is licensed under the MIT License - see the `LICENSE` file for details.

## 🌟 Acknowledgments

- Stellar Development Foundation for Soroban
- The Rust and Stellar communities
- All our contributors and supporters

## 📞 Contact & Community

- **Website**: https://fanilab.com
- **Email**: contact@fanilab.com
- **Twitter**: [@FaniLabHQ](https://twitter.com/FaniLabHQ)
- **Discord**: [Join our community](https://discord.gg/fanilab)
- **GitHub**: [FaniLab Organization](https://github.com/fanilab)

---

Built with ❤️ by the FaniLab Team | Powered by Stellar Soroban
