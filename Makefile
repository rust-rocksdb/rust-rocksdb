all: format build test

format:
	@cargo fmt --all
	@librocksdb_sys/crocksdb/format-diff.sh > /dev/null || true

build:
	@cargo build

test:
	@export RUST_BACKTRACE=1 && cargo test -- --nocapture

clean:
	@cargo clean
	@cd librocksdb_sys && cargo clean

