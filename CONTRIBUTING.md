# Contributing to SwiftChain-SmartContract

Thank you for your interest in contributing to **SwiftChain-SmartContract**! We welcome contributions from the community to help build a trustless, Blockchain-Powered Logistics & Escrow Delivery Platform on the Stellar network. Whether you are fixing bugs, proposing new features, or optimizing gas usage, your help is incredibly valuable.

## Getting Started

1.  **Fork the repository** on GitHub.
2.  **Clone your fork** locally:
    ```bash
    git clone https://github.com/your-username/SwiftChain-SmartContract.git
    cd SwiftChain-SmartContract
    ```
3.  **Create a branch** for your feature or bug fix:
    ```bash
    git checkout -b feature/my-new-feature
    ```

## Development Workflow

The SwiftChain Smart Contract repository is structured as a **Cargo Workspace**. This means multiple smart contracts live in the same repository and share dependencies. 

The workspace consists of three main crates located in the `contracts/` directory:

*   **`shared_types/`**: The most critical crate. It acts as the common language between all contracts. It contains shared structs (e.g., `DeliveryDetails`), enums (e.g., `DeliveryStatus`), custom errors, and event definitions. *If a data structure needs to be passed between the Escrow and Delivery contracts, it MUST be defined here.*
*   **`escrow_contract/`**: Handles the locking, releasing, and refunding of funds based on delivery states.
*   **`delivery_contract/`**: Manages the logistics metadata, driver assignments, and delivery confirmation logic.

### Prerequisites

You will need the following installed:

*   **Rust**: Latest stable toolchain. `https://www.rust-lang.org/tools/install`
*   **WASM Target**: Required for compiling Soroban contracts.
    ```bash
    rustup target add wasm32-unknown-unknown
    ```
*   **Stellar CLI**: For compiling, deploying, and invoking contracts.
    ```bash
    cargo install --locked stellar-cli
    ```

### Building the Contracts

Because this is a Cargo workspace, you **do not** need to `cd` into individual contract directories to build them. You can manage everything from the root directory.

**For Linux / macOS Users (using Make):**
```bash
# Build all contracts
make build

# Build specific contracts
make build-escrow
make build-delivery
make build-dispute
```

**For Windows Users (or users without Make):**
```bash
# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Build specific contracts
cargo build -p escrow_contract --target wasm32-unknown-unknown --release
cargo build -p delivery_contract --target wasm32-unknown-unknown --release
cargo build -p dispute_resolution_contract --target wasm32-unknown-unknown --release
```

## Testing Guidelines

Testing is a critical part of Soroban smart contract development.

1. **Where to write tests:** Tests should be written in a `test.rs` file located directly inside the specific contract's directory (e.g., `contracts/escrow_contract/test.rs`).
2. **Shared Types Testing:** Even though `shared_types` is not a smart contract, any helper functions, struct validation logic, or complex enum implementations added here *must* be accompanied by unit tests.
3. **Running tests:** You can run all tests across the entire workspace from the root directory:

**Using Make:**
```bash
make test
```

**Using Cargo (Windows/All):**
```bash
cargo test
```

*Note: Running `cargo test` generates `.json` ledger snapshot files (e.g., `test_init_and_get_status.1.json`). These are automatically ignored by our `.gitignore` and should **not** be committed.*

## Feature Requests & Git Issues

We believe the community should drive the project's priorities. 

### Tackling Existing Issues
When looking for something to work on, please check the GitHub Issues tab. We highly recommend starting with issues labeled `shared-types` if you are new, as building the core data structures is the first step before implementing contract logic.

### Requesting a New Feature
1.  **Check existing requests**: Browse existing Issues to see if someone has already suggested it.
2.  **Open an Issue**: If your idea is new, open a GitHub Issue. Include:
    *   A clear, descriptive title.
    *   The problem or use case you're trying to solve in the logistics flow.
    *   Your proposed solution or approach.

## Submitting a Pull Request

1.  **Ensure all tests pass**: Run `cargo test` from the root directory and ensure 100% of tests pass.
2.  **Update documentation**: If you change the behavior of a contract, update the relevant markdown files in the `docs/` folder.
3.  **Format your code**: Ensure your Rust code is properly formatted before committing.
    ```bash
    cargo fmt --all
    ```
4.  **Submit your PR** to the `main` branch. Provide a clear description of what your changes do, which issues they resolve, and any new cross-contract dependencies introduced.

## License

By contributing, you agree that your contributions will be licensed under the **MIT License**, same as the project.