---
name: code-review
description: Review pull requests, diffs and commits in rust-rocksdb, the Rust bindings over RocksDB's C API (librocksdb-sys). Use when reviewing changes to src/, librocksdb-sys/, tests/ or CI, and especially unsafe FFI code, raw pointers, Drop impls, C callbacks, Send/Sync impls and object lifetimes. Focuses on memory leaks, double frees, use-after-free, dangling pointers, allocator mismatches, panics across FFI and other soundness bugs before style.
---

# Code review for rust-rocksdb

rust-rocksdb is a safe Rust layer over the RocksDB **C API**, which itself wraps the C++
library. Almost every public method ends in an `unsafe` call through `librocksdb-sys`, so a
review is first a review of **ownership and lifetimes across the FFI boundary**. A leak or
double free here does not show up in `cargo test`. It shows up weeks later as a slow RSS
climb or heap corruption in someone's production database.

Review in this order. Do not spend time on style until the first three are clean:

1. Memory safety and ownership (leaks, double free, use-after-free, dangling pointers)
2. Callback safety (Rust code called from C++ threads)
3. Thread safety (`Send`/`Sync`, data races)
4. FFI correctness (types, nulls, errors, strings)
5. Public API, compatibility, tests and docs

## Project map

| Path | What it is |
|---|---|
| `librocksdb-sys/` | `-sys` crate. `build.rs` compiles RocksDB C++ with `cc` and runs `bindgen` over `rocksdb/include/rocksdb/c.h`. |
| `librocksdb-sys/rocksdb/` | RocksDB git submodule (upstream `https://github.com/facebook/rocksdb`). The pinned version is the `+x.y.z` suffix of the version in `librocksdb-sys/Cargo.toml`. |
| `librocksdb-sys/rocksdb/db/c.cc` | **Implementation of the C API. This file decides who owns each pointer.** |
| `src/ffi_util.rs` | Shared FFI helpers: `ffi_try!`, `convert_rocksdb_error`, `from_cstr_and_free`, `from_cstr_without_free`, `raw_data`, `CSlice`, `to_cpath`, `CStrLike`, `opt_bytes_to_ptr`. |
| `src/db.rs`, `src/transactions/` | `DBCommon` / `TransactionDB` / `OptimisticTransactionDB`, plus their drop order. |
| `src/db_options.rs` | `Options`, `ReadOptions`, etc. `OptionsMustOutliveDB` keeps Rust-owned objects alive while C++ holds raw pointers to them. |
| `src/comparator.rs`, `merge_operator.rs`, `compaction_filter*.rs`, `slice_transform.rs`, `write_batch.rs` | `extern "C"` trampolines: C++ calls back into Rust. |
| `src/column_family.rs` | `ColumnFamily`, and an `Arc` pointer cast between `UnboundColumnFamily` and `BoundColumnFamily`. Both types must keep an identical layout and `Drop`. |
| `src/raw_ptr.rs` | `AsRawPtr`, behind the `raw-ptr` feature. |
| `tests/` | Integration tests. Use `tests/util::DBPath` for temporary DBs. |
| `tests/fail/` | `trybuild` compile-fail cases that prove the borrow checker rejects lifetime misuse. |

## Step 1: establish ownership from the source, not from the name

For **every** new or changed `ffi::rocksdb_*` call in the diff:

1. Find its declaration in `librocksdb-sys/rocksdb/include/rocksdb/c.h`.
2. Find its implementation in `librocksdb-sys/rocksdb/db/c.cc`:
   `grep -n "rocksdb_<name>(" librocksdb-sys/rocksdb/db/c.cc`.
3. For each pointer argument and for the return value, write down:
   - **Owned by the caller.** The caller must free it exactly once, with the matching function.
   - **Borrowed.** Valid only while some other object lives. The caller must not free it.
   - **Ownership transferred into C++.** Usually wrapped in a `std::shared_ptr` or passed to
     `.reset()`. The caller must never free it again.
   - **Stored as a raw pointer.** C++ keeps the pointer without owning it. The caller must
     keep the object alive for as long as C++ may use it.

