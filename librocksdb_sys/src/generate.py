#!/usr/bin/env python3

import re
import sys

root = '../rocksdb/include/rocksdb'

tasks = [
    (
        'DBStatisticsTickerType',
        'statistics.h',
        re.compile(r'enum Tickers .* {'),
        re.compile(r'};\s*'),
        re.compile(r'\s*\w(_\w)*.*,'),
    ),
    (
        'DBStatisticsHistogramType',
        'statistics.h',
        re.compile(r'enum Histograms .* {'),
        re.compile(r'};\s*'),
        re.compile(r'\s*\w(_\w)*.*,'),
    ),
]

print('/// This file is generated from generate.py.')
print('/// Re-generate it if you upgrade to a new version of RocksDB.')
print('')

for task in tasks:
    begin = False
    count = 0
    for line in open(root + '/' + task[1]):
        if not begin:
            if task[2].match(line):
                begin = True
                print('#[derive(Copy, Clone, Debug, Eq, PartialEq)]')
                print('#[repr(C)]')
                print('pub enum {} {{'.format(task[0]))
            continue
        if task[3].match(line):
            print('}')
            break
        if not task[4].match(line):
            continue
        tokens = line.split(',')[0].split('=')
        if len(tokens) == 1:
            name = tokens[0].strip(' ')
            value = count
        elif len(tokens) == 2:
            name = tokens[0].strip(' ')
            value = tokens[1].strip(' ')
            count = int(value)
        else:
            sys.exit("invalid enum: " + line)
        name = ''.join([w.capitalize() for w in name.split('_')])
        count = count + 1
        print('    {} = {},'.format(name, value))
