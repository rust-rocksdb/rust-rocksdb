#!/bin/bash

# Run cargo fmt and cargo clippy only on OSX host

if [[ ${TRAVIS_OS_NAME} == "osx" ]]; then
    cargo fmt --all -- --check
    cargo clippy --all --tests -- -D warnings
fi