If the submodule is not checked out, read the file upstream at the tag that matches the pinned
version. If you still cannot determine ownership, **say so in the review** and ask the author.
Do not guess.

### Ownership cheat sheet (verified against `c.cc`; re-check after a submodule bump)

| C API pattern | Who frees | How |
|---|---|---|
| `char** errptr` out-param | caller | Use `ffi_try!`, which calls `convert_rocksdb_error`. That frees the string, so the pointer must not be used again. |
| `char*` returned from `rocksdb_get*`, `rocksdb_*property_value*`, `rocksdb_get_db_identity`, `rocksdb_transaction_get*`, etc. (malloc'd) | caller | `ffi::rocksdb_free`, via `from_cstr_and_free`, `CSlice::from_raw_parts` or `raw_data` followed by `rocksdb_free`. |
| Values and errors from `rocksdb_multi_get*` | caller, **per element** | Every non-null value and every non-null error (see `convert_values` in `src/db.rs`). |
| Accessors into a container: `rocksdb_iter_key/value/timestamp`, `rocksdb_pinnableslice_value`, `rocksdb_livefiles_name`, `rocksdb_livefiles_*key`, `rocksdb_writebatch_data` | **nobody** (borrowed) | Copy with `from_cstr_without_free` or `raw_data`, or return a slice whose lifetime is tied to the container. Never free. |
| Arrays and aggregate results: `rocksdb_list_column_families`, `rocksdb_livefiles`, `rocksdb_get_column_family_metadata*` | caller | Their dedicated `*_destroy` function, after all elements have been copied. |
| Objects from `*_create*`, `*_open*`, `rocksdb_create_column_family*` | caller | The matching `*_destroy`, `*_close` or `rocksdb_column_family_handle_destroy`, **exactly once**. |
| `rocksdb_options_set_merge_operator`, `_set_compaction_filter_factory`, `_set_prefix_extractor` | **C++** (`shared_ptr` / `reset`) | Rust must not destroy the object afterwards and must not hand the same pointer to a second `Options`. |
| `rocksdb_options_set_comparator`, `_set_compaction_filter`, `_set_wal_filter` | **Rust** (stored raw) | Keep it alive as long as any `Options` copy or DB uses it: store it in `OptionsMustOutliveDB` (see `OwnedComparator` and `OwnedCompactionFilter`). |
| `rocksdb_options_set_env` | Rust (stores the raw `Env*`) | The `Env` must outlive the options and the DB; it is tracked in `outlive.env`. |
| `rocksdb_options_set_row_cache`, `rocksdb_block_based_options_set_block_cache` | shared (copies `shared_ptr`) | The crate still tracks these in `outlive`. Keep new code consistent with that. |
| `rocksdb_readoptions_set_iterate_{lower,upper}_bound`, `_set_timestamp`, `_set_iter_start_ts` | Rust (**stores the pointer** to your bytes) | `ReadOptions` must own the `Vec<u8>`. The iterator must own the `ReadOptions` (issue #660). Never mutate or reallocate that `Vec` after passing its pointer. |

For any **new** setter that takes `const char*` + `size_t`, check whether `c.cc` copies the data
into a `std::string` or keeps a `Slice` or a raw pointer. If it keeps the pointer, the Rust side
must own the buffer for the C object's whole lifetime.

## Step 2: memory leaks (highest priority)

Check each item below and report every violation.

- **Every owned pointer reaches an RAII owner or an explicit free on every path.** That includes
  early returns through `?`, through `ffi_try!` (which **expands to `return Err(..)`**), and
  through panics (`unwrap`, `expect`, `assert!`, indexing, user closures).
  - Correct: wrap the pointer in its owning struct (`NonNull<..>` + `Drop`) on the next line
    after creation, before any other fallible call.
  - Bug: allocate A, call `ffi_try!(...)` for B, and only then wrap A. If B fails, A leaks.
- **Loops over C arrays.** If the conversion of element *i* can fail or panic, elements after
  *i* and the array itself leak. Copy everything first, call `*_destroy`, then handle errors.
  Alternatively use a guard.
- **`Box::into_raw` handed to C** (callback state, merge results, filters). Check all three:
  1. A destructor callback exists that calls `Box::from_raw` with the **exact same type**,
     including generic parameters (`destructor_callback::<F, PF>` must match the
     `MergeOperatorCallback<F, PF>` that was boxed).
  2. The C object that owns the state is itself destroyed, or its ownership is transferred to
     something that destroys it.
  3. If the C `*_create` can fail or return null, the box is reclaimed instead of leaked.
- **`mem::forget`, `ManuallyDrop`, `Box::leak`, `into_raw` and `Arc::into_raw`** need a
  `// SAFETY:` or ownership comment that names who frees the value and when. Without one,
  treat it as a leak.
- **`SliceTransform` intentionally has no `Drop`.** Ownership moves into
  `set_prefix_extractor`. A new API that accepts a `SliceTransform` without transferring it
  leaks it. One that transfers the same `SliceTransform` twice double frees it.
- **`Clone` / `Copy` on a type that owns a raw pointer is a double free.** `#[derive(Clone)]`
  copies the pointer. Clone either through a C copy function (as `Options::clone` does with
  `rocksdb_options_create_copy`, which also clones `outlive`) or share through `Arc`. See the
  warning on `CSlice` in `src/ffi_util.rs`.
- **Ignored error out-params.** Passing `&mut err` and never checking it leaks the error string
  and also loses the error. Always use `ffi_try!`.
- **Transaction state.** `TransactionDB` tracks prepared 2PC transactions in `prepared` and
  clears them in `Drop`. New transaction paths must keep that bookkeeping balanced.

## Step 3: use-after-free, dangling pointers and lifetimes

- **Pointers to temporaries.** `let p = CString::new(x)?.as_ptr();` dangles because the
  `CString` is dropped at the end of the `let` statement. Calling `.as_ptr()` on a temporary is
  fine **only** inside the argument list of the FFI call in the same statement, as in
  `to_cpath(p)?.as_ptr()` in `src/db.rs`. The same applies to `Vec`, `String` and
  `to_vec()` temporaries.
- **Borrowed C memory needs a Rust lifetime.** A slice from `rocksdb_iter_key/value`,
  `rocksdb_pinnableslice_value` and similar accessors must borrow `&self`. Every method that
  can invalidate that memory (`next`, `prev`, `seek*`, `refresh`, `set_*`) must take
  `&mut self`.
- **Child objects must not outlive their parent.** Iterators, snapshots, `DBPinnableSlice`,
  transactions, checkpoints, `SstFileWriter<'a>`, `MemoryUsageBuilder<'a>` and
  `BoundColumnFamily<'a>` carry `PhantomData<&'a Parent>`. A new child type needs:
  - the lifetime parameter wired the same way;
  - a compile-fail case in `tests/fail/<name>.rs` with the expected `<name>.stderr`.
- **A raw pointer stored in a struct without a lifetime** (for example in an options struct)
  lets the referenced object die first. Flag it, and ask for one of: a lifetime, ownership
  (an `Arc` or an owned copy), or `OptionsMustOutliveDB`.
- **Drop order is declaration order.** Fields drop top to bottom, after `Drop::drop` has run.
  - `DBCommon { inner, cfs, path, _outlive }`: `Drop` first destroys the column family handles
    (`drop_all_cfs_internal`). Then `inner` closes the DB, and only after that are the
    comparator, filter, cache and env in `_outlive` freed.
  - Moving `_outlive` above `inner`, or destroying CF handles after the DB is closed, is a
    use-after-free. Treat any field reordering in these structs as a blocker until proven
    safe. The same applies to `TransactionDB` and `OptimisticTransactionDBInner`.
- **Pointer casts and `transmute`.** `UnboundColumnFamily::bound_column_family` casts an
  `Arc` between the two types. Any change to either struct's fields or `Drop` must be mirrored
  in the other. Reject new `transmute`s when a safe alternative or `cast()` with a documented
  layout argument exists.
- **`slice::from_raw_parts(ptr, len)` with a null `ptr` is UB, even when `len == 0`.** Check
  whether C can pass null; for example, `existing_value` in the merge operator is null when
  there is no value. Guard it, or use `raw_data`, which handles null.

## Step 4: callbacks (C++ calling into Rust)

Trampolines are `unsafe extern "C" fn`s that receive a `*mut c_void` state pointer.

- **Type identity.** The cast `raw as *mut T` must name exactly the `T` that was boxed. With
  generics, check that every `Some(callback::<..>)` passed to `*_create` uses the same type
  arguments.
- **Panics abort the process.** Since Rust 1.81 a panic that unwinds out of an `extern "C"`
  function aborts the process. User closures for the comparator, merge operator, compaction
  filter, logger, slice transform and write batch iterator can all panic.
  - For new callbacks, prefer `std::panic::catch_unwind` and map a panic to the failure value
    the C API supports (the merge operator can set `*success = 0`).
  - Otherwise, document that a panic aborts the process.
  - Do not "fix" this with `extern "C-unwind"`: RocksDB's C++ frames are not unwind-safe for
    Rust panics.
- **Concurrency.** Callbacks run on RocksDB background threads (compaction, flush), often
  several at once.
  - The closure must be `Send + Sync + 'static` (as `MergeFn` is).
  - The trampoline must create `&*` (shared), **not** `&mut *`, when it can be called
    concurrently. `FnMut` state shared across threads is a data race.
  - Exception: a compaction filter created per compaction by a factory is used from only one
    thread at a time.
- **Allocator pairing for buffers returned to C++:**
  - Merge results come back from Rust as `Box<[u8]>`, and `merge_operator::delete_callback`
    frees them with Rust's allocator. If `delete_value` were `None`, `c.cc` would call `free()`
    on Rust-allocated memory. That is an allocator mismatch and corrupts the heap under
    jemalloc, mimalloc, a custom `#[global_allocator]`, or the Windows CRT.
  - Compaction filter `Decision::Change(&'static [u8])`: `c.cc` copies the value and never
    frees the pointer. Returning a pointer into a temporary `Vec` dangles; returning
    `Box::into_raw` leaks on every call.
  - Slice transform: `transform_fn` must return a sub-slice of its input (or `'static` data),
    never a fresh allocation.
- **`name_callback`** returns a pointer to a `CString` field of the state. It must stay valid
  for the whole life of the C object; never return a pointer to a temporary.
- **Stack state.** Synchronous APIs (`WriteBatch::iterate`) may pass `&mut T` from the stack.
  A callback that C++ keeps after the call returns must not capture stack pointers.

## Step 5: memory allocators

Memory must be freed by the allocator that allocated it.

- Memory allocated by RocksDB is freed with **`ffi::rocksdb_free`**. Never use `libc::free`,
  `CString::from_raw`, `Box::from_raw` or `Vec::from_raw_parts` on it. With the `jemalloc`
  feature RocksDB may use unprefixed jemalloc; with `mt_static` on Windows it may use a
  different CRT heap.
- Memory allocated by Rust (`Box`, `Vec`, `CString`) is freed only by Rust.
- Changes to the jemalloc wiring in `librocksdb-sys/build.rs` (`ROCKSDB_JEMALLOC`,
  `JEMALLOC_NO_DEMANGLE`, the list of targets without unprefixed malloc) change which
  allocator backs RocksDB's `malloc`. Review them against every target in CI (Linux,
  macOS, Windows, musl, i686, armv7).

## Step 6: thread safety

- Every `unsafe impl Send` / `unsafe impl Sync` needs a `// SAFETY:` comment that cites
  RocksDB's thread-safety guarantee for the wrapped C++ object. For example, `DB` is
  thread-safe, a `WriteBatch` is not safe for concurrent mutation, and an `Iterator` must not
  be used from several threads at once.
- `Sync` is acceptable only if every `&self` method is safe to call concurrently. Methods that
  mutate C++ state must take `&mut self`.
- When a type's auto traits change, update the compile-time `is_send` / `is_sync` lists in
  `src/lib.rs`.
- Do not hold the column-family map lock (`RwLock` in `MultiThreaded` mode) across a RocksDB
  call that can call back into Rust code that takes the same lock.
- Code that touches column families must compile and be correct both with and without the
  `multi-threaded-cf` feature, because `ColumnFamilyRef` changes type. CI runs both.

## Step 7: FFI correctness

- **Errors.** Any C function with a `char** errptr` goes through `ffi_try!`. Remember that the
  macro returns from the enclosing function or closure.
- **Integer casts.** The crate allows `clippy::cast_possible_truncation`, `cast_sign_loss` and
  `cast_possible_wrap`, so check every `as` by hand:
  - a negative `c_int` cast to `usize` (see the assert in `MergeOperands::new`);
  - `u64` / `usize` cast to `c_int`;
  - lengths cast to `u32`.
- **Booleans** go out as `c_uchar::from(b)` and come back as `x != 0`. Never `transmute` them.
- **Enums from C ints** need a fallible mapping with an explicit unknown arm. Never transmute
  an int into a Rust enum.
- **Nulls.**
  - Wrap creators that must not fail in `NonNull::new(..)`. Prefer returning `Err` over
    `unwrap()` where the C API can legitimately return null.
  - Treat a null return from getters as `None`.
- **Strings in.** Names go through `CStrLike` / `into_c_string()`; paths through `to_cpath`,
  which preserves non-UTF-8 bytes on Unix. Existing setters `unwrap()` on an interior NUL. New
  public APIs that take user-controlled strings should return `Result` instead of panicking.
- **Strings out.** Be consistent with neighbouring code: `from_utf8_lossy` for names and paths,
  `to_str` + `Err` when the value is parsed.
- **Out-params** use `&raw mut x` (the crate's style on Rust 1.89).
- **Availability.** Each new `ffi::` symbol must exist in the pinned RocksDB version's `c.h`.
  bindgen regenerates the bindings from that header at build time, so the PR must not depend
  on functions that exist only in a newer RocksDB without also bumping the submodule.

## Step 8: API, compatibility, tests, docs

- **MSRV and edition.** MSRV is in `rust-toolchain.toml` and `rust-version`; the edition is
  2021. Do not use newer std APIs or language features.
- **Breaking changes.** Changes to public signatures, trait bounds, auto traits or feature
  semantics need a `CHANGELOG.md` entry and a mention in the PR description.
- **Behaviour.** New public methods mirror RocksDB naming and semantics. The doc comment
  states which RocksDB option or function it wraps, and links the upstream wiki or header
  where helpful.
- **Doc examples** must compile and run (`cargo test --doc`). They create the DB in a
  `tempfile` dir and call `DB::destroy`.
- **Tests:**
  - Behaviour changes come with a test in `tests/` (use `DBPath` from `tests/util`).
  - New lifetime-bound types come with a `tests/fail/` case.
  - Changes to memory-sensitive paths get a test that exercises drop and the error paths,
    not only the happy path. `test_*_memory_usage.rs` shows how to observe memory.
- **CI gates already enforced** (do not repeat them as review comments; only point out when a
  change obviously breaks them): `cargo fmt --check`, `cargo clippy --all-targets` with
  `-D warnings` and most features (`#![warn(clippy::pedantic)]` with the allowances in
  `src/lib.rs`), `cargo rustdoc -- -D warnings`, `cargo nextest run --all` with and without
  `multi-threaded-cf`, and a jemalloc run.

### `librocksdb-sys` and build changes

- **Submodule bump:**
  - Diff `include/rocksdb/c.h` between the old and new versions and look for removed
    functions, changed signatures and changed ownership in `db/c.cc`.
  - Update `rocksdb_lib_sources.txt`, `build_version.cc` if needed, and the `+x.y.z` version
    suffix.
- **`build.rs`.** Defines must stay in sync with RocksDB's own build (`CMakeLists.txt` /
  `Makefile`); a missing define can silently change struct layouts or features. Watch
  cross-compilation targets and `bindgen` flags (`ctypes_prefix("libc")`,
  `size_t_is_usize(true)`).

## Red-flag patterns to grep for in the diff

```
Box::into_raw   Box::from_raw   Arc::into_raw   Arc::from_raw   mem::forget   ManuallyDrop
Box::leak       transmute       from_raw_parts  .as_ptr()       &mut *(       unsafe impl
extern "C"      rocksdb_free    _destroy(       _create         null_mut()    #[derive(Clone
ffi_try!        CString::from_raw   libc::free
```

Each hit is not necessarily a bug, but each one needs the ownership question from Step 1
answered.

## Reference patterns (what correct code looks like here)

Owning wrapper:

```rust
pub struct Foo {
    inner: NonNull<ffi::rocksdb_foo_t>,
}

impl Foo {
    pub fn new() -> Self {
        // Wrap immediately; nothing fallible between create and the RAII owner.
        let inner = NonNull::new(unsafe { ffi::rocksdb_foo_create() }).expect("rocksdb_foo_create");
        Self { inner }
    }
}

impl Drop for Foo {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_foo_destroy(self.inner.as_ptr()) }
    }
}

// SAFETY: rocksdb::Foo is immutable after construction and safe to share between threads.
unsafe impl Send for Foo {}
unsafe impl Sync for Foo {}
```

Malloc'd result plus error out-param:

```rust
let mut len: size_t = 0;
let ptr = unsafe { ffi_try!(ffi::rocksdb_get_something(self.inner(), &raw mut len)) };
if ptr.is_null() {
    return Ok(None);
}
// CSlice frees with rocksdb_free on drop, including on panic.
Ok(Some(unsafe { CSlice::from_raw_parts(ptr, len) }))
```

Callback state handed to C++:

```rust
let state = Box::into_raw(Box::new(MyCallback { name, f })).cast::<c_void>();
let obj = unsafe {
    ffi::rocksdb_thing_create(
        state,
        Some(destructor::<F>), // Box::from_raw(raw as *mut MyCallback<F>)
        Some(invoke::<F>),     // &*(raw as *const MyCallback<F>), plus catch_unwind
        Some(name::<F>),
    )
};
// Then either transfer `obj` to a setter that takes ownership (shared_ptr), or wrap it in an
// owning type stored in OptionsMustOutliveDB if the setter only stores the raw pointer.
```

## Known sharp edges in the existing code (do not copy these patterns)

When a PR extends one of these, ask for the safe pattern instead. These were accurate when this
skill was written; verify that they still exist before citing one.

- `ReadOptions::set_snapshot` stores the raw snapshot pointer, but `ReadOptions` has no
  lifetime. Dropping the snapshot first leaves a dangling pointer.
- `Options::set_compaction_filter` accepts an `FnMut` and the trampoline uses `&mut *`, while
  RocksDB may call a single (non-factory) filter from several compaction threads at once.
- `CompareFn` (`Box<dyn Fn(..)>`) has no `Send + Sync` bound, and the comparator trampolines
  build `&mut Self`, even though comparators run on background threads.
- Callback names are converted with `into_c_string().unwrap()`, which panics on an interior
  NUL.

## Writing the review

For each finding give:

- **Severity:**
  - **blocker**: UB, use-after-free, double free, data race, allocator mismatch, or a leak on
    a hot or common path.
  - **major**: a leak on an error path, a panic across FFI, or a missing lifetime or
    compile-fail test for a new child type.
  - **minor**: docs, naming, missing CHANGELOG entry or test gaps that are not about memory.
  - **nit**: optional polish.
- **Location:** `path/to/file.rs:LINE`.
- **What is wrong**, in one sentence.
- **Failure scenario.** A concrete sequence of calls or inputs and its consequence, for example:
  "`Foo::open` succeeds, then `ffi_try!` on `rocksdb_bar` returns `Err` → the
  `rocksdb_foo_t*` from line 120 is never destroyed → leaks 1 object per failed open".
- **Fix**, ideally as a short code suggestion that uses the crate's existing helpers.

Rules:

- Cite the `c.cc` line or behaviour that establishes ownership whenever a finding depends on it.
- Do not report formatting or anything clippy with `-D warnings` already rejects.
- Do not report speculative issues without a failure scenario. Put them under
  "Questions for the author".
- If the diff is clean on Steps 1–7, say so explicitly ("No ownership, lifetime or callback
  issues found"), so the reader knows those checks were done.
