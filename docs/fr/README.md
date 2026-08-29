# FaniLab-SmartContract 📦🔗

> **FaniLab** est une plateforme de logistique et d'escrow alimentée par la blockchain, conçue pour connecter les personnes et entreprises ayant besoin de transporter des marchandises avec les fournisseurs de transport disponibles. Ce référentiel contient les contrats intelligents Stellar Soroban qui alimentent le système d'escrow blockchain utilisé par la plateforme logistique.

---

## 📖 Table des matières
- [Vue d'ensemble](#-vue-densemble)
- [Structure du dépôt](#-structure-du-dépôt)
- [Démarrage rapide](#-démarrage-rapide)
- [Contribution](#-contribution)

---

## 🌍 Vue d'ensemble

Ce dépôt contient les contrats Soroban de FaniLab pour la gestion d'escrow, la livraison, les litiges, la réputation et la gouvernance. Il reflète la structure actuelle du workspace et est synchronisé sur le README anglais.

Le projet est organisé en six contrats Rust et une bibliothèque partagée :
- `escrow_contract`
- `delivery_contract`
- `dispute_resolution_contract`
- `fleet_management_contract`
- `identity_reputation_contract`
- `settlement_contract`
- `shared_types`

## 📂 Structure du dépôt

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

## ⚙️ Démarrage rapide

1. Installer Rust et le target Soroban approprié.
2. Vérifier le workspace Rust depuis la racine : `cargo test` (si vous exécutez la suite locale).
3. Compiler les contrats selon les scripts et Makefile du dépôt.
4. Consulter les fichiers de documentation dans `docs/` pour l'API, le déploiement et la sécurité.

## 🤝 Contribution

Ce document est une traduction de la version anglaise du dépôt et doit rester aligné sur la réalité du code et de la structure du monorepo. Les changements fonctionnels doivent être appliqués dans le code source et le README anglais, puis reflétés ici.

## ⚙️ Instructions d'installation

1. **Installez Rust et les utilitaires standard :**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add wasm32v1-none
   ```

2. **Installez Stellar CLI :**
   ```bash
   cargo install --locked stellar-cli
   ```

3. **Clonez le référentiel :**
   ```bash
   git clone https://github.com/your-org/FaniLab-SmartContract.git
   cd FaniLab-SmartContract
   ```

4. **Construisez les contrats :**

   **Pour les utilisateurs Linux / macOS (utilisant Make) :**
   
   Pour construire tous les contrats :
   ```bash
   make build
   ```
   
   Pour construire des contrats spécifiques :
   ```bash
   make build-escrow
   make build-delivery
   make build-dispute
   ```
   
   Pour exécuter les tests :
   ```bash
   make test
   ```
   
   **Pour les utilisateurs Windows (ou sans Make) :**
   
   Vous pouvez exécuter les commandes `cargo` sous-jacentes directement depuis le répertoire racine :
   
   Pour construire tous les contrats :
   ```bash
   cargo build --target wasm32v1-none --release
   ```
   
   Pour construire des contrats spécifiques :
   ```bash
   cargo build -p escrow_contract --target wasm32v1-none --release
   cargo build -p delivery_contract --target wasm32v1-none --release
   cargo build -p dispute_resolution_contract --target wasm32v1-none --release
   ```
   
   Pour exécuter les tests :
   ```bash
   cargo test
   ```

## 🚢 Instructions de déploiement du contrat

1. **Configurez votre identité réseau Stellar :**
   ```bash
   stellar keys generate deployer
   ```

2. **Financez l'identité sur Testnet :**
   ```bash
   stellar keys fund deployer --network testnet
   ```

3. **Déployez le contrat Escrow :**
   ```bash
   ./scripts/deploy-contract.sh escrow_contract
   ```

4. **Initialisez le contrat :**
   ```bash
   ./scripts/initialize-contract.sh escrow_contract
   ```

## 🔑 Variables d'environnement

Copiez le fichier `.env.example` en `.env` et remplissez vos variables :

```env
STELLAR_NETWORK=testnet
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
CONTRACT_DEPLOYER_KEY=S...
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
```

## 🔄 Pipeline CI/CD

Ce projet utilise GitHub Actions pour CI/CD. Le pipeline `.github/workflows/ci.yml` est configuré pour automatiquement :
- Exécuter les vérifications de formatage Rust (`cargo fmt`).
- Exécuter le lint Rust (`cargo clippy`).
- Compiler les contrats Soroban.
- Vérifier la construction WASM.
- Exécuter tous les tests unitaires et d'intégration.

## 📊 Statut du projet

![CI Status](https://github.com/fanilab/FaniLab-SmartContract/workflows/Rust%20CI/badge.svg)
![Security Audit](https://github.com/fanilab/FaniLab-SmartContract/workflows/Security%20Audit/badge.svg)
[![codecov](https://codecov.io/gh/fanilab/FaniLab-SmartContract/branch/main/graph/badge.svg)](https://codecov.io/gh/fanilab/FaniLab-SmartContract)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

- **Version actuelle** : 0.2.0
- **Statut d'audit** : En attente
- **Couverture de test** : > 80%
- **Réseau** : Testnet (Mainnet bientôt)

## 📚 Documentation

- [Référence API](../API.md)
- [Guide de déploiement](../DEPLOYMENT.md)
- [Liste de contrôle d'audit de sécurité](../SECURITY_AUDIT.md)
- [Guide de test](../TESTING.md)
- [Modèle de gouvernance](../GOVERNANCE.md)
- [Enregistrements de décisions architecturales](../ARCHITECTURE_DECISION_RECORDS.md)

## 🤝 Directives de contribution

Veuillez consulter notre fichier `CONTRIBUTING.md` pour les détails sur notre code de conduite et le processus de soumission des demandes d'extraction.

## 🔒 Sécurité

La sécurité est notre priorité absolue. Veuillez consulter [SECURITY.md](../SECURITY.md) pour notre politique de sécurité et notre processus de signalement des vulnérabilités.

**Bug Bounty** : Nous offrons des récompenses jusqu'à $50 000 pour les découvertes de sécurité critiques.

## 📜 Licence

Ce projet est autorisé sous la licence MIT - voir le fichier `LICENSE` pour les détails.

## 🌟 Remerciements

- Stellar Development Foundation pour Soroban
- Les communautés Rust et Stellar
- Tous nos contributeurs et supporters

## 📞 Contact & Communauté

- **Site Web** : https://fanilab.com
- **Email** : contact@fanilab.com
- **Twitter** : [@FaniLabHQ](https://twitter.com/FaniLabHQ)
- **Discord** : [Rejoignez notre communauté](https://discord.gg/fanilab)
- **GitHub** : [Organisation FaniLab](https://github.com/fanilab)

---

Construit avec ❤️ par l'équipe FaniLab | Alimenté par Stellar Soroban
