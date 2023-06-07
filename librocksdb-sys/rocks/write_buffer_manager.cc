#include "rocks/ctypes.hpp"

using namespace ROCKSDB_NAMESPACE;

using std::shared_ptr;

extern "C" {
void rocksdb_options_set_write_buffer_manager(rocksdb_options_t* opt, rocksdb_write_buffer_manager_t* manager) {
  opt->rep.write_buffer_manager = manager->rep;
}
rocksdb_write_buffer_manager_t* rocksdb_write_buffer_manager_create(size_t buffer_size) {
  auto manager = new rocksdb_write_buffer_manager_t;
  manager->rep.reset(new WriteBufferManager(buffer_size));
  return manager;
}

rocksdb_write_buffer_manager_t* rocksdb_write_buffer_manager_create_with_cache(size_t buffer_size, rocksdb_cache_t* cache,bool allow_stall){
  auto manager = new rocksdb_write_buffer_manager_t;
  manager->rep.reset(new WriteBufferManager(buffer_size, cache->rep, allow_stall));
  return manager;
}

void rocksdb_write_buffer_manager_destroy(rocksdb_write_buffer_manager_t* manager) { delete manager; }

unsigned char rocksdb_write_buffer_manager_enabled(rocksdb_write_buffer_manager_t* manager) {
  return manager->rep->enabled();
}

size_t rocksdb_write_buffer_manager_memory_usage(rocksdb_write_buffer_manager_t* manager) {
  return manager->rep->memory_usage();
}

size_t rocksdb_write_buffer_manager_buffer_size(rocksdb_write_buffer_manager_t* manager) {
  return manager->rep->buffer_size();
}
}
