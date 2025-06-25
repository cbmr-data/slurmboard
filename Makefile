.PHONY: debug release static

debug:
	cargo build

release:
	cargo build --release

static-debug:
# Requires `rustup target add x86_64-unknown-linux-musl`
	cargo build --target=x86_64-unknown-linux-musl

static-release:
# Requires `rustup target add x86_64-unknown-linux-musl`
	cargo build --target=x86_64-unknown-linux-musl --release
