#!/usr/bin/env bash

set -e

con=1
if [ "$MAKE_PARALLELISM" ]; then
  con=$MAKE_PARALLELISM
else
  if [[ -f /proc/cpuinfo ]]; then
      con=`grep -c processor /proc/cpuinfo`
  else
      con=`sysctl -n hw.ncpu 2>/dev/null || echo 1`
  fi
fi

function error() {
    echo $@ >&2
    return 1
}

function md5_check() {
    if which md5sum &>/dev/null; then
        hash=`md5sum $1 | cut -d ' ' -f 1`
    elif which openssl &>/dev/null; then
        hash=`openssl md5 -hex $1 | cut -d ' ' -f 2`
    else
        error can\'t find hash tool.
    fi

    [[ "$hash" == "$2" ]] || error $1: hash not correct, expect $2, got $hash
}

retry=3
function download() {
    if [[ -f $2 ]] && md5_check $2 $3; then
        return
    fi

    if which curl &>/dev/null; then
        curl --retry $retry -L $1 -o $2
    elif which wget &>/dev/null; then
        wget --retry-connrefused --waitretry=1 --read-timeout=20 --timeout=15 --tries $retry $1 -O $2
    else
        error can\'t find wget and curl.
    fi

    md5_check $2 $3
}

function compile_z() {
    if [[ -f libz.a ]]; then
        return
    fi

    rm -rf zlib-1.2.11
    download https://github.com/madler/zlib/archive/v1.2.11.tar.gz zlib-1.2.11.tar.gz 0095d2d2d1f3442ce1318336637b695f
    tar xf zlib-1.2.11.tar.gz
    cd zlib-1.2.11
    CFLAGS='-fPIC' ./configure --static
    make -j $con
    cp libz.a ../
    cd ..
}

function compile_bz2() {
    if [[ -f libbz2.a ]]; then
        return
    fi

    rm -rf bzip2-1.0.6
    download http://www.bzip.org/1.0.6/bzip2-1.0.6.tar.gz bzip2-1.0.6.tar.gz 00b516f4704d4a7cb50a1d97e6e8e15b
    tar xvzf bzip2-1.0.6.tar.gz
    cd bzip2-1.0.6
    make CFLAGS='-fPIC -O2 -g -D_FILE_OFFSET_BITS=64' -j $con
    cp libbz2.a ../
    cd ..
}

function compile_snappy() {
    if [[ -f libsnappy.a ]]; then
        return
    fi

    rm -rf snappy-1.1.1
    download http://pkgs.fedoraproject.org/repo/pkgs/snappy/snappy-1.1.1.tar.gz/8887e3b7253b22a31f5486bca3cbc1c2/snappy-1.1.1.tar.gz snappy-1.1.1.tar.gz 8887e3b7253b22a31f5486bca3cbc1c2
    tar xvzf snappy-1.1.1.tar.gz
    cd snappy-1.1.1
    ./configure --with-pic --enable-static
    make -j $con
    mv .libs/libsnappy.a ../
    cd ..
}

function compile_lz4() {
    if [[ -f liblz4.a ]]; then
        return
    fi

    rm -rf lz4-r127
    download https://github.com/Cyan4973/lz4/archive/r131.tar.gz lz4-r131.tar.gz 42b09fab42331da9d3fb33bd5c560de9
    tar xvzf lz4-r131.tar.gz
    cd lz4-r131/lib
    make CFLAGS='-fPIC -O2' all -j $con
    mv liblz4.a ../../
    cd ../..
}

function compile_zstd() {
    if [[ -f libzstd.a ]]; then
        return
    fi

    rm -rf zstd-1.2.0
    download https://github.com/facebook/zstd/archive/v1.2.0.tar.gz zstd-1.2.0.tar.gz d7777b0aafa7002a4dee1e2db42afe30
    tar xvzf zstd-1.2.0.tar.gz
    cd zstd-1.2.0/lib
    make CPPFLAGS='-fPIC -I. -I./common' -j $con
    mv libzstd.a ../..
    cd ../..
}

function compile_rocksdb() {
    if [[ -f librocksdb.a ]]; then
        return
    fi

    version=v5.6.1
    vernum=5.6.1
    echo building rocksdb-$version
    rm -rf rocksdb rocksdb-$vernum
    download https://github.com/facebook/rocksdb/archive/$version.tar.gz rocksdb-$version.tar.gz 51ad34cbe2d161c3be8aa16a5ad81c3d
    tar xf rocksdb-$version.tar.gz
    wd=`pwd`
    mv rocksdb-$vernum rocksdb
    cd rocksdb
    export EXTRA_CFLAGS="-fPIC -I${wd}/zlib-1.2.11 -I${wd}/bzip2-1.0.6 -I${wd}/snappy-1.1.1 -I${wd}/lz4-r131/lib -I${wd}/zstd-1.2.0/lib"
    export EXTRA_CXXFLAGS="-DZLIB -DBZIP2 -DSNAPPY -DLZ4 -DZSTD $EXTRA_CFLAGS"
    DISABLE_JEMALLOC=1 make static_lib -j $con
    mv librocksdb.a ../
    cd ..
}

function find_library() {
    if [[ "$CXX" = "" ]]; then
        if g++ --version &>/dev/null; then
            CXX=g++
        elif clang++ --version &>/dev/null; then
            CXX=clang++
        else
            error failed to find valid cxx compiler.
        fi
    fi

    $CXX --print-file-name $1
}

if [[ $# -eq 0 ]]; then
    error $0 [compile_bz2\|compile_z\|compile_lz4\|compile_zstd\|compile_rocksdb\|compile_snappy\|find_library]
fi

$@
