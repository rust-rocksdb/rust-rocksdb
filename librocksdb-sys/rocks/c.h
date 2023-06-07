// These functions are missing from upstream rocksdb/c.h
// This should be upstreamed.
#pragma once

#ifdef __cplusplus
extern "C"
{
#endif
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

    typedef struct rocksdb_options_t rocksdb_options_t;
    /* write_buffer_manager */
    typedef struct rocksdb_write_buffer_manager_t rocksdb_write_buffer_manager_t;
    /* cache */
    typedef struct rocksdb_cache_t rocksdb_cache_t;

    void rocksdb_options_set_write_buffer_manager(rocksdb_options_t *opt, rocksdb_write_buffer_manager_t *manager);

    /* write_buffer_manager */
    rocksdb_write_buffer_manager_t *rocksdb_write_buffer_manager_create(size_t buffer_size);
    rocksdb_write_buffer_manager_t *rocksdb_write_buffer_manager_create_with_cache(size_t buffer_size, rocksdb_cache_t *cache, bool allow_stall);

    void rocksdb_write_buffer_manager_destroy(rocksdb_write_buffer_manager_t *manager);

    unsigned char rocksdb_write_buffer_manager_enabled(rocksdb_write_buffer_manager_t *manager);
    size_t rocksdb_write_buffer_manager_memory_usage(rocksdb_write_buffer_manager_t *manager);
    size_t rocksdb_write_buffer_manager_buffer_size(rocksdb_write_buffer_manager_t *manager);
#ifdef __cplusplus
}
#endif