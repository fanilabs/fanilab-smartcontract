default: build

all: test

test: build
	cargo test

test-verbose: build
	cargo test -- --nocapture

build:
	cargo build --target wasm32v1-none --release
	@echo "Contracts built successfully in target/wasm32v1-none/release/"

build-escrow:
	cargo build -p escrow_contract --target wasm32v1-none --release
	@echo "Escrow contract built successfully!"

build-delivery:
	cargo build -p delivery_contract --target wasm32v1-none --release
	@echo "Delivery contract built successfully!"

build-dispute:
	cargo build -p dispute_resolution_contract --target wasm32v1-none --release
	@echo "Dispute resolution contract built successfully!"

build-fleet:
	cargo build -p fleet_management_contract --target wasm32v1-none --release
	@echo "Fleet management contract built successfully!"

build-identity:
	cargo build -p identity_reputation_contract --target wasm32v1-none --release
	@echo "Identity reputation contract built successfully!"

build-settlement:
	cargo build -p settlement_contract --target wasm32v1-none --release
	@echo "Settlement contract built successfully!"

clean:
	cargo clean

format:
	cargo fmt --all

fmt:
	$(MAKE) format

lint:
	cargo clippy --all-targets --all-features -- -D warnings

audit:
	cargo audit

coverage:
	cargo tarpaulin --out Html --output-dir coverage

check:
	cargo check

.PHONY: default all test test-verbose build build-escrow build-delivery build-dispute build-fleet build-identity build-settlement clean format fmt lint audit coverage check
