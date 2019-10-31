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

update_titan:
	@if [ -n "${TITAN_REPO}" ]; then \
		git config --file=.gitmodules submodule.titan.url https://github.com/${TITAN_REPO}/titan.git; \
	fi
	@if [ -n "${TITAN_BRANCH}" ]; then \
		git config --file=.gitmodules submodule.titan.branch ${TITAN_BRANCH}; \
	fi
	@git submodule sync
	@git submodule update --init --remote librocksdb_sys/libtitan_sys/titan