#!/bin/bash

git diff `git merge-base master HEAD` ./librocksdb_sys/crocksdb | clang-format-diff -style=google -p1 -i
