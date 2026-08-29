# FaniLab-SmartContract 📦🔗

> **FaniLab** é uma Plataforma de Logística e Depósito em Caução com Blockchain, projetada para conectar indivíduos e empresas que precisam transportar mercadorias com provedores de transporte disponíveis. Este repositório contém os contratos inteligentes Stellar Soroban que alimentam o sistema de depósito em caução blockchain usado pela plataforma de logística.

---

## 📖 Índice
- [Visão geral](#-visão-geral)
- [Estrutura do repositório](#-estrutura-do-repositório)
- [Guia rápido](#-guia-rápido)
- [Contribuição](#-contribuição)

---

## 🌍 Visão geral

Este repositório contém os contratos Soroban do FaniLab para escrow, entrega, disputa, reputação e governança. Ele reflete a estrutura atual do workspace e segue a versão em inglês.

O projeto está organizado em seis contratos Rust e uma biblioteca compartilhada:
- `escrow_contract`
- `delivery_contract`
- `dispute_resolution_contract`
- `fleet_management_contract`
- `identity_reputation_contract`
- `settlement_contract`
- `shared_types`

## 📂 Estrutura do repositório

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

## ⚙️ Guia rápido

1. Instale Rust e o target Soroban apropriado.
2. Confira o workspace do Rust na raiz do projeto: `cargo test`.
3. Compile os contratos usando os scripts e o Makefile do repositório.
4. Consulte a documentação em `docs/` para API, implantação e segurança.

## 🤝 Contribuição

Este documento é uma tradução do README em inglês e precisa permanecer alinhado com a implementação real do projeto. Alterações de código devem ser refletidas também na versão em inglês e neste arquivo.

## ⚙️ Instruções de Instalação

1. **Instale Rust e utilitários padrão:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add wasm32v1-none
   ```

2. **Instale Stellar CLI:**
   ```bash
   cargo install --locked stellar-cli
   ```

3. **Clone o repositório:**
   ```bash
   git clone https://github.com/your-org/FaniLab-SmartContract.git
   cd FaniLab-SmartContract
   ```

4. **Construa os contratos:**

   **Para Usuários Linux / macOS (usando Make):**
   
   Para construir todos os contratos:
   ```bash
   make build
   ```
   
   Para construir contratos específicos:
   ```bash
   make build-escrow
   make build-delivery
   make build-dispute
   ```
   
   Para executar testes:
   ```bash
   make test
   ```
   
   **Para Usuários Windows (ou sem Make):**
   
   Você pode executar os comandos `cargo` subjacentes diretamente do diretório raiz:
   
   Para construir todos os contratos:
   ```bash
   cargo build --target wasm32v1-none --release
   ```
   
   Para construir contratos específicos:
   ```bash
   cargo build -p escrow_contract --target wasm32v1-none --release
   cargo build -p delivery_contract --target wasm32v1-none --release
   cargo build -p dispute_resolution_contract --target wasm32v1-none --release
   ```
   
   Para executar testes:
   ```bash
   cargo test
   ```

## 🚢 Instruções de Implantação de Contrato

1. **Configure sua identidade de rede Stellar:**
   ```bash
   stellar keys generate deployer
   ```

2. **Financie a identidade no Testnet:**
   ```bash
   stellar keys fund deployer --network testnet
   ```

3. **Implante o contrato Escrow:**
   ```bash
   ./scripts/deploy-contract.sh escrow_contract
   ```

4. **Inicialize o contrato:**
   ```bash
   ./scripts/initialize-contract.sh escrow_contract
   ```

## 🔑 Variáveis de Ambiente

Copie o arquivo `.env.example` para `.env` e preencha suas variáveis:

```env
STELLAR_NETWORK=testnet
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
CONTRACT_DEPLOYER_KEY=S...
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
```

## 🔄 Pipeline CI/CD

Este projeto usa GitHub Actions para CI/CD. O pipeline `.github/workflows/ci.yml` é configurado para automaticamente:
- Executar verificações de formatação Rust (`cargo fmt`).
- Executar linting Rust (`cargo clippy`).
- Compilar os contratos Soroban.
- Verificar a compilação WASM.
- Executar todos os testes unitários e de integração.

## 📊 Status do Projeto

![CI Status](https://github.com/fanilab/FaniLab-SmartContract/workflows/Rust%20CI/badge.svg)
![Security Audit](https://github.com/fanilab/FaniLab-SmartContract/workflows/Security%20Audit/badge.svg)
[![codecov](https://codecov.io/gh/fanilab/FaniLab-SmartContract/branch/main/graph/badge.svg)](https://codecov.io/gh/fanilab/FaniLab-SmartContract)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

- **Versão Atual**: 0.2.0
- **Status da Auditoria**: Pendente
- **Cobertura de Teste**: > 80%
- **Rede**: Testnet (Mainnet em breve)

## 📚 Documentação

- [Referência de API](../API.md)
- [Guia de Implantação](../DEPLOYMENT.md)
- [Lista de Verificação de Auditoria de Segurança](../SECURITY_AUDIT.md)
- [Guia de Testes](../TESTING.md)
- [Modelo de Governança](../GOVERNANCE.md)
- [Registros de Decisão de Arquitetura](../ARCHITECTURE_DECISION_RECORDS.md)

## 🤝 Diretrizes de Contribuição

Por favor, consulte nosso arquivo `CONTRIBUTING.md` para detalhes sobre nosso código de conduta e o processo de envio de pull requests.

## 🔒 Segurança

A segurança é nossa prioridade máxima. Por favor, consulte [SECURITY.md](../SECURITY.md) para nossa política de segurança e processo de relatório de vulnerabilidades.

**Bug Bounty**: Oferecemos recompensas até $50.000 por descobertas críticas de segurança.

## 📜 Licença

Este projeto está licenciado sob a Licença MIT - consulte o arquivo `LICENSE` para detalhes.

## 🌟 Agradecimentos

- Stellar Development Foundation por Soroban
- As comunidades Rust e Stellar
- Todos os nossos contribuidores e apoiadores

## 📞 Contato & Comunidade

- **Website**: https://fanilab.com
- **Email**: contact@fanilab.com
- **Twitter**: [@FaniLabHQ](https://twitter.com/FaniLabHQ)
- **Discord**: [Junte-se à nossa comunidade](https://discord.gg/fanilab)
- **GitHub**: [Organização FaniLab](https://github.com/fanilab)

---

Construído com ❤️ pela Equipe FaniLab | Alimentado por Stellar Soroban
