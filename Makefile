all: format build test

format:
	@cargo fmt --all
	@scripts/format-diff.sh

build:
	@cargo build

test:
	@export RUST_BACKTRACE=1 && cargo test -- --nocapture

clean:
	@cargo clean
	@cd librocksdb_sys && cargo clean

# TODO it could be worth fixing some of these lints
clippy:
	@cargo clippy --all -- \
	-D warnings \
	-A clippy::redundant_field_names -A clippy::single_match \
	-A clippy::assign_op_pattern -A clippy::new_without_default -A clippy::useless_let_if_seq \
	-A clippy::needless_return -A clippy::len_zero

update_titan:
	@if [ -n "${TITAN_REPO}" ]; then \
		git config --file=.gitmodules submodule.titan.url https://github.com/${TITAN_REPO}/titan.git; \
	fi
	@if [ -n "${TITAN_BRANCH}" ]; then \
		git config --file=.gitmodules submodule.titan.branch ${TITAN_BRANCH}; \
	fi
	@git submodule sync
	@git submodule update --init --remote librocksdb_sys/libtitan_sys/titan

update_rocksdb:
	@if [ -n "${ROCKSDB_REPO}" ]; then \
		git config --file=.gitmodules submodule.rocksdb.url https://github.com/${ROCKSDB_REPO}/rocksdb.git; \
	fi
	@if [ -n "${ROCKSDB_BRANCH}" ]; then \
		git config --file=.gitmodules submodule.rocksdb.branch ${ROCKSDB_BRANCH}; \
	fi
	@git submodule sync
	@git submodule update --init --remote librocksdb_sys/rocksdb

update_rocksdb_cloud:
	@if [ -n "${ROCKSDB_CLOUD_REPO}" ]; then \
		git config --file=.gitmodules submodule.rocksdb-cloud.url https://github.com/${ROCKSDB_CLOUD_REPO}/rocksdb.git; \
	fi
	@if [ -n "${ROCKSDB_CLOUD_BRANCH}" ]; then \
		git config --file=.gitmodules submodule.rocksdb-cloud.branch ${ROCKSDB_CLOUD_BRANCH}; \
	fi
	@git submodule sync
	@git submodule update --init --remote librocksdb_sys/librocksdb_cloud_sys/rocksdb-cloud
	