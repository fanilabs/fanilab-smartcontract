default: build

all: test

test: build
	cargo test

build:
	cargo build --target wasm32-unknown-unknown --release
	@echo "Contracts built successfully in target/wasm32-unknown-unknown/release/"

build-escrow:
	cargo build -p escrow_contract --target wasm32-unknown-unknown --release
	@echo "Escrow contract built successfully!"

build-delivery:
	cargo build -p delivery_contract --target wasm32-unknown-unknown --release
	@echo "Delivery contract built successfully!"

build-dispute:
	cargo build -p dispute_resolution_contract --target wasm32-unknown-unknown --release
	@echo "Dispute resolution contract built successfully!"

clean:
	cargo clean

fmt:
	cargo fmt --all

check:
	cargo check
