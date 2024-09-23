**Revised README for `rust-rocksdb` Repository**

---

# rust-rocksdb

[![RocksDB build](https://github.com/rust-rocksdb/rust-rocksdb/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/rust-rocksdb/rust-rocksdb/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/rocksdb.svg)](https://crates.io/crates/rocksdb)
[![documentation](https://docs.rs/rocksdb/badge.svg)](https://docs.rs/rocksdb)
[![license](https://img.shields.io/crates/l/rocksdb.svg)](https://github.com/rust-rocksdb/rust-rocksdb/blob/master/LICENSE)
[![Gitter chat](https://badges.gitter.im/rust-rocksdb/gitter.svg)](https://gitter.im/rust-rocksdb/lobby)
![rust 1.70.0 required](https://img.shields.io/badge/rust-1.70.0-blue.svg?label=MSRV)

![GitHub commits (since latest release)](https://img.shields.io/github/commits-since/rust-rocksdb/rust-rocksdb/latest.svg)

## 1. Introduction

Welcome to the `rust-rocksdb` repository. This document delineates the terms, conditions, and procedural guidelines governing the utilization, contribution, and maintenance of this software package. By accessing and using this repository, contributors and users acknowledge and agree to adhere to the stipulations herein.

## 2. Requirements

To ensure proper functionality and compatibility, the following dependencies must be satisfied:

- **Clang and LLVM**: These are requisite for the compilation and linkage processes integral to `rust-rocksdb`.

## 3. Contribution Guidelines

### 3.1. General Provisions

All prospective contributors are hereby invited to submit feedback and pull requests. Contributions are subject to review and must conform to the repository’s established standards and protocols.

### 3.2. Issue Management

Pursuant to the repository’s governance policies, contributors must adhere to the following procedures when submitting issues:

- **Issue Submission**: All issues must be filed through the designated GitHub Issues interface. Each issue must be clearly articulated, specifying the nature of the problem, steps to reproduce, and any relevant contextual information.
  
- **Redundancy Prevention**: Contributors are obligated to review existing issues prior to submission to mitigate redundancy. The creation of duplicate or redundant issues is expressly discouraged as it dilutes the repository’s focus and impedes the resolution of substantive matters.

- **Self-Management of Issues**: Contributors retain the authority to manage their own issues, including the unilateral closure or deletion thereof, utilizing GitHub’s built-in functionalities. Submitting additional issues to request the closure or deletion of one’s own prior submissions is deemed superfluous and procedurally inefficient.

- **Repository Maintainer Discretion**: Repository maintainers reserve the right to summarily close or dismiss issues that are identified as redundant or duplicative without further deliberation, pursuant to the principles of procedural economy and repository hygiene.

### 3.3. Pull Request Protocol

All pull requests must undergo a rigorous review process. Contributors are expected to ensure that their submissions are comprehensive, well-documented, and align with the repository’s architectural and coding standards.

## 4. Usage

This binding is statically linked with a specific version of RocksDB. To facilitate a custom build, ensure the cloning of RocksDB and compression submodules as follows:

```shell
git submodule update --init --recursive
```

## 5. Compression Support

By default, support for [Snappy](https://github.com/google/snappy), [LZ4](https://github.com/lz4/lz4), [Zstd](https://github.com/facebook/zstd), [Zlib](https://zlib.net), and [Bzip2](http://www.bzip.org) compression is enabled through crate features. Should support for all these compression algorithms be unnecessary, default features may be disabled and specific compression algorithms enabled individually. For instance, to enable only LZ4 compression support, modify your `Cargo.toml` as follows:

```toml
[dependencies.rocksdb]
default-features = false
features = ["lz4"]
```

## 6. Multithreaded ColumnFamily Alternation

RocksDB permits the creation and deletion of column families from multiple threads concurrently. However, this crate does not enable such functionality by default to maintain compatibility. To permit concurrent modification of column families, enable the crate feature `multi-threaded-cf`, which configures the binding's data structures to utilize `RwLock` by default. Alternatively, one may directly instantiate `DBWithThreadMode<MultiThreaded>` without enabling the aforementioned crate feature.

## 7. Runtime Library Switching (Windows Only)

The feature `mt_static` mandates the library to be built with the [/MT](https://learn.microsoft.com/en-us/cpp/build/reference/md-mt-ld-use-run-time-library?view=msvc-170) flag, thereby instructing the library to employ the static version of the run-time library. This configuration is particularly advantageous in scenarios where dependency tree conflicts arise due to disparate run-time library versions.

## 8. Repository Governance and Policies

### 8.1. Issue Tracking Integrity

Pursuant to maintaining the integrity and efficiency of the issue tracking system, contributors are required to:

- **Avoid Redundancy**: Refrain from submitting issues that duplicate existing concerns unless absolutely necessary.
  
- **Self-Management**: Utilize GitHub’s native functionalities to manage personal issues without necessitating auxiliary submissions for closure or deletion.

- **Compliance with Policies**: Adhere strictly to the repository’s policies regarding issue submission, management, and closure to ensure procedural efficiency and repository hygiene.

### 8.2. Enforcement and Remediation

Repository maintainers are empowered to enforce these policies through the following mechanisms:

- **Issue Closure**: Summarily close issues identified as redundant or duplicative.
  
- **Contributor Education**: Provide guidance and reminders to contributors regarding proper issue management practices.

- **Policy Amendments**: Implement amendments to governance policies as deemed necessary to uphold repository integrity.

## 9. Legal Disclaimer

All contributions to this repository are subject to the terms outlined herein and the accompanying [LICENSE](https://github.com/rust-rocksdb/rust-rocksdb/blob/master/LICENSE). By contributing, you agree to abide by these terms and to respect the intellectual property and proprietary rights of all stakeholders involved.

## 10. Acknowledgments

The collaborative efforts of all contributors are duly acknowledged. Your commitment to maintaining high standards of procedural efficiency and repository integrity is instrumental to the continued success and advancement of `rust-rocksdb`.
