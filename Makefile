build-deps:
	cargo build --target wasm32-unknown-unknown --release \
	  -p liquidity-pool-contract
	cargo build --target wasm32-unknown-unknown --release \
	  -p vendor-registry-contract
	stellar contract optimize \
	  --wasm target/wasm32-unknown-unknown/release/liquidity_pool_contract.wasm
	stellar contract optimize \
	  --wasm target/wasm32-unknown-unknown/release/vendor_registry_contract.wasm
