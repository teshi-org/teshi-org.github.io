.PHONY: install dev build build-wasm clean

install:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli --version 0.2.126 --locked
	npm ci

dev: build-wasm
	npm run dev

build-wasm:
	./scripts/build-wasm.sh

build:
	./scripts/build-wasm.sh --release
	npm run build:web

clean:
	cargo clean
	rm -rf dist web/src/wasm
