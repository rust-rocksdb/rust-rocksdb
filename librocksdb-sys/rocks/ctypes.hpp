#pragma once

#include <iostream>

#include "rocksdb/options.h"
#include "rocksdb/write_buffer_manager.h"
#include "rocksdb/cache.h"

using std::shared_ptr;

using namespace ROCKSDB_NAMESPACE;

#ifdef __cplusplus
extern "C" {
#endif
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

struct rocksdb_options_t {
  Options rep;
};
struct rocksdb_cache_t {
  std::shared_ptr<Cache> rep;
};
/* write_buffer_manager */
struct rocksdb_write_buffer_manager_t {
  std::shared_ptr<WriteBufferManager> rep;
};

#ifdef __cplusplus
}
#endif
