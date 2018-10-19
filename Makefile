.PHONY: all format

# format rust code using the specified command for the specified file or directory.
# $(call do-format-with-cmd,cmd,file-or-dir)
define do-format-with-cmd
	$1 --write-mode diff $2 |  grep -E "Diff .*at line" > /dev/null && $1 --write-mode overwrite $2 || exit 0
endef

# format rust code in the specified file or directory
# a file of rust code follows the convention of having suffix '.rs'
# $(call format-code-in,file-or-dir)
define format-code-in
	$(if $(filter %.rs, $1),  \
	$(call do-format-with-cmd, rustfmt, $1), \
	cd $1 && $(call do-format-with-cmd, cargo fmt --))
endef

all: format build test

build:
	@cargo build 

test:
	@export RUST_BACKTRACE=1 && cargo test -- --nocapture 

format: 
	@$(call format-code-in, .)
	@$(call format-code-in, tests/test.rs)
	@$(call format-code-in, librocksdb_sys)
	# User may not install clang-format-diff.py, ignore any error here. 
	@librocksdb_sys/crocksdb/format-diff.sh > /dev/null || true 

clean:
	@cargo clean
	@cd librocksdb_sys && cargo clean

update-rocksdb:
	@git subtree pull -P librocksdb_sys/rocksdb https://github.com/pingcap/rocksdb.git release-5.15 --squash
