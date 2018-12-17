// Copyright 2018 PingCAP, Inc.
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

pub fn is_power_of_two(mut n: usize) -> bool {
    if n == 0 {
        return false;
    }
    while n % 2 == 0 {
        n = n / 2;
    }
    n == 1
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_power_of_two() {
        assert_eq!(is_power_of_two(0), false);
        assert_eq!(is_power_of_two(1), true);
        assert_eq!(is_power_of_two(2), true);
        assert_eq!(is_power_of_two(4), true);
        assert_eq!(is_power_of_two(8), true);
        assert_eq!(is_power_of_two(5), false);
    }
}
