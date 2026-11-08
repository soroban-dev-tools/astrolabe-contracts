.PHONY: all build test fmt fmt-check lint check clean

all: check build

build:
	cargo build --target wasm32v1-none --release

test:
	cargo test --all

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

lint:
	cargo clippy --all-targets -- -D warnings

check: fmt-check lint test build

clean:
	cargo clean
