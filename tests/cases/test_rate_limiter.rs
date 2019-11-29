// Copyright 2017 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use std::thread;

use rocksdb::RateLimiter;

#[test]
fn test_rate_limiter() {
    let rate_limiter = RateLimiter::new(10 * 1024 * 1024, 100 * 1000, 10);
    assert_eq!(rate_limiter.get_singleburst_bytes(), 1 * 1024 * 1024);

    rate_limiter.set_bytes_per_second(20 * 1024 * 1024);
    assert_eq!(rate_limiter.get_bytes_per_second(), 20 * 1024 * 1024);

    assert_eq!(rate_limiter.get_singleburst_bytes(), 2 * 1024 * 1024);

    let low = 0;
    let high = 1;
    let total = 2;

    assert_eq!(rate_limiter.get_total_bytes_through(total), 0);

    rate_limiter.request(1024 * 1024, low);
    assert_eq!(rate_limiter.get_total_bytes_through(low), 1024 * 1024);

    rate_limiter.request(2048 * 1024, high);
    assert_eq!(rate_limiter.get_total_bytes_through(high), 2048 * 1024);

    assert_eq!(rate_limiter.get_total_bytes_through(total), 3072 * 1024);
}

#[test]
fn test_rate_limiter_sendable() {
    let rate_limiter = RateLimiter::new(10 * 1024 * 1024, 100 * 1000, 10);

    let handle = thread::spawn(move || {
        rate_limiter.request(1024, 0);
    });

    handle.join().unwrap();
}
