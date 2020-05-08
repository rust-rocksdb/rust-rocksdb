/*  Copyright (c) 2011-present, Facebook, Inc.  All rights reserved.
  This source code is licensed under the BSD-style license found in the
  LICENSE file in the root directory of this source tree. An additional grant
  of patent rights can be found in the PATENTS file in the same directory.
 Copyright (c) 2011 The LevelDB Authors. All rights reserved.
  Use of this source code is governed by a BSD-style license that can be
  found in the LICENSE file. See the AUTHORS file for names of contributors.

  C bindings for rocksdb.  May be useful as a stable ABI that can be
  used by programs that keep rocksdb in a shared library, or for
  a JNI api.

  Does not support:
  . getters for the option types
  . custom comparators that implement key shortening
  . capturing post-write-snapshot
  . custom iter, db, env, cache implementations using just the C bindings

  Some conventions:

  (1) We expose just opaque struct pointers and functions to clients.
  This allows us to change internal representations without having to
  recompile clients.

  (2) For simplicity, there is no equivalent to the Slice type.  Instead,
  the caller has to pass the pointer and length as separate
  arguments.

  (3) Errors are represented by a null-terminated c string.  NULL
  means no error.  All operations that can raise an error are passed
  a "char** errptr" as the last argument.  One of the following must
  be true on entry:
     *errptr == NULL
     *errptr points to a malloc()ed null-terminated error message
  On success, a leveldb routine leaves *errptr unchanged.
  On failure, leveldb frees the old value of *errptr and
  set *errptr to a malloc()ed error message.

  (4) Bools have the type unsigned char (0 == false; rest == true)

  (5) All of the pointer arguments must be non-NULL.
*/

#ifndef C_ROCKSDB_INCLUDE_CWRAPPER_H_
#define C_ROCKSDB_INCLUDE_CWRAPPER_H_

#pragma once

#ifdef _WIN32
#ifdef C_ROCKSDB_DLL
#ifdef C_ROCKSDB_LIBRARY_EXPORTS
#define C_ROCKSDB_LIBRARY_API __declspec(dllexport)
#else
#define C_ROCKSDB_LIBRARY_API __declspec(dllimport)
#endif
#else
#define C_ROCKSDB_LIBRARY_API
#endif
#else
#define C_ROCKSDB_LIBRARY_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

/* Exported types */

typedef struct crocksdb_t                 crocksdb_t;
typedef struct crocksdb_status_ptr_t      crocksdb_status_ptr_t;
typedef struct crocksdb_backup_engine_t   crocksdb_backup_engine_t;
typedef struct crocksdb_backup_engine_info_t   crocksdb_backup_engine_info_t;
typedef struct crocksdb_restore_options_t crocksdb_restore_options_t;
typedef struct crocksdb_lru_cache_options_t crocksdb_lru_cache_options_t;
typedef struct crocksdb_cache_t           crocksdb_cache_t;
typedef struct crocksdb_memory_allocator_t crocksdb_memory_allocator_t;
typedef struct crocksdb_compactionfilter_t crocksdb_compactionfilter_t;
typedef struct crocksdb_compactionfiltercontext_t
    crocksdb_compactionfiltercontext_t;
typedef struct crocksdb_compactionfilterfactory_t
    crocksdb_compactionfilterfactory_t;
typedef struct crocksdb_comparator_t      crocksdb_comparator_t;
typedef struct crocksdb_env_t             crocksdb_env_t;
typedef struct crocksdb_fifo_compaction_options_t crocksdb_fifo_compaction_options_t;
typedef struct crocksdb_filelock_t        crocksdb_filelock_t;
typedef struct crocksdb_filterpolicy_t    crocksdb_filterpolicy_t;
typedef struct crocksdb_flushoptions_t    crocksdb_flushoptions_t;
typedef struct crocksdb_iterator_t        crocksdb_iterator_t;
typedef struct crocksdb_logger_t          crocksdb_logger_t;
typedef struct crocksdb_logger_impl_t crocksdb_logger_impl_t;
typedef struct crocksdb_mergeoperator_t   crocksdb_mergeoperator_t;
typedef struct crocksdb_options_t         crocksdb_options_t;
typedef struct crocksdb_column_family_descriptor
    crocksdb_column_family_descriptor;
typedef struct crocksdb_compactoptions_t crocksdb_compactoptions_t;
typedef struct crocksdb_block_based_table_options_t
    crocksdb_block_based_table_options_t;
typedef struct crocksdb_cuckoo_table_options_t
    crocksdb_cuckoo_table_options_t;
typedef struct crocksdb_randomfile_t      crocksdb_randomfile_t;
typedef struct crocksdb_readoptions_t     crocksdb_readoptions_t;
typedef struct crocksdb_seqfile_t         crocksdb_seqfile_t;
typedef struct crocksdb_slicetransform_t  crocksdb_slicetransform_t;
typedef struct crocksdb_snapshot_t        crocksdb_snapshot_t;
typedef struct crocksdb_writablefile_t    crocksdb_writablefile_t;
typedef struct crocksdb_writebatch_t      crocksdb_writebatch_t;
typedef struct crocksdb_writeoptions_t    crocksdb_writeoptions_t;
typedef struct crocksdb_universal_compaction_options_t crocksdb_universal_compaction_options_t;
typedef struct crocksdb_livefiles_t     crocksdb_livefiles_t;
typedef struct crocksdb_column_family_handle_t crocksdb_column_family_handle_t;
typedef struct crocksdb_envoptions_t      crocksdb_envoptions_t;
typedef struct crocksdb_sequential_file_t crocksdb_sequential_file_t;
typedef struct crocksdb_ingestexternalfileoptions_t crocksdb_ingestexternalfileoptions_t;
typedef struct crocksdb_sstfilereader_t   crocksdb_sstfilereader_t;
typedef struct crocksdb_sstfilewriter_t   crocksdb_sstfilewriter_t;
typedef struct crocksdb_externalsstfileinfo_t   crocksdb_externalsstfileinfo_t;
typedef struct crocksdb_ratelimiter_t     crocksdb_ratelimiter_t;
typedef struct crocksdb_pinnableslice_t   crocksdb_pinnableslice_t;
typedef struct crocksdb_user_collected_properties_t
    crocksdb_user_collected_properties_t;
typedef struct crocksdb_user_collected_properties_iterator_t
    crocksdb_user_collected_properties_iterator_t;
typedef struct crocksdb_table_properties_t crocksdb_table_properties_t;
typedef struct crocksdb_table_properties_collection_t
    crocksdb_table_properties_collection_t;
typedef struct crocksdb_table_properties_collection_iterator_t
    crocksdb_table_properties_collection_iterator_t;
typedef struct crocksdb_table_properties_collector_t
    crocksdb_table_properties_collector_t;
typedef struct crocksdb_table_properties_collector_factory_t
    crocksdb_table_properties_collector_factory_t;
typedef struct crocksdb_flushjobinfo_t crocksdb_flushjobinfo_t;
typedef struct crocksdb_compactionjobinfo_t crocksdb_compactionjobinfo_t;
typedef struct crocksdb_externalfileingestioninfo_t
    crocksdb_externalfileingestioninfo_t;
typedef struct crocksdb_eventlistener_t crocksdb_eventlistener_t;
typedef struct crocksdb_keyversions_t crocksdb_keyversions_t;
typedef struct crocksdb_column_family_meta_data_t crocksdb_column_family_meta_data_t;
typedef struct crocksdb_level_meta_data_t crocksdb_level_meta_data_t;
typedef struct crocksdb_sst_file_meta_data_t crocksdb_sst_file_meta_data_t;
typedef struct crocksdb_compaction_options_t crocksdb_compaction_options_t;
typedef struct crocksdb_perf_context_t crocksdb_perf_context_t;
typedef struct crocksdb_iostats_context_t crocksdb_iostats_context_t;
typedef struct crocksdb_writestallinfo_t crocksdb_writestallinfo_t;
typedef struct crocksdb_writestallcondition_t crocksdb_writestallcondition_t;
typedef struct crocksdb_map_property_t crocksdb_map_property_t;

typedef enum crocksdb_table_property_t {
  kDataSize = 1,
  kIndexSize = 2,
  kFilterSize = 3,
  kRawKeySize = 4,
  kRawValueSize = 5,
  kNumDataBlocks = 6,
  kNumEntries = 7,
  kFormatVersion = 8,
  kFixedKeyLen = 9,
  kColumnFamilyID = 10,
  kColumnFamilyName = 11,
  kFilterPolicyName = 12,
  kComparatorName = 13,
  kMergeOperatorName = 14,
  kPrefixExtractorName = 15,
  kPropertyCollectorsNames = 16,
  kCompressionName = 17,
} crocksdb_table_property_t;

typedef enum crocksdb_ratelimiter_mode_t {
  kReadsOnly = 1,
  kWritesOnly = 2,
  kAllIo = 3,
} crocksdb_ratelimiter_mode_t;

typedef enum crocksdb_backgrounderrorreason_t {
  kFlush = 1,
  kCompaction = 2,
  kWriteCallback = 3,
  kMemTable = 4,
} crocksdb_backgrounderrorreason_t;

#ifdef OPENSSL
typedef enum crocksdb_encryption_method_t {
  kUnknown = 0,
  kPlaintext = 1,
  kAES128_CTR = 2,
  kAES192_CTR = 3,
  kAES256_CTR = 4,
} crocksdb_encryption_method_t;

typedef struct crocksdb_file_encryption_info_t crocksdb_file_encryption_info_t;
typedef struct crocksdb_encryption_key_manager_t
    crocksdb_encryption_key_manager_t;
#endif

/* DB operations */

extern C_ROCKSDB_LIBRARY_API crocksdb_t* crocksdb_open(
    const crocksdb_options_t* options, const char* name, char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_t* crocksdb_open_with_ttl(
    const crocksdb_options_t* options, const char* name, int ttl, char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_t* crocksdb_open_for_read_only(
    const crocksdb_options_t* options, const char* name,
    unsigned char error_if_log_file_exist, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_status_ptr_get_error(
    crocksdb_status_ptr_t*, char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_backup_engine_t* crocksdb_backup_engine_open(
    const crocksdb_options_t* options, const char* path, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_backup_engine_create_new_backup(
    crocksdb_backup_engine_t* be, crocksdb_t* db, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_backup_engine_purge_old_backups(
    crocksdb_backup_engine_t* be, uint32_t num_backups_to_keep, char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_restore_options_t*
crocksdb_restore_options_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_restore_options_destroy(
    crocksdb_restore_options_t* opt);
extern C_ROCKSDB_LIBRARY_API void crocksdb_restore_options_set_keep_log_files(
    crocksdb_restore_options_t* opt, int v);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_backup_engine_restore_db_from_latest_backup(
    crocksdb_backup_engine_t* be, const char* db_dir, const char* wal_dir,
    const crocksdb_restore_options_t* restore_options, char** errptr);

extern C_ROCKSDB_LIBRARY_API const crocksdb_backup_engine_info_t*
crocksdb_backup_engine_get_backup_info(crocksdb_backup_engine_t* be);

extern C_ROCKSDB_LIBRARY_API int crocksdb_backup_engine_info_count(
    const crocksdb_backup_engine_info_t* info);

extern C_ROCKSDB_LIBRARY_API int64_t
crocksdb_backup_engine_info_timestamp(const crocksdb_backup_engine_info_t* info,
                                     int index);

extern C_ROCKSDB_LIBRARY_API uint32_t
crocksdb_backup_engine_info_backup_id(const crocksdb_backup_engine_info_t* info,
                                     int index);

extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_backup_engine_info_size(const crocksdb_backup_engine_info_t* info,
                                int index);

extern C_ROCKSDB_LIBRARY_API uint32_t crocksdb_backup_engine_info_number_files(
    const crocksdb_backup_engine_info_t* info, int index);

extern C_ROCKSDB_LIBRARY_API void crocksdb_backup_engine_info_destroy(
    const crocksdb_backup_engine_info_t* info);

extern C_ROCKSDB_LIBRARY_API void crocksdb_backup_engine_close(
    crocksdb_backup_engine_t* be);

extern C_ROCKSDB_LIBRARY_API crocksdb_t* crocksdb_open_column_families(
    const crocksdb_options_t* options, const char* name, int num_column_families,
    const char** column_family_names,
    const crocksdb_options_t** column_family_options,
    crocksdb_column_family_handle_t** column_family_handles, char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_t* crocksdb_open_column_families_with_ttl(
    const crocksdb_options_t* options, const char* name, int num_column_families,
    const char** column_family_names,
    const crocksdb_options_t** column_family_options,
    const int32_t* ttl_array, unsigned char read_only,
    crocksdb_column_family_handle_t** column_family_handles,
    char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_t*
crocksdb_open_for_read_only_column_families(
    const crocksdb_options_t* options, const char* name, int num_column_families,
    const char** column_family_names,
    const crocksdb_options_t** column_family_options,
    crocksdb_column_family_handle_t** column_family_handles,
    unsigned char error_if_log_file_exist, char** errptr);

extern C_ROCKSDB_LIBRARY_API char** crocksdb_list_column_families(
    const crocksdb_options_t* options, const char* name, size_t* lencf,
    char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_list_column_families_destroy(
    char** list, size_t len);

extern C_ROCKSDB_LIBRARY_API crocksdb_column_family_handle_t*
crocksdb_create_column_family(crocksdb_t* db,
                             const crocksdb_options_t* column_family_options,
                             const char* column_family_name, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_drop_column_family(
    crocksdb_t* db, crocksdb_column_family_handle_t* handle, char** errptr);

extern C_ROCKSDB_LIBRARY_API uint32_t crocksdb_column_family_handle_id(
    crocksdb_column_family_handle_t*);

extern C_ROCKSDB_LIBRARY_API void crocksdb_column_family_handle_destroy(
    crocksdb_column_family_handle_t*);

extern C_ROCKSDB_LIBRARY_API void crocksdb_close(crocksdb_t* db);

// This function will wait until all currently running background processes
// finish. After it returns, no background process will be run until
// crocksdb_continue_bg_work is called
extern C_ROCKSDB_LIBRARY_API void crocksdb_pause_bg_work(crocksdb_t* db);
extern C_ROCKSDB_LIBRARY_API void crocksdb_continue_bg_work(crocksdb_t* db);

extern C_ROCKSDB_LIBRARY_API void crocksdb_put(
    crocksdb_t* db, const crocksdb_writeoptions_t* options, const char* key,
    size_t keylen, const char* val, size_t vallen, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_put_cf(
    crocksdb_t* db, const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family, const char* key,
    size_t keylen, const char* val, size_t vallen, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_delete(
    crocksdb_t* db, const crocksdb_writeoptions_t* options, const char* key,
    size_t keylen, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_delete_cf(
    crocksdb_t* db, const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family, const char* key,
    size_t keylen, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_single_delete(
    crocksdb_t* db, const crocksdb_writeoptions_t* options, const char* key,
    size_t keylen, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_single_delete_cf(
    crocksdb_t* db, const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family, const char* key,
    size_t keylen, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_delete_range_cf(
    crocksdb_t* db, const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family,
    const char* begin_key, size_t begin_keylen,
    const char* end_key, size_t end_keylen,
    char **errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_merge(
    crocksdb_t* db, const crocksdb_writeoptions_t* options, const char* key,
    size_t keylen, const char* val, size_t vallen, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_merge_cf(
    crocksdb_t* db, const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family, const char* key,
    size_t keylen, const char* val, size_t vallen, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_write(
    crocksdb_t* db, const crocksdb_writeoptions_t* options,
    crocksdb_writebatch_t* batch, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_write_multi_batch(
    crocksdb_t* db, const crocksdb_writeoptions_t* options,
    crocksdb_writebatch_t** batches, size_t batch_size, char** errptr);

/* Returns NULL if not found.  A malloc()ed array otherwise.
   Stores the length of the array in *vallen. */
extern C_ROCKSDB_LIBRARY_API char* crocksdb_get(
    crocksdb_t* db, const crocksdb_readoptions_t* options, const char* key,
    size_t keylen, size_t* vallen, char** errptr);

extern C_ROCKSDB_LIBRARY_API char* crocksdb_get_cf(
    crocksdb_t* db, const crocksdb_readoptions_t* options,
    crocksdb_column_family_handle_t* column_family, const char* key,
    size_t keylen, size_t* vallen, char** errptr);

// if values_list[i] == NULL and errs[i] == NULL,
// then we got status.IsNotFound(), which we will not return.
// all errors except status status.ok() and status.IsNotFound() are returned.
//
// errs, values_list and values_list_sizes must be num_keys in length,
// allocated by the caller.
// errs is a list of strings as opposed to the conventional one error,
// where errs[i] is the status for retrieval of keys_list[i].
// each non-NULL errs entry is a malloc()ed, null terminated string.
// each non-NULL values_list entry is a malloc()ed array, with
// the length for each stored in values_list_sizes[i].
extern C_ROCKSDB_LIBRARY_API void crocksdb_multi_get(
    crocksdb_t* db, const crocksdb_readoptions_t* options, size_t num_keys,
    const char* const* keys_list, const size_t* keys_list_sizes,
    char** values_list, size_t* values_list_sizes, char** errs);

extern C_ROCKSDB_LIBRARY_API void crocksdb_multi_get_cf(
    crocksdb_t* db, const crocksdb_readoptions_t* options,
    const crocksdb_column_family_handle_t* const* column_families,
    size_t num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes, char** values_list,
    size_t* values_list_sizes, char** errs);

extern C_ROCKSDB_LIBRARY_API crocksdb_iterator_t* crocksdb_create_iterator(
    crocksdb_t* db, const crocksdb_readoptions_t* options);

extern C_ROCKSDB_LIBRARY_API crocksdb_iterator_t* crocksdb_create_iterator_cf(
    crocksdb_t* db, const crocksdb_readoptions_t* options,
    crocksdb_column_family_handle_t* column_family);

extern C_ROCKSDB_LIBRARY_API void crocksdb_create_iterators(
    crocksdb_t *db, crocksdb_readoptions_t* opts,
    crocksdb_column_family_handle_t** column_families,
    crocksdb_iterator_t** iterators, size_t size, char** errptr);

extern C_ROCKSDB_LIBRARY_API const crocksdb_snapshot_t* crocksdb_create_snapshot(
    crocksdb_t* db);

extern C_ROCKSDB_LIBRARY_API void crocksdb_release_snapshot(
    crocksdb_t* db, const crocksdb_snapshot_t* snapshot);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_get_snapshot_sequence_number(const crocksdb_snapshot_t* snapshot);

/* Returns NULL if property name is unknown.
   Else returns a pointer to a malloc()-ed null-terminated value. */
extern C_ROCKSDB_LIBRARY_API crocksdb_map_property_t* crocksdb_create_map_property();

extern C_ROCKSDB_LIBRARY_API void crocksdb_destroy_map_property(crocksdb_map_property_t* info);

extern C_ROCKSDB_LIBRARY_API unsigned char crocksdb_get_map_property_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    const char* property, crocksdb_map_property_t* data);

extern C_ROCKSDB_LIBRARY_API char* crocksdb_map_property_value(
    crocksdb_map_property_t* info,
    const char* propname);

extern C_ROCKSDB_LIBRARY_API uint64_t crocksdb_map_property_int_value(
    crocksdb_map_property_t* info, const char* propname);

extern C_ROCKSDB_LIBRARY_API char* crocksdb_property_value(crocksdb_t* db,
                                                        const char* propname);

extern C_ROCKSDB_LIBRARY_API char* crocksdb_property_value_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    const char* propname);

extern C_ROCKSDB_LIBRARY_API void crocksdb_approximate_sizes(
    crocksdb_t* db, int num_ranges, const char* const* range_start_key,
    const size_t* range_start_key_len, const char* const* range_limit_key,
    const size_t* range_limit_key_len, uint64_t* sizes);

extern C_ROCKSDB_LIBRARY_API void crocksdb_approximate_sizes_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    int num_ranges, const char* const* range_start_key,
    const size_t* range_start_key_len, const char* const* range_limit_key,
    const size_t* range_limit_key_len, uint64_t* sizes);

extern C_ROCKSDB_LIBRARY_API void crocksdb_approximate_memtable_stats(
    const crocksdb_t* db,
    const char* range_start_key, size_t range_start_key_len,
    const char* range_limit_key, size_t range_limit_key_len,
    uint64_t* count, uint64_t* size);

extern C_ROCKSDB_LIBRARY_API void crocksdb_approximate_memtable_stats_cf(
    const crocksdb_t* db, const crocksdb_column_family_handle_t* cf,
    const char* range_start_key, size_t range_start_key_len,
    const char* range_limit_key, size_t range_limit_key_len,
    uint64_t* count, uint64_t* size);

extern C_ROCKSDB_LIBRARY_API void crocksdb_compact_range(crocksdb_t* db,
                                                      const char* start_key,
                                                      size_t start_key_len,
                                                      const char* limit_key,
                                                      size_t limit_key_len);

extern C_ROCKSDB_LIBRARY_API void crocksdb_compact_range_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    const char* start_key, size_t start_key_len, const char* limit_key,
    size_t limit_key_len);

extern C_ROCKSDB_LIBRARY_API void crocksdb_compact_range_opt(
    crocksdb_t* db, crocksdb_compactoptions_t* opt, const char* start_key,
    size_t start_key_len, const char* limit_key, size_t limit_key_len);

extern C_ROCKSDB_LIBRARY_API void crocksdb_compact_range_cf_opt(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    crocksdb_compactoptions_t* opt, const char* start_key, size_t start_key_len,
    const char* limit_key, size_t limit_key_len);

extern C_ROCKSDB_LIBRARY_API void crocksdb_delete_file(crocksdb_t* db,
                                                    const char* name);

extern C_ROCKSDB_LIBRARY_API const crocksdb_livefiles_t* crocksdb_livefiles(
    crocksdb_t* db);

extern C_ROCKSDB_LIBRARY_API void crocksdb_flush(
    crocksdb_t* db, const crocksdb_flushoptions_t* options, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_flush_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    const crocksdb_flushoptions_t* options, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_flush_cfs(
  crocksdb_t* db, const crocksdb_column_family_handle_t** column_familys,
  int num_handles, const crocksdb_flushoptions_t* options, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_flush_wal(
    crocksdb_t* db, unsigned char sync, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_sync_wal(
    crocksdb_t* db, char** errptr);

extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_get_latest_sequence_number(crocksdb_t* db);

extern C_ROCKSDB_LIBRARY_API void crocksdb_disable_file_deletions(crocksdb_t* db,
                                                               char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_enable_file_deletions(
    crocksdb_t* db, unsigned char force, char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_options_t*
crocksdb_get_db_options(crocksdb_t* db);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_set_db_options(crocksdb_t* db,
                        const char** names,
                        const char** values,
                        size_t num_options,
                        char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_options_t* crocksdb_get_options_cf(
    const crocksdb_t* db, crocksdb_column_family_handle_t* column_family);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_set_options_cf(crocksdb_t* db,
                        crocksdb_column_family_handle_t* cf,
                        const char** names,
                        const char** values,
                        size_t num_options,
                        char** errptr);

/* Management operations */

extern C_ROCKSDB_LIBRARY_API void crocksdb_destroy_db(
    const crocksdb_options_t* options, const char* name, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_repair_db(
    const crocksdb_options_t* options, const char* name, char** errptr);

/* Iterator */

extern C_ROCKSDB_LIBRARY_API void crocksdb_iter_destroy(crocksdb_iterator_t*);
extern C_ROCKSDB_LIBRARY_API unsigned char crocksdb_iter_valid(
    const crocksdb_iterator_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_iter_seek_to_first(crocksdb_iterator_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_iter_seek_to_last(crocksdb_iterator_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_iter_seek(crocksdb_iterator_t*,
                                                  const char* k, size_t klen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_iter_seek_for_prev(crocksdb_iterator_t*,
                                                           const char* k,
                                                           size_t klen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_iter_next(crocksdb_iterator_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_iter_prev(crocksdb_iterator_t*);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_iter_key(
    const crocksdb_iterator_t*, size_t* klen);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_iter_value(
    const crocksdb_iterator_t*, size_t* vlen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_iter_get_error(
    const crocksdb_iterator_t*, char** errptr);

/* Write batch */

extern C_ROCKSDB_LIBRARY_API crocksdb_writebatch_t* crocksdb_writebatch_create();
extern C_ROCKSDB_LIBRARY_API crocksdb_writebatch_t*
    crocksdb_writebatch_create_with_capacity(size_t reserved_bytes);
extern C_ROCKSDB_LIBRARY_API crocksdb_writebatch_t* crocksdb_writebatch_create_from(
    const char* rep, size_t size);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_destroy(
    crocksdb_writebatch_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_clear(crocksdb_writebatch_t*);
extern C_ROCKSDB_LIBRARY_API int crocksdb_writebatch_count(crocksdb_writebatch_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_put(crocksdb_writebatch_t*,
                                                       const char* key,
                                                       size_t klen,
                                                       const char* val,
                                                       size_t vlen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_put_cf(
    crocksdb_writebatch_t*, crocksdb_column_family_handle_t* column_family,
    const char* key, size_t klen, const char* val, size_t vlen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_putv(
    crocksdb_writebatch_t* b, int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes, int num_values,
    const char* const* values_list, const size_t* values_list_sizes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_putv_cf(
    crocksdb_writebatch_t* b, crocksdb_column_family_handle_t* column_family,
    int num_keys, const char* const* keys_list, const size_t* keys_list_sizes,
    int num_values, const char* const* values_list,
    const size_t* values_list_sizes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_merge(crocksdb_writebatch_t*,
                                                         const char* key,
                                                         size_t klen,
                                                         const char* val,
                                                         size_t vlen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_merge_cf(
    crocksdb_writebatch_t*, crocksdb_column_family_handle_t* column_family,
    const char* key, size_t klen, const char* val, size_t vlen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_mergev(
    crocksdb_writebatch_t* b, int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes, int num_values,
    const char* const* values_list, const size_t* values_list_sizes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_mergev_cf(
    crocksdb_writebatch_t* b, crocksdb_column_family_handle_t* column_family,
    int num_keys, const char* const* keys_list, const size_t* keys_list_sizes,
    int num_values, const char* const* values_list,
    const size_t* values_list_sizes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_delete(crocksdb_writebatch_t*,
                                                          const char* key,
                                                          size_t klen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_delete_cf(
    crocksdb_writebatch_t*, crocksdb_column_family_handle_t* column_family,
    const char* key, size_t klen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_single_delete(crocksdb_writebatch_t*,
                                                          const char* key,
                                                          size_t klen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_single_delete_cf(
    crocksdb_writebatch_t*, crocksdb_column_family_handle_t* column_family,
    const char* key, size_t klen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_deletev(
    crocksdb_writebatch_t* b, int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_deletev_cf(
    crocksdb_writebatch_t* b, crocksdb_column_family_handle_t* column_family,
    int num_keys, const char* const* keys_list, const size_t* keys_list_sizes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_delete_range(
    crocksdb_writebatch_t* b, const char* start_key, size_t start_key_len,
    const char* end_key, size_t end_key_len);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_delete_range_cf(
    crocksdb_writebatch_t* b, crocksdb_column_family_handle_t* column_family,
    const char* start_key, size_t start_key_len, const char* end_key,
    size_t end_key_len);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_delete_rangev(
    crocksdb_writebatch_t* b, int num_keys, const char* const* start_keys_list,
    const size_t* start_keys_list_sizes, const char* const* end_keys_list,
    const size_t* end_keys_list_sizes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_delete_rangev_cf(
    crocksdb_writebatch_t* b, crocksdb_column_family_handle_t* column_family,
    int num_keys, const char* const* start_keys_list,
    const size_t* start_keys_list_sizes, const char* const* end_keys_list,
    const size_t* end_keys_list_sizes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_put_log_data(
    crocksdb_writebatch_t*, const char* blob, size_t len);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_iterate(
    crocksdb_writebatch_t*, void* state,
    void (*put)(void*, const char* k, size_t klen, const char* v, size_t vlen),
    void (*deleted)(void*, const char* k, size_t klen));
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_writebatch_data(
    crocksdb_writebatch_t*, size_t* size);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_set_save_point(crocksdb_writebatch_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_pop_save_point(
    crocksdb_writebatch_t*, char** errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writebatch_rollback_to_save_point(crocksdb_writebatch_t*, char** errptr);

/* Block based table options */

extern C_ROCKSDB_LIBRARY_API crocksdb_block_based_table_options_t*
crocksdb_block_based_options_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_block_based_options_destroy(
    crocksdb_block_based_table_options_t* options);
extern C_ROCKSDB_LIBRARY_API void crocksdb_block_based_options_set_metadata_block_size(
    crocksdb_block_based_table_options_t* options, size_t block_size);
extern C_ROCKSDB_LIBRARY_API void crocksdb_block_based_options_set_block_size(
    crocksdb_block_based_table_options_t* options, size_t block_size);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_block_size_deviation(
    crocksdb_block_based_table_options_t* options, int block_size_deviation);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_block_restart_interval(
    crocksdb_block_based_table_options_t* options, int block_restart_interval);
extern C_ROCKSDB_LIBRARY_API void crocksdb_block_based_options_set_filter_policy(
    crocksdb_block_based_table_options_t* options,
    crocksdb_filterpolicy_t* filter_policy);
extern C_ROCKSDB_LIBRARY_API void crocksdb_block_based_options_set_no_block_cache(
    crocksdb_block_based_table_options_t* options, unsigned char no_block_cache);
extern C_ROCKSDB_LIBRARY_API void crocksdb_block_based_options_set_block_cache(
    crocksdb_block_based_table_options_t* options, crocksdb_cache_t* block_cache);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_block_cache_compressed(
    crocksdb_block_based_table_options_t* options,
    crocksdb_cache_t* block_cache_compressed);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_whole_key_filtering(
    crocksdb_block_based_table_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_block_based_options_set_format_version(
    crocksdb_block_based_table_options_t*, int);
enum {
  crocksdb_block_based_table_index_type_binary_search = 0,
  crocksdb_block_based_table_index_type_hash_search = 1,
  crocksdb_block_based_table_index_type_two_level_index_search = 2,
};
extern C_ROCKSDB_LIBRARY_API void crocksdb_block_based_options_set_index_type(
    crocksdb_block_based_table_options_t*, int);  // uses one of the above enums
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_hash_index_allow_collision(
    crocksdb_block_based_table_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_partition_filters(
    crocksdb_block_based_table_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_cache_index_and_filter_blocks(
    crocksdb_block_based_table_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_pin_top_level_index_and_filter(
    crocksdb_block_based_table_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_cache_index_and_filter_blocks_with_high_priority(
    crocksdb_block_based_table_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_pin_l0_filter_and_index_blocks_in_cache(
    crocksdb_block_based_table_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_block_based_options_set_read_amp_bytes_per_bit(
    crocksdb_block_based_table_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_block_based_table_factory(
    crocksdb_options_t* opt, crocksdb_block_based_table_options_t* table_options);

extern C_ROCKSDB_LIBRARY_API size_t crocksdb_options_get_block_cache_usage(
    crocksdb_options_t *opt);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_block_cache_capacity(
  crocksdb_options_t* opt, size_t capacity, char **errptr);
extern C_ROCKSDB_LIBRARY_API size_t crocksdb_options_get_block_cache_capacity(
  crocksdb_options_t* opt);

/* Flush job info */

extern C_ROCKSDB_LIBRARY_API const char* crocksdb_flushjobinfo_cf_name(
    const crocksdb_flushjobinfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_flushjobinfo_file_path(
    const crocksdb_flushjobinfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API const crocksdb_table_properties_t*
crocksdb_flushjobinfo_table_properties(const crocksdb_flushjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API unsigned char
crocksdb_flushjobinfo_triggered_writes_slowdown(const crocksdb_flushjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API unsigned char
crocksdb_flushjobinfo_triggered_writes_stop(const crocksdb_flushjobinfo_t*);

/* Compaction job info */
extern C_ROCKSDB_LIBRARY_API void crocksdb_compactionjobinfo_status(const crocksdb_compactionjobinfo_t* info, char**
errptr);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_compactionjobinfo_cf_name(
    const crocksdb_compactionjobinfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API size_t
crocksdb_compactionjobinfo_input_files_count(
    const crocksdb_compactionjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_compactionjobinfo_input_file_at(const crocksdb_compactionjobinfo_t*,
                                         size_t pos, size_t*);
extern C_ROCKSDB_LIBRARY_API size_t
crocksdb_compactionjobinfo_output_files_count(
    const crocksdb_compactionjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_compactionjobinfo_output_file_at(const crocksdb_compactionjobinfo_t*,
                                          size_t pos, size_t*);
extern C_ROCKSDB_LIBRARY_API const crocksdb_table_properties_collection_t*
crocksdb_compactionjobinfo_table_properties(
    const crocksdb_compactionjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_compactionjobinfo_elapsed_micros(const crocksdb_compactionjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_compactionjobinfo_num_corrupt_keys(
    const crocksdb_compactionjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API int
crocksdb_compactionjobinfo_output_level(
    const crocksdb_compactionjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_compactionjobinfo_input_records(
    const crocksdb_compactionjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_compactionjobinfo_output_records(
    const crocksdb_compactionjobinfo_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_compactionjobinfo_total_input_bytes(
    const crocksdb_compactionjobinfo_t* info);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_compactionjobinfo_total_output_bytes(
    const crocksdb_compactionjobinfo_t* info);

/* External file ingestion info */
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_externalfileingestioninfo_cf_name(
    const crocksdb_externalfileingestioninfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_externalfileingestioninfo_internal_file_path(
    const crocksdb_externalfileingestioninfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API const crocksdb_table_properties_t*
crocksdb_externalfileingestioninfo_table_properties(
    const crocksdb_externalfileingestioninfo_t*);

/* External write stall info */
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_writestallinfo_cf_name(
    const crocksdb_writestallinfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API const crocksdb_writestallcondition_t*
crocksdb_writestallinfo_cur(
    const crocksdb_writestallinfo_t*);
extern C_ROCKSDB_LIBRARY_API const crocksdb_writestallcondition_t*
crocksdb_writestallinfo_prev(
    const crocksdb_writestallinfo_t*);

/* Event listener */

typedef void (*on_flush_completed_cb)(void*, crocksdb_t*,
                                      const crocksdb_flushjobinfo_t*);
typedef void (*on_compaction_completed_cb)(void*, crocksdb_t*,
                                           const crocksdb_compactionjobinfo_t*);
typedef void (*on_external_file_ingested_cb)(
    void*, crocksdb_t*, const crocksdb_externalfileingestioninfo_t*);
typedef void (*on_background_error_cb)(void*, crocksdb_backgrounderrorreason_t,
                                       crocksdb_status_ptr_t*);
typedef void (*on_stall_conditions_changed_cb)(
    void*, const crocksdb_writestallinfo_t*);
typedef void (*crocksdb_logger_logv_cb)(void*, int log_level, const char*);

extern C_ROCKSDB_LIBRARY_API crocksdb_eventlistener_t*
crocksdb_eventlistener_create(
    void* state_, void (*destructor_)(void*),
    on_flush_completed_cb on_flush_completed,
    on_compaction_completed_cb on_compaction_completed,
    on_external_file_ingested_cb on_external_file_ingested,
    on_background_error_cb on_background_error,
    on_stall_conditions_changed_cb on_stall_conditions_changed);
extern C_ROCKSDB_LIBRARY_API void crocksdb_eventlistener_destroy(
    crocksdb_eventlistener_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_add_eventlistener(
    crocksdb_options_t*, crocksdb_eventlistener_t*);

/* Cuckoo table options */

extern C_ROCKSDB_LIBRARY_API crocksdb_cuckoo_table_options_t*
crocksdb_cuckoo_options_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_cuckoo_options_destroy(
    crocksdb_cuckoo_table_options_t* options);
extern C_ROCKSDB_LIBRARY_API void crocksdb_cuckoo_options_set_hash_ratio(
    crocksdb_cuckoo_table_options_t* options, double v);
extern C_ROCKSDB_LIBRARY_API void crocksdb_cuckoo_options_set_max_search_depth(
    crocksdb_cuckoo_table_options_t* options, uint32_t v);
extern C_ROCKSDB_LIBRARY_API void crocksdb_cuckoo_options_set_cuckoo_block_size(
    crocksdb_cuckoo_table_options_t* options, uint32_t v);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_cuckoo_options_set_identity_as_first_hash(
    crocksdb_cuckoo_table_options_t* options, unsigned char v);
extern C_ROCKSDB_LIBRARY_API void crocksdb_cuckoo_options_set_use_module_hash(
    crocksdb_cuckoo_table_options_t* options, unsigned char v);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_cuckoo_table_factory(
    crocksdb_options_t* opt, crocksdb_cuckoo_table_options_t* table_options);

/* Options */

extern C_ROCKSDB_LIBRARY_API crocksdb_options_t* crocksdb_options_create();
extern C_ROCKSDB_LIBRARY_API crocksdb_options_t* crocksdb_options_copy(const crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_destroy(crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_column_family_descriptor_destroy(
    crocksdb_column_family_descriptor* cf_desc);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_name_from_column_family_descriptor(
    const crocksdb_column_family_descriptor* cf_desc);
extern C_ROCKSDB_LIBRARY_API crocksdb_options_t*
crocksdb_options_from_column_family_descriptor(
    const crocksdb_column_family_descriptor* cf_desc);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_increase_parallelism(
    crocksdb_options_t* opt, int total_threads);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_optimize_for_point_lookup(
    crocksdb_options_t* opt, uint64_t block_cache_size_mb);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_optimize_level_style_compaction(
    crocksdb_options_t* opt, uint64_t memtable_memory_budget);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_optimize_universal_style_compaction(
    crocksdb_options_t* opt, uint64_t memtable_memory_budget);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_compaction_filter(
    crocksdb_options_t*, crocksdb_compactionfilter_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_compaction_filter_factory(
    crocksdb_options_t*, crocksdb_compactionfilterfactory_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_compaction_readahead_size(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_comparator(
    crocksdb_options_t*, crocksdb_comparator_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_merge_operator(
    crocksdb_options_t*, crocksdb_mergeoperator_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_compression_per_level(
    crocksdb_options_t* opt, int* level_values, size_t num_levels);
extern C_ROCKSDB_LIBRARY_API size_t crocksdb_options_get_compression_level_number(
    crocksdb_options_t* opt);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_get_compression_per_level(
    crocksdb_options_t* opt, int *level_values);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_set_bottommost_compression(crocksdb_options_t *opt, int c);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_create_if_missing(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_create_missing_column_families(crocksdb_options_t*,
                                                   unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_error_if_exists(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_paranoid_checks(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_env(crocksdb_options_t*,
                                                        crocksdb_env_t*);
extern C_ROCKSDB_LIBRARY_API crocksdb_logger_t* crocksdb_logger_create(
    void* rep, void (*destructor_)(void*), crocksdb_logger_logv_cb logv);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_info_log(crocksdb_options_t*,
                                                             crocksdb_logger_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_info_log_level(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_write_buffer_size(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_db_write_buffer_size(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_open_files(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_total_wal_size(
    crocksdb_options_t* opt, uint64_t n);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_compression_options(
    crocksdb_options_t*, int, int, int, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_use_direct_reads(crocksdb_options_t* opt,
    unsigned char v);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_use_direct_io_for_flush_and_compaction(
    crocksdb_options_t *opt, unsigned char v);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_prefix_extractor(
    crocksdb_options_t*, crocksdb_slicetransform_t*);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_memtable_insert_with_hint_prefix_extractor(
    crocksdb_options_t*, crocksdb_slicetransform_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_num_levels(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API int crocksdb_options_get_num_levels(
    crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_level0_file_num_compaction_trigger(crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_level0_slowdown_writes_trigger(crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API int
crocksdb_options_get_level0_slowdown_writes_trigger(crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_level0_stop_writes_trigger(crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API int
crocksdb_options_get_level0_stop_writes_trigger(crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_target_file_size_base(
    crocksdb_options_t*, uint64_t);
extern C_ROCKSDB_LIBRARY_API uint64_t crocksdb_options_get_target_file_size_base(
    const crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_target_file_size_multiplier(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_bytes_for_level_base(
    crocksdb_options_t*, uint64_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_optimize_filters_for_hits(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_level_compaction_dynamic_level_bytes(crocksdb_options_t*,
                                                         unsigned char);
extern C_ROCKSDB_LIBRARY_API unsigned char
crocksdb_options_get_level_compaction_dynamic_level_bytes(
    const crocksdb_options_t* const options);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_max_bytes_for_level_multiplier(crocksdb_options_t*, double);
extern C_ROCKSDB_LIBRARY_API double
crocksdb_options_get_max_bytes_for_level_multiplier(crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_max_bytes_for_level_multiplier_additional(
    crocksdb_options_t*, int* level_values, size_t num_levels);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_enable_statistics(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_reset_statistics(
    crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API unsigned char crocksdb_load_latest_options(
    const char* dbpath, crocksdb_env_t* env, crocksdb_options_t* db_options,
    crocksdb_column_family_descriptor*** cf_descs, size_t* cf_descs_len,
    unsigned char ignore_unknown_options, char** errptr);

/* returns a pointer to a malloc()-ed, null terminated string */
extern C_ROCKSDB_LIBRARY_API char* crocksdb_options_statistics_get_string(
    crocksdb_options_t* opt);
extern C_ROCKSDB_LIBRARY_API uint64_t crocksdb_options_statistics_get_ticker_count(
    crocksdb_options_t* opt, uint32_t ticker_type);
extern C_ROCKSDB_LIBRARY_API uint64_t crocksdb_options_statistics_get_and_reset_ticker_count(
    crocksdb_options_t* opt, uint32_t ticker_type);
extern C_ROCKSDB_LIBRARY_API char*
crocksdb_options_statistics_get_histogram_string(crocksdb_options_t* opt,
                                                 uint32_t type);
extern C_ROCKSDB_LIBRARY_API unsigned char crocksdb_options_statistics_get_histogram(
    crocksdb_options_t* opt,
    uint32_t type,
    double* median,
    double* percentile95,
    double* percentile99,
    double* average,
    double* standard_deviation,
    double* max);

extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_write_buffer_number(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_min_write_buffer_number_to_merge(crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_max_write_buffer_number_to_maintain(crocksdb_options_t *,
                                                         int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_background_jobs(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API int crocksdb_options_get_max_background_jobs(
    const crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_max_log_file_size(crocksdb_options_t *, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_log_file_time_to_roll(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_keep_log_file_num(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_recycle_log_file_num(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_soft_rate_limit(
    crocksdb_options_t*, double);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_hard_rate_limit(
    crocksdb_options_t*, double);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_soft_pending_compaction_bytes_limit(
    crocksdb_options_t* opt, size_t v);
extern C_ROCKSDB_LIBRARY_API size_t crocksdb_options_get_soft_pending_compaction_bytes_limit(
    crocksdb_options_t* opt);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_hard_pending_compaction_bytes_limit(
    crocksdb_options_t* opt, size_t v);
extern C_ROCKSDB_LIBRARY_API size_t crocksdb_options_get_hard_pending_compaction_bytes_limit(
    crocksdb_options_t* opt);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_rate_limit_delay_max_milliseconds(crocksdb_options_t*,
                                                      unsigned int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_manifest_file_size(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_table_cache_numshardbits(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_writable_file_max_buffer_size(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_arena_block_size(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_use_fsync(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_db_paths(crocksdb_options_t *, const char *const *,
                              const size_t *, const uint64_t *, int);
extern C_ROCKSDB_LIBRARY_API size_t
crocksdb_options_get_db_paths_num(crocksdb_options_t *);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_options_get_db_path(crocksdb_options_t *, size_t index);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_options_get_path_target_size(crocksdb_options_t*, size_t index);


extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_db_log_dir(
    crocksdb_options_t*, const char*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_wal_dir(crocksdb_options_t*,
                                                            const char*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_wal_ttl_seconds(
    crocksdb_options_t*, uint64_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_wal_size_limit_mb(
    crocksdb_options_t*, uint64_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_manifest_preallocation_size(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_allow_mmap_reads(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_allow_mmap_writes(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_is_fd_close_on_exec(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_skip_log_error_on_recovery(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_stats_dump_period_sec(
    crocksdb_options_t*, unsigned int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_advise_random_on_open(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_access_hint_on_compaction_start(crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_use_adaptive_mutex(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_bytes_per_sync(
    crocksdb_options_t*, uint64_t);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_enable_pipelined_write(crocksdb_options_t *, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_enable_multi_batch_write(crocksdb_options_t *opt,
                                             unsigned char v);
extern C_ROCKSDB_LIBRARY_API unsigned char
crocksdb_options_is_enable_multi_batch_write(crocksdb_options_t* opt);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_unordered_write(crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_allow_concurrent_memtable_write(crocksdb_options_t *,
                                                     unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_manual_wal_flush(
    crocksdb_options_t *, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_enable_write_thread_adaptive_yield(crocksdb_options_t*,
                                                       unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_max_sequential_skip_in_iterations(crocksdb_options_t*,
                                                      uint64_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_disable_auto_compactions(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API int crocksdb_options_get_disable_auto_compactions(
    const crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_delete_obsolete_files_period_micros(crocksdb_options_t*,
                                                        uint64_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_prepare_for_bulk_load(
    crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_options_get_memtable_factory_name(crocksdb_options_t *opt);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_memtable_vector_rep(
    crocksdb_options_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_memtable_prefix_bloom_size_ratio(
    crocksdb_options_t*, double);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_compaction_bytes(
    crocksdb_options_t*, uint64_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_hash_skip_list_rep(
    crocksdb_options_t*, size_t, int32_t, int32_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_hash_link_list_rep(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_doubly_skip_list_rep(crocksdb_options_t *opt);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_plain_table_factory(
    crocksdb_options_t*, uint32_t, int, double, size_t);

extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_min_level_to_compress(
    crocksdb_options_t* opt, int level);

extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_memtable_huge_page_size(
    crocksdb_options_t*, size_t);

extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_successive_merges(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_bloom_locality(
    crocksdb_options_t*, uint32_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_inplace_update_support(
    crocksdb_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_inplace_update_num_locks(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_report_bg_io_stats(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_compaction_readahead_size(
    crocksdb_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_max_subcompactions(
    crocksdb_options_t*, uint32_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_wal_bytes_per_sync(
    crocksdb_options_t*, uint64_t);

enum {
  crocksdb_tolerate_corrupted_tail_records_recovery = 0,
  crocksdb_absolute_consistency_recovery = 1,
  crocksdb_point_in_time_recovery = 2,
  crocksdb_skip_any_corrupted_records_recovery = 3
};
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_wal_recovery_mode(
    crocksdb_options_t*, int);

enum {
  crocksdb_no_compression = 0,
  crocksdb_snappy_compression = 1,
  crocksdb_zlib_compression = 2,
  crocksdb_bz2_compression = 3,
  crocksdb_lz4_compression = 4,
  crocksdb_lz4hc_compression = 5
};
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_compression(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API int
crocksdb_options_get_compression(crocksdb_options_t *);

enum {
  crocksdb_level_compaction = 0,
  crocksdb_universal_compaction = 1,
  crocksdb_fifo_compaction = 2
};
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_compaction_style(
    crocksdb_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_universal_compaction_options(
    crocksdb_options_t*, crocksdb_universal_compaction_options_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_fifo_compaction_options(
    crocksdb_options_t* opt, crocksdb_fifo_compaction_options_t* fifo);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_ratelimiter(
    crocksdb_options_t* opt, crocksdb_ratelimiter_t* limiter);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_vector_memtable_factory(
    crocksdb_options_t* opt, uint64_t reserved_bytes);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_atomic_flush(
    crocksdb_options_t* opt, unsigned char enable);

enum {
  compaction_by_compensated_size = 0,
  compaction_by_oldest_largestseq_first = 1,
  compaction_by_oldest_smallest_seq_first = 2,
  compaction_by_min_overlapping_ratio = 3,
};
extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_set_compaction_priority(crocksdb_options_t *, unsigned char);

extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_delayed_write_rate(
    crocksdb_options_t*, uint64_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_options_set_force_consistency_checks(
    crocksdb_options_t*, unsigned char);

/* RateLimiter */
extern C_ROCKSDB_LIBRARY_API crocksdb_ratelimiter_t* crocksdb_ratelimiter_create(
    int64_t rate_bytes_per_sec, int64_t refill_period_us, int32_t fairness);
extern C_ROCKSDB_LIBRARY_API crocksdb_ratelimiter_t* crocksdb_ratelimiter_create_with_auto_tuned(
    int64_t rate_bytes_per_sec, int64_t refill_period_us, int32_t fairness,
    crocksdb_ratelimiter_mode_t mode, unsigned char auto_tuned);
extern C_ROCKSDB_LIBRARY_API void crocksdb_ratelimiter_destroy(crocksdb_ratelimiter_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_ratelimiter_set_bytes_per_second(
    crocksdb_ratelimiter_t *limiter, int64_t rate_bytes_per_sec);
extern C_ROCKSDB_LIBRARY_API int64_t crocksdb_ratelimiter_get_singleburst_bytes(
    crocksdb_ratelimiter_t *limiter);
enum {
  env_io_priority_low = 0,
  env_io_priority_high = 1,
  env_io_priority_total = 2,
};
extern C_ROCKSDB_LIBRARY_API void crocksdb_ratelimiter_request(crocksdb_ratelimiter_t *limiter,
    int64_t bytes, unsigned char pri);
extern C_ROCKSDB_LIBRARY_API int64_t crocksdb_ratelimiter_get_total_bytes_through(
    crocksdb_ratelimiter_t *limiter, unsigned char pri);
extern C_ROCKSDB_LIBRARY_API int64_t crocksdb_ratelimiter_get_bytes_per_second(
    crocksdb_ratelimiter_t *limiter);
extern C_ROCKSDB_LIBRARY_API int64_t crocksdb_ratelimiter_get_total_requests(
    crocksdb_ratelimiter_t *limiter, unsigned char pri);

/* Compaction Filter */

extern C_ROCKSDB_LIBRARY_API crocksdb_compactionfilter_t*
crocksdb_compactionfilter_create(
    void* state, void (*destructor)(void*),
    unsigned char (*filter)(void*, int level, const char* key,
                            size_t key_length, const char* existing_value,
                            size_t value_length, char** new_value,
                            size_t* new_value_length,
                            unsigned char* value_changed),
    const char* (*name)(void*));
extern C_ROCKSDB_LIBRARY_API void crocksdb_compactionfilter_set_ignore_snapshots(
    crocksdb_compactionfilter_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_compactionfilter_destroy(
    crocksdb_compactionfilter_t*);

/* Compaction Filter Context */

extern C_ROCKSDB_LIBRARY_API unsigned char
crocksdb_compactionfiltercontext_is_full_compaction(
    crocksdb_compactionfiltercontext_t* context);

extern C_ROCKSDB_LIBRARY_API unsigned char
crocksdb_compactionfiltercontext_is_manual_compaction(
    crocksdb_compactionfiltercontext_t* context);

/* Compaction Filter Factory */

extern C_ROCKSDB_LIBRARY_API crocksdb_compactionfilterfactory_t*
crocksdb_compactionfilterfactory_create(
    void* state, void (*destructor)(void*),
    crocksdb_compactionfilter_t* (*create_compaction_filter)(
        void*, crocksdb_compactionfiltercontext_t* context),
    const char* (*name)(void*));
extern C_ROCKSDB_LIBRARY_API void crocksdb_compactionfilterfactory_destroy(
    crocksdb_compactionfilterfactory_t*);

/* Comparator */

extern C_ROCKSDB_LIBRARY_API crocksdb_comparator_t* crocksdb_comparator_create(
    void* state, void (*destructor)(void*),
    int (*compare)(void*, const char* a, size_t alen, const char* b,
                   size_t blen),
    const char* (*name)(void*));
extern C_ROCKSDB_LIBRARY_API void crocksdb_comparator_destroy(
    crocksdb_comparator_t*);

/* Filter policy */

extern C_ROCKSDB_LIBRARY_API crocksdb_filterpolicy_t* crocksdb_filterpolicy_create(
    void* state, void (*destructor)(void*),
    char* (*create_filter)(void*, const char* const* key_array,
                           const size_t* key_length_array, int num_keys,
                           size_t* filter_length),
    unsigned char (*key_may_match)(void*, const char* key, size_t length,
                                   const char* filter, size_t filter_length),
    void (*delete_filter)(void*, const char* filter, size_t filter_length),
    const char* (*name)(void*));
extern C_ROCKSDB_LIBRARY_API void crocksdb_filterpolicy_destroy(
    crocksdb_filterpolicy_t*);

extern C_ROCKSDB_LIBRARY_API crocksdb_filterpolicy_t*
crocksdb_filterpolicy_create_bloom(int bits_per_key);
extern C_ROCKSDB_LIBRARY_API crocksdb_filterpolicy_t*
crocksdb_filterpolicy_create_bloom_full(int bits_per_key);

/* Merge Operator */

extern C_ROCKSDB_LIBRARY_API crocksdb_mergeoperator_t*
crocksdb_mergeoperator_create(
    void* state, void (*destructor)(void*),
    char* (*full_merge)(void*, const char* key, size_t key_length,
                        const char* existing_value,
                        size_t existing_value_length,
                        const char* const* operands_list,
                        const size_t* operands_list_length, int num_operands,
                        unsigned char* success, size_t* new_value_length),
    char* (*partial_merge)(void*, const char* key, size_t key_length,
                           const char* const* operands_list,
                           const size_t* operands_list_length, int num_operands,
                           unsigned char* success, size_t* new_value_length),
    void (*delete_value)(void*, const char* value, size_t value_length),
    const char* (*name)(void*));
extern C_ROCKSDB_LIBRARY_API void crocksdb_mergeoperator_destroy(
    crocksdb_mergeoperator_t*);

/* Read options */

extern C_ROCKSDB_LIBRARY_API crocksdb_readoptions_t* crocksdb_readoptions_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_destroy(
    crocksdb_readoptions_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_verify_checksums(
    crocksdb_readoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_fill_cache(
    crocksdb_readoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_snapshot(
    crocksdb_readoptions_t*, const crocksdb_snapshot_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_iterate_lower_bound(
    crocksdb_readoptions_t*, const char* key, size_t keylen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_iterate_upper_bound(
    crocksdb_readoptions_t*, const char* key, size_t keylen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_read_tier(
    crocksdb_readoptions_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_tailing(
    crocksdb_readoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_readoptions_set_managed(crocksdb_readoptions_t *, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_readahead_size(
    crocksdb_readoptions_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_readoptions_set_max_skippable_internal_keys(crocksdb_readoptions_t *,
                                                     uint64_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_total_order_seek(
    crocksdb_readoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_readoptions_set_prefix_same_as_start(
    crocksdb_readoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_readoptions_set_pin_data(crocksdb_readoptions_t *, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_readoptions_set_background_purge_on_iterator_cleanup(
    crocksdb_readoptions_t *, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_readoptions_set_ignore_range_deletions(crocksdb_readoptions_t *,
                                                unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_readoptions_set_table_filter(
    crocksdb_readoptions_t*,
    void*,
    int(*table_filter)(void*, const crocksdb_table_properties_t*),
    void(*destory)(void*));

/* Write options */

extern C_ROCKSDB_LIBRARY_API crocksdb_writeoptions_t*
crocksdb_writeoptions_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_writeoptions_destroy(
    crocksdb_writeoptions_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writeoptions_set_sync(
    crocksdb_writeoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_writeoptions_disable_wal(
    crocksdb_writeoptions_t* opt, int disable);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_writeoptions_set_ignore_missing_column_families(
    crocksdb_writeoptions_t *, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_writeoptions_set_no_slowdown(crocksdb_writeoptions_t *, unsigned char);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_writeoptions_set_low_pri(crocksdb_writeoptions_t *, unsigned char);

/* Compact range options */

extern C_ROCKSDB_LIBRARY_API crocksdb_compactoptions_t*
crocksdb_compactoptions_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_compactoptions_destroy(
    crocksdb_compactoptions_t*);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_compactoptions_set_exclusive_manual_compaction(
    crocksdb_compactoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_compactoptions_set_change_level(
    crocksdb_compactoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_compactoptions_set_target_level(
    crocksdb_compactoptions_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_compactoptions_set_target_path_id(
    crocksdb_compactoptions_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_compactoptions_set_max_subcompactions(crocksdb_compactoptions_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_compactoptions_set_bottommost_level_compaction(crocksdb_compactoptions_t*, int);

/* Flush options */

extern C_ROCKSDB_LIBRARY_API crocksdb_flushoptions_t*
crocksdb_flushoptions_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_flushoptions_destroy(
    crocksdb_flushoptions_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_flushoptions_set_wait(
    crocksdb_flushoptions_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_flushoptions_set_allow_write_stall(
    crocksdb_flushoptions_t*, unsigned char);

/* Memory allocator */

extern C_ROCKSDB_LIBRARY_API crocksdb_memory_allocator_t*
crocksdb_jemalloc_nodump_allocator_create(char** errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_memory_allocator_destroy(crocksdb_memory_allocator_t*);

/* Cache */

extern C_ROCKSDB_LIBRARY_API crocksdb_lru_cache_options_t*
crocksdb_lru_cache_options_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_lru_cache_options_destroy(
    crocksdb_lru_cache_options_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_lru_cache_options_set_capacity(
    crocksdb_lru_cache_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void crocksdb_lru_cache_options_set_num_shard_bits(
    crocksdb_lru_cache_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_lru_cache_options_set_strict_capacity_limit(
    crocksdb_lru_cache_options_t*, unsigned char);
extern C_ROCKSDB_LIBRARY_API void crocksdb_lru_cache_options_set_high_pri_pool_ratio(
    crocksdb_lru_cache_options_t*, double);
extern C_ROCKSDB_LIBRARY_API void crocksdb_lru_cache_options_set_memory_allocator(
    crocksdb_lru_cache_options_t*, crocksdb_memory_allocator_t*);
extern C_ROCKSDB_LIBRARY_API crocksdb_cache_t* crocksdb_cache_create_lru(
    crocksdb_lru_cache_options_t*);
extern C_ROCKSDB_LIBRARY_API void crocksdb_cache_destroy(crocksdb_cache_t* cache);
extern C_ROCKSDB_LIBRARY_API void crocksdb_cache_set_capacity(
    crocksdb_cache_t* cache, size_t capacity);

/* Env */

extern C_ROCKSDB_LIBRARY_API crocksdb_env_t* crocksdb_default_env_create();
extern C_ROCKSDB_LIBRARY_API crocksdb_env_t* crocksdb_mem_env_create();
extern C_ROCKSDB_LIBRARY_API crocksdb_env_t*
crocksdb_ctr_encrypted_env_create(crocksdb_env_t* base_env,
                                  const char* ciphertext,
                                  size_t ciphertext_len);
extern C_ROCKSDB_LIBRARY_API void crocksdb_env_set_background_threads(
    crocksdb_env_t* env, int n);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_env_set_high_priority_background_threads(crocksdb_env_t* env, int n);
extern C_ROCKSDB_LIBRARY_API void crocksdb_env_join_all_threads(
    crocksdb_env_t* env);
extern C_ROCKSDB_LIBRARY_API void crocksdb_env_file_exists(
    crocksdb_env_t* env, const char* path, char** errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_env_delete_file(
    crocksdb_env_t* env, const char* path, char** errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_env_destroy(crocksdb_env_t*);

extern C_ROCKSDB_LIBRARY_API crocksdb_envoptions_t* crocksdb_envoptions_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_envoptions_destroy(
    crocksdb_envoptions_t* opt);

extern C_ROCKSDB_LIBRARY_API crocksdb_sequential_file_t*
crocksdb_sequential_file_create(crocksdb_env_t* env, const char* path,
                                const crocksdb_envoptions_t* opts, char** errptr);
extern C_ROCKSDB_LIBRARY_API size_t crocksdb_sequential_file_read(
    crocksdb_sequential_file_t*, size_t n, char* buf, char** errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_sequential_file_skip(
    crocksdb_sequential_file_t*, size_t n, char** errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_sequential_file_destroy(
    crocksdb_sequential_file_t*);

/* KeyManagedEncryptedEnv */

#ifdef OPENSSL
extern C_ROCKSDB_LIBRARY_API crocksdb_file_encryption_info_t*
crocksdb_file_encryption_info_create();
extern C_ROCKSDB_LIBRARY_API void crocksdb_file_encryption_info_destroy(
    crocksdb_file_encryption_info_t* file_info);
extern C_ROCKSDB_LIBRARY_API crocksdb_encryption_method_t
crocksdb_file_encryption_info_method(
    crocksdb_file_encryption_info_t* file_info);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_file_encryption_info_key(
    crocksdb_file_encryption_info_t* file_info, size_t* keylen);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_file_encryption_info_iv(
    crocksdb_file_encryption_info_t* file_info, size_t* ivlen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_file_encryption_info_set_method(
    crocksdb_file_encryption_info_t* file_info,
    crocksdb_encryption_method_t method);
extern C_ROCKSDB_LIBRARY_API void crocksdb_file_encryption_info_set_key(
    crocksdb_file_encryption_info_t* file_info, const char* key, size_t keylen);
extern C_ROCKSDB_LIBRARY_API void crocksdb_file_encryption_info_set_iv(
    crocksdb_file_encryption_info_t* file_info, const char* iv, size_t ivlen);

typedef const char* (*crocksdb_encryption_key_manager_get_file_cb)(
    void* state, const char* fname, crocksdb_file_encryption_info_t* file_info);
typedef const char* (*crocksdb_encryption_key_manager_new_file_cb)(
    void* state, const char* fname, crocksdb_file_encryption_info_t* file_info);
typedef const char* (*crocksdb_encryption_key_manager_delete_file_cb)(
    void* state, const char* fname);
typedef const char* (*crocksdb_encryption_key_manager_link_file_cb)(
    void* state, const char* src_fname, const char* dst_fname);
typedef const char* (*crocksdb_encryption_key_manager_rename_file_cb)(
    void* state, const char* src_fname, const char* dst_fname);

extern C_ROCKSDB_LIBRARY_API crocksdb_encryption_key_manager_t*
crocksdb_encryption_key_manager_create(
    void* state, void (*destructor)(void*),
    crocksdb_encryption_key_manager_get_file_cb get_file,
    crocksdb_encryption_key_manager_new_file_cb new_file,
    crocksdb_encryption_key_manager_delete_file_cb delete_file,
    crocksdb_encryption_key_manager_link_file_cb link_file,
    crocksdb_encryption_key_manager_rename_file_cb rename_file);
extern C_ROCKSDB_LIBRARY_API void crocksdb_encryption_key_manager_destroy(
    crocksdb_encryption_key_manager_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_encryption_key_manager_get_file(
    crocksdb_encryption_key_manager_t* key_manager, const char* fname,
    crocksdb_file_encryption_info_t* file_info);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_encryption_key_manager_new_file(
    crocksdb_encryption_key_manager_t* key_manager, const char* fname,
    crocksdb_file_encryption_info_t* file_info);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_encryption_key_manager_delete_file(
    crocksdb_encryption_key_manager_t* key_manager, const char* fname);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_encryption_key_manager_link_file(
    crocksdb_encryption_key_manager_t* key_manager, const char* src_fname,
    const char* dst_fname);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_encryption_key_manager_rename_file(
    crocksdb_encryption_key_manager_t* key_manager, const char* src_fname,
    const char* dst_fname);

extern C_ROCKSDB_LIBRARY_API crocksdb_env_t*
crocksdb_key_managed_encrypted_env_create(crocksdb_env_t*,
                                          crocksdb_encryption_key_manager_t*);
#endif

/* SstFile */

extern C_ROCKSDB_LIBRARY_API crocksdb_sstfilereader_t*
crocksdb_sstfilereader_create(const crocksdb_options_t* io_options);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_sstfilereader_open(crocksdb_sstfilereader_t* reader,
                            const char* name, char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_iterator_t*
crocksdb_sstfilereader_new_iterator(crocksdb_sstfilereader_t* reader,
                                    const crocksdb_readoptions_t* options);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_sstfilereader_read_table_properties(
    const crocksdb_sstfilereader_t* reader,
    void* ctx, void (*cb)(void*, const crocksdb_table_properties_t*));

extern C_ROCKSDB_LIBRARY_API void
crocksdb_sstfilereader_verify_checksum(crocksdb_sstfilereader_t* reader,
                                       char** errptr);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_sstfilereader_destroy(crocksdb_sstfilereader_t* reader);

extern C_ROCKSDB_LIBRARY_API crocksdb_sstfilewriter_t*
crocksdb_sstfilewriter_create(const crocksdb_envoptions_t* env,
                              const crocksdb_options_t* io_options);
extern C_ROCKSDB_LIBRARY_API crocksdb_sstfilewriter_t*
crocksdb_sstfilewriter_create_cf(const crocksdb_envoptions_t* env,
                             const crocksdb_options_t* io_options,
                             crocksdb_column_family_handle_t* column_family);
extern C_ROCKSDB_LIBRARY_API void crocksdb_sstfilewriter_open(
    crocksdb_sstfilewriter_t* writer, const char* name, char** errptr);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_sstfilewriter_put(crocksdb_sstfilewriter_t *writer, const char *key,
                           size_t keylen, const char *val, size_t vallen,
                           char **errptr);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_sstfilewriter_merge(crocksdb_sstfilewriter_t *writer, const char *key,
                             size_t keylen, const char *val, size_t vallen,
                             char **errptr);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_sstfilewriter_delete(crocksdb_sstfilewriter_t *writer, const char *key,
                              size_t keylen, char **errptr);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_sstfilewriter_delete_range(crocksdb_sstfilewriter_t *writer, const char *begin_key,
                                    size_t begin_keylen, const char *end_key, size_t end_keylen,
                                    char **errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_sstfilewriter_finish(
    crocksdb_sstfilewriter_t* writer, crocksdb_externalsstfileinfo_t* info, char** errptr);
extern C_ROCKSDB_LIBRARY_API uint64_t crocksdb_sstfilewriter_file_size(
    crocksdb_sstfilewriter_t* writer);
extern C_ROCKSDB_LIBRARY_API void crocksdb_sstfilewriter_destroy(
    crocksdb_sstfilewriter_t* writer);

/* ExternalSstFileInfo */

extern C_ROCKSDB_LIBRARY_API crocksdb_externalsstfileinfo_t*
crocksdb_externalsstfileinfo_create();
extern C_ROCKSDB_LIBRARY_API void
crocksdb_externalsstfileinfo_destroy(crocksdb_externalsstfileinfo_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_externalsstfileinfo_file_path(crocksdb_externalsstfileinfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_externalsstfileinfo_smallest_key(crocksdb_externalsstfileinfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_externalsstfileinfo_largest_key(crocksdb_externalsstfileinfo_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_externalsstfileinfo_sequence_number(crocksdb_externalsstfileinfo_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_externalsstfileinfo_file_size(crocksdb_externalsstfileinfo_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_externalsstfileinfo_num_entries(crocksdb_externalsstfileinfo_t*);

extern C_ROCKSDB_LIBRARY_API crocksdb_ingestexternalfileoptions_t*
crocksdb_ingestexternalfileoptions_create();
extern C_ROCKSDB_LIBRARY_API void
crocksdb_ingestexternalfileoptions_set_move_files(
    crocksdb_ingestexternalfileoptions_t* opt, unsigned char move_files);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_ingestexternalfileoptions_set_snapshot_consistency(
    crocksdb_ingestexternalfileoptions_t* opt,
    unsigned char snapshot_consistency);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_ingestexternalfileoptions_set_allow_global_seqno(
    crocksdb_ingestexternalfileoptions_t* opt, unsigned char allow_global_seqno);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_ingestexternalfileoptions_set_allow_blocking_flush(
    crocksdb_ingestexternalfileoptions_t* opt,
    unsigned char allow_blocking_flush);
extern C_ROCKSDB_LIBRARY_API void crocksdb_ingestexternalfileoptions_destroy(
    crocksdb_ingestexternalfileoptions_t* opt);

extern C_ROCKSDB_LIBRARY_API void crocksdb_ingest_external_file(
    crocksdb_t* db, const char* const* file_list, const size_t list_len,
    const crocksdb_ingestexternalfileoptions_t* opt, char** errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_ingest_external_file_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* handle,
    const char* const* file_list, const size_t list_len,
    const crocksdb_ingestexternalfileoptions_t* opt, char** errptr);
extern C_ROCKSDB_LIBRARY_API unsigned char crocksdb_ingest_external_file_optimized(
    crocksdb_t* db, crocksdb_column_family_handle_t* handle,
    const char* const* file_list, const size_t list_len,
    const crocksdb_ingestexternalfileoptions_t* opt, char** errptr);

/* SliceTransform */

extern C_ROCKSDB_LIBRARY_API crocksdb_slicetransform_t*
crocksdb_slicetransform_create(
    void* state, void (*destructor)(void*),
    char* (*transform)(void*, const char* key, size_t length,
                       size_t* dst_length),
    unsigned char (*in_domain)(void*, const char* key, size_t length),
    unsigned char (*in_range)(void*, const char* key, size_t length),
    const char* (*name)(void*));
extern C_ROCKSDB_LIBRARY_API crocksdb_slicetransform_t*
    crocksdb_slicetransform_create_fixed_prefix(size_t);
extern C_ROCKSDB_LIBRARY_API crocksdb_slicetransform_t*
crocksdb_slicetransform_create_noop();
extern C_ROCKSDB_LIBRARY_API void crocksdb_slicetransform_destroy(
    crocksdb_slicetransform_t*);

/* Universal Compaction options */

enum {
  crocksdb_similar_size_compaction_stop_style = 0,
  crocksdb_total_size_compaction_stop_style = 1
};

extern C_ROCKSDB_LIBRARY_API crocksdb_universal_compaction_options_t*
crocksdb_universal_compaction_options_create();
extern C_ROCKSDB_LIBRARY_API void
crocksdb_universal_compaction_options_set_size_ratio(
    crocksdb_universal_compaction_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_universal_compaction_options_set_min_merge_width(
    crocksdb_universal_compaction_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_universal_compaction_options_set_max_merge_width(
    crocksdb_universal_compaction_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_universal_compaction_options_set_max_size_amplification_percent(
    crocksdb_universal_compaction_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_universal_compaction_options_set_compression_size_percent(
    crocksdb_universal_compaction_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_universal_compaction_options_set_stop_style(
    crocksdb_universal_compaction_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void crocksdb_universal_compaction_options_destroy(
    crocksdb_universal_compaction_options_t*);

extern C_ROCKSDB_LIBRARY_API crocksdb_fifo_compaction_options_t*
crocksdb_fifo_compaction_options_create();
extern C_ROCKSDB_LIBRARY_API void
crocksdb_fifo_compaction_options_set_max_table_files_size(
    crocksdb_fifo_compaction_options_t* fifo_opts, uint64_t size);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_fifo_compaction_options_set_allow_compaction(
    crocksdb_fifo_compaction_options_t* fifo_opts, unsigned char allow_compaction);
extern C_ROCKSDB_LIBRARY_API void crocksdb_fifo_compaction_options_destroy(
    crocksdb_fifo_compaction_options_t* fifo_opts);

extern C_ROCKSDB_LIBRARY_API int crocksdb_livefiles_count(
    const crocksdb_livefiles_t*);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_livefiles_name(
    const crocksdb_livefiles_t*, int index);
extern C_ROCKSDB_LIBRARY_API int crocksdb_livefiles_level(
    const crocksdb_livefiles_t*, int index);
extern C_ROCKSDB_LIBRARY_API size_t
crocksdb_livefiles_size(const crocksdb_livefiles_t*, int index);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_livefiles_smallestkey(
    const crocksdb_livefiles_t*, int index, size_t* size);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_livefiles_largestkey(
    const crocksdb_livefiles_t*, int index, size_t* size);
extern C_ROCKSDB_LIBRARY_API void crocksdb_livefiles_destroy(
    const crocksdb_livefiles_t*);

/* Utility Helpers */

extern C_ROCKSDB_LIBRARY_API void crocksdb_get_options_from_string(
    const crocksdb_options_t* base_options, const char* opts_str,
    crocksdb_options_t* new_options, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_delete_files_in_range(
    crocksdb_t* db,
    const char* start_key, size_t start_key_len,
    const char* limit_key, size_t limit_key_len,
    unsigned char include_end, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_delete_files_in_range_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    const char* start_key, size_t start_key_len,
    const char* limit_key, size_t limit_key_len,
    unsigned char include_end, char** errptr);

extern C_ROCKSDB_LIBRARY_API void crocksdb_delete_files_in_ranges_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* cf,
    const char* const* start_keys, const size_t* start_keys_lens,
    const char* const* limit_keys, const size_t* limit_keys_lens,
    size_t num_ranges, unsigned char include_end, char** errptr);

// referring to convention (3), this should be used by client
// to free memory that was malloc()ed
extern C_ROCKSDB_LIBRARY_API void crocksdb_free(void* ptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_logger_t* crocksdb_create_env_logger(
    const char* fname, crocksdb_env_t* env);
extern C_ROCKSDB_LIBRARY_API crocksdb_logger_t *
crocksdb_create_log_from_options(const char *path, crocksdb_options_t *opts,
                                 char **errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_log_destroy(crocksdb_logger_t *);

extern C_ROCKSDB_LIBRARY_API crocksdb_pinnableslice_t* crocksdb_get_pinned(
    crocksdb_t* db, const crocksdb_readoptions_t* options, const char* key,
    size_t keylen, char** errptr);
extern C_ROCKSDB_LIBRARY_API crocksdb_pinnableslice_t* crocksdb_get_pinned_cf(
    crocksdb_t* db, const crocksdb_readoptions_t* options,
    crocksdb_column_family_handle_t* column_family, const char* key,
    size_t keylen, char** errptr);
extern C_ROCKSDB_LIBRARY_API void crocksdb_pinnableslice_destroy(
    crocksdb_pinnableslice_t* v);
extern C_ROCKSDB_LIBRARY_API const char* crocksdb_pinnableslice_value(
    const crocksdb_pinnableslice_t* t, size_t* vlen);

extern C_ROCKSDB_LIBRARY_API size_t crocksdb_get_supported_compression_number();
extern C_ROCKSDB_LIBRARY_API void crocksdb_get_supported_compression(int *, size_t);

/* Table Properties */

extern C_ROCKSDB_LIBRARY_API uint64_t crocksdb_table_properties_get_u64(
    const crocksdb_table_properties_t*, crocksdb_table_property_t prop);

extern C_ROCKSDB_LIBRARY_API const char* crocksdb_table_properties_get_str(
    const crocksdb_table_properties_t*, crocksdb_table_property_t prop,
    size_t* slen);

extern C_ROCKSDB_LIBRARY_API const crocksdb_user_collected_properties_t*
crocksdb_table_properties_get_user_properties(
    const crocksdb_table_properties_t*);

extern C_ROCKSDB_LIBRARY_API const char* crocksdb_user_collected_properties_get(
    const crocksdb_user_collected_properties_t* props, const char* key,
    size_t klen, size_t* vlen);

extern C_ROCKSDB_LIBRARY_API size_t crocksdb_user_collected_properties_len(
    const crocksdb_user_collected_properties_t*);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_user_collected_properties_add(
    crocksdb_user_collected_properties_t*,
    const char* key, size_t key_len, const char* value, size_t value_len);

extern C_ROCKSDB_LIBRARY_API crocksdb_user_collected_properties_iterator_t*
crocksdb_user_collected_properties_iter_create(
    const crocksdb_user_collected_properties_t*);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_user_collected_properties_iter_destroy(
   crocksdb_user_collected_properties_iterator_t*);

extern C_ROCKSDB_LIBRARY_API unsigned char
crocksdb_user_collected_properties_iter_valid(
    const crocksdb_user_collected_properties_iterator_t*);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_user_collected_properties_iter_next(
    crocksdb_user_collected_properties_iterator_t*);

extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_user_collected_properties_iter_key(
    const crocksdb_user_collected_properties_iterator_t*, size_t* klen);

extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_user_collected_properties_iter_value(
    const crocksdb_user_collected_properties_iterator_t*, size_t* vlen);

/* Table Properties Collection */

extern C_ROCKSDB_LIBRARY_API size_t crocksdb_table_properties_collection_len(
    const crocksdb_table_properties_collection_t*);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_table_properties_collection_destroy(crocksdb_table_properties_collection_t*);

extern C_ROCKSDB_LIBRARY_API crocksdb_table_properties_collection_iterator_t*
crocksdb_table_properties_collection_iter_create(
    const crocksdb_table_properties_collection_t*);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_table_properties_collection_iter_destroy(
    crocksdb_table_properties_collection_iterator_t*);

extern C_ROCKSDB_LIBRARY_API unsigned char
crocksdb_table_properties_collection_iter_valid(
    const crocksdb_table_properties_collection_iterator_t*);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_table_properties_collection_iter_next(
    crocksdb_table_properties_collection_iterator_t*);

extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_table_properties_collection_iter_key(
    const crocksdb_table_properties_collection_iterator_t*, size_t* klen);

extern C_ROCKSDB_LIBRARY_API const crocksdb_table_properties_t*
crocksdb_table_properties_collection_iter_value(
    const crocksdb_table_properties_collection_iterator_t*);

/* Table Properties Collector */

extern C_ROCKSDB_LIBRARY_API crocksdb_table_properties_collector_t*
crocksdb_table_properties_collector_create(
    void* state,
    const char* (*name)(void*),
    void (*destruct)(void*),
    void (*add)(void*,
                const char* key, size_t key_len,
                const char* value, size_t value_len,
                int entry_type, uint64_t seq, uint64_t file_size),
    void (*finish)(void*, crocksdb_user_collected_properties_t* props));

extern C_ROCKSDB_LIBRARY_API void
crocksdb_table_properties_collector_destroy(crocksdb_table_properties_collector_t*);

/* Table Properties Collector Factory */

extern C_ROCKSDB_LIBRARY_API crocksdb_table_properties_collector_factory_t*
crocksdb_table_properties_collector_factory_create(
    void* state,
    const char* (*name)(void*),
    void (*destruct)(void*),
    crocksdb_table_properties_collector_t*
    (*create_table_properties_collector)(void*, uint32_t cf));

extern C_ROCKSDB_LIBRARY_API void
crocksdb_table_properties_collector_factory_destroy(
    crocksdb_table_properties_collector_factory_t*);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_options_add_table_properties_collector_factory(
    crocksdb_options_t* opt, crocksdb_table_properties_collector_factory_t* f);

/* Get Table Properties */
extern C_ROCKSDB_LIBRARY_API crocksdb_table_properties_collection_t*
crocksdb_get_properties_of_all_tables(crocksdb_t* db, char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_table_properties_collection_t*
crocksdb_get_properties_of_all_tables_cf(crocksdb_t* db,
                                        crocksdb_column_family_handle_t* cf,
                                        char** errptr);

extern C_ROCKSDB_LIBRARY_API crocksdb_table_properties_collection_t*
crocksdb_get_properties_of_tables_in_range(
    crocksdb_t* db, crocksdb_column_family_handle_t* cf, int num_ranges,
    const char* const* start_keys, const size_t* start_keys_lens,
    const char* const* limit_keys, const size_t* limit_keys_lens,
    char** errptr);

/* Get All Key Versions */

extern C_ROCKSDB_LIBRARY_API void
crocksdb_keyversions_destroy(crocksdb_keyversions_t *kvs);

extern C_ROCKSDB_LIBRARY_API crocksdb_keyversions_t *
crocksdb_get_all_key_versions(crocksdb_t *db, const char *begin_key,
                              size_t begin_keylen, const char *end_key,
                              size_t end_keylen, char **errptr);

extern C_ROCKSDB_LIBRARY_API size_t
crocksdb_keyversions_count(const crocksdb_keyversions_t *kvs);

extern C_ROCKSDB_LIBRARY_API const char *
crocksdb_keyversions_key(const crocksdb_keyversions_t *kvs, int index);

extern C_ROCKSDB_LIBRARY_API const char *
crocksdb_keyversions_value(const crocksdb_keyversions_t *kvs, int index);

extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_keyversions_seq(const crocksdb_keyversions_t *kvs, int index);

extern C_ROCKSDB_LIBRARY_API int
crocksdb_keyversions_type(const crocksdb_keyversions_t *kvs, int index);

/* Modify Sst File Seq No */
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_set_external_sst_file_global_seq_no(
  crocksdb_t *db,
  crocksdb_column_family_handle_t *column_family,
  const char *file,
  uint64_t seq_no,
  char **errptr);

/* ColumnFamilyMetaData */
extern C_ROCKSDB_LIBRARY_API void
crocksdb_get_column_family_meta_data(crocksdb_t* db,
                                     crocksdb_column_family_handle_t* cf,
                                     crocksdb_column_family_meta_data_t*);
extern C_ROCKSDB_LIBRARY_API crocksdb_column_family_meta_data_t*
crocksdb_column_family_meta_data_create();
extern C_ROCKSDB_LIBRARY_API void
crocksdb_column_family_meta_data_destroy(crocksdb_column_family_meta_data_t*);
extern C_ROCKSDB_LIBRARY_API size_t
crocksdb_column_family_meta_data_level_count(const crocksdb_column_family_meta_data_t*);
extern C_ROCKSDB_LIBRARY_API const crocksdb_level_meta_data_t*
crocksdb_column_family_meta_data_level_data(const crocksdb_column_family_meta_data_t*,
                                            size_t n);
extern C_ROCKSDB_LIBRARY_API size_t
crocksdb_level_meta_data_file_count(const crocksdb_level_meta_data_t*);
extern C_ROCKSDB_LIBRARY_API const crocksdb_sst_file_meta_data_t*
crocksdb_level_meta_data_file_data(const crocksdb_level_meta_data_t*, size_t n);
extern C_ROCKSDB_LIBRARY_API size_t
crocksdb_sst_file_meta_data_size(const crocksdb_sst_file_meta_data_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_sst_file_meta_data_name(const crocksdb_sst_file_meta_data_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_sst_file_meta_data_smallestkey(const crocksdb_sst_file_meta_data_t*, size_t*);
extern C_ROCKSDB_LIBRARY_API const char*
crocksdb_sst_file_meta_data_largestkey(const crocksdb_sst_file_meta_data_t*, size_t*);

/* CompactFiles */
extern C_ROCKSDB_LIBRARY_API crocksdb_compaction_options_t*
crocksdb_compaction_options_create();
extern C_ROCKSDB_LIBRARY_API void
crocksdb_compaction_options_destroy(crocksdb_compaction_options_t*);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_compaction_options_set_compression(crocksdb_compaction_options_t*, int);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_compaction_options_set_output_file_size_limit(crocksdb_compaction_options_t*, size_t);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_compaction_options_set_max_subcompactions(crocksdb_compaction_options_t*, int);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_compact_files_cf(crocksdb_t*, crocksdb_column_family_handle_t*,
                          crocksdb_compaction_options_t*,
                          const char** input_file_names,
                          size_t input_file_count,
                          int output_level,
                          char** errptr);

/* PerfContext */
extern C_ROCKSDB_LIBRARY_API int crocksdb_get_perf_level(void);
extern C_ROCKSDB_LIBRARY_API void crocksdb_set_perf_level(int level);
extern C_ROCKSDB_LIBRARY_API crocksdb_perf_context_t*
crocksdb_get_perf_context(void);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_perf_context_reset(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_user_key_comparison_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_block_cache_hit_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_block_read_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_block_read_byte(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_block_read_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_block_checksum_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_block_decompress_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_get_read_bytes(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_multiget_read_bytes(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_iter_read_bytes(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_internal_key_skipped_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_internal_delete_skipped_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_internal_recent_skipped_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_internal_merge_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_get_snapshot_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_get_from_memtable_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_get_from_memtable_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_get_post_process_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_get_from_output_files_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_seek_on_memtable_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_seek_on_memtable_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_next_on_memtable_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_prev_on_memtable_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_seek_child_seek_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_seek_child_seek_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_seek_min_heap_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_seek_max_heap_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_seek_internal_seek_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_find_next_user_entry_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_write_wal_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_write_memtable_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_write_delay_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_write_pre_and_post_process_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_db_mutex_lock_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_write_thread_wait_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_write_scheduling_flushes_compactions_time(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_db_condition_wait_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_merge_operator_time_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_read_index_block_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_read_filter_block_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_new_table_block_iter_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_new_table_iterator_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_block_seek_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_find_table_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_bloom_memtable_hit_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_bloom_memtable_miss_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_bloom_sst_hit_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_bloom_sst_miss_count(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_new_sequential_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_new_random_access_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_new_writable_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_reuse_writable_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_new_random_rw_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_new_directory_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_file_exists_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_get_children_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_get_children_file_attributes_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_delete_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_create_dir_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_create_dir_if_missing_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_delete_dir_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_get_file_size_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_get_file_modification_time_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_rename_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_link_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_lock_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_unlock_file_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_env_new_logger_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_encrypt_data_nanos(crocksdb_perf_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_perf_context_decrypt_data_nanos(crocksdb_perf_context_t*);

// IOStatsContext
extern C_ROCKSDB_LIBRARY_API crocksdb_iostats_context_t*
crocksdb_get_iostats_context(void);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_iostats_context_reset(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_bytes_written(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_bytes_read(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_open_nanos(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_allocate_nanos(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_write_nanos(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_read_nanos(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_range_sync_nanos(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_fsync_nanos(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_prepare_write_nanos(crocksdb_iostats_context_t*);
extern C_ROCKSDB_LIBRARY_API uint64_t
crocksdb_iostats_context_logger_nanos(crocksdb_iostats_context_t*);

extern C_ROCKSDB_LIBRARY_API void
crocksdb_run_ldb_tool(int argc, char** argv, const crocksdb_options_t* opts);
extern C_ROCKSDB_LIBRARY_API void
crocksdb_run_sst_dump_tool(int argc, char** argv, const crocksdb_options_t* opts);

/* Titan */
struct ctitandb_blob_index_t {
  uint64_t file_number;
  uint64_t blob_offset;
  uint64_t blob_size;
};

typedef struct ctitandb_options_t ctitandb_options_t;
typedef struct ctitandb_readoptions_t ctitandb_readoptions_t;
typedef struct ctitandb_blob_index_t ctitandb_blob_index_t;

extern C_ROCKSDB_LIBRARY_API crocksdb_t* ctitandb_open_column_families(
    const char* name, 
    const ctitandb_options_t* tdb_options, int num_column_families,
    const char** column_family_names,
    const ctitandb_options_t** titan_column_family_options,
    crocksdb_column_family_handle_t** column_family_handles, char** errptr);

extern C_ROCKSDB_LIBRARY_API
crocksdb_column_family_handle_t* ctitandb_create_column_family(
    crocksdb_t* db,
    const ctitandb_options_t* titan_column_family_options,
    const char* column_family_name,
    char** errptr);

/* TitanDBOptions */

extern C_ROCKSDB_LIBRARY_API ctitandb_options_t* ctitandb_options_create();

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_destroy(ctitandb_options_t*);

extern C_ROCKSDB_LIBRARY_API ctitandb_options_t* ctitandb_options_copy(
    ctitandb_options_t*);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_rocksdb_options(ctitandb_options_t* opts, const crocksdb_options_t* rocksdb_opts);

extern C_ROCKSDB_LIBRARY_API ctitandb_options_t* ctitandb_get_titan_options_cf(
    const crocksdb_t* db, crocksdb_column_family_handle_t* column_family);

extern C_ROCKSDB_LIBRARY_API ctitandb_options_t* ctitandb_get_titan_db_options(crocksdb_t* db);

extern C_ROCKSDB_LIBRARY_API const char* ctitandb_options_dirname(
    ctitandb_options_t*);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_dirname(
    ctitandb_options_t*, const char* name);

extern C_ROCKSDB_LIBRARY_API uint64_t
ctitandb_options_min_blob_size(ctitandb_options_t*);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_min_blob_size(
    ctitandb_options_t*, uint64_t size);

extern C_ROCKSDB_LIBRARY_API int ctitandb_options_blob_file_compression(
    ctitandb_options_t*);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_gc_merge_rewrite(
    ctitandb_options_t*, unsigned char);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_blob_file_compression(
    ctitandb_options_t*, int type);

extern C_ROCKSDB_LIBRARY_API void ctitandb_decode_blob_index(
    const char* value, size_t value_size, ctitandb_blob_index_t* index,
    char** errptr);

extern C_ROCKSDB_LIBRARY_API void ctitandb_encode_blob_index(
    const ctitandb_blob_index_t* index, char** value, size_t* value_size);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_disable_background_gc(
    ctitandb_options_t* options, unsigned char disable);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_level_merge(ctitandb_options_t* options,
                                                unsigned char enable);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_range_merge(ctitandb_options_t* options,
                                                unsigned char enable);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_max_sorted_runs(ctitandb_options_t* options,
                                            int size);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_max_gc_batch_size(
    ctitandb_options_t* options, uint64_t size);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_min_gc_batch_size(
    ctitandb_options_t* options, uint64_t size);

extern C_ROCKSDB_LIBRARY_API void
ctitandb_options_set_blob_file_discardable_ratio(ctitandb_options_t* options,
                                                 double ratio);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_sample_file_size_ratio(
    ctitandb_options_t* options, double ratio);

extern C_ROCKSDB_LIBRARY_API void
ctitandb_options_set_merge_small_file_threshold(ctitandb_options_t* options,
                                                uint64_t size);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_max_background_gc(
    ctitandb_options_t* options, int32_t size);

extern C_ROCKSDB_LIBRARY_API void
ctitandb_options_set_purge_obsolete_files_period_sec(
    ctitandb_options_t* options, unsigned int period);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_blob_cache(
    ctitandb_options_t* options, crocksdb_cache_t* cache);

extern C_ROCKSDB_LIBRARY_API size_t ctitandb_options_get_blob_cache_usage(
    ctitandb_options_t *opt);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_blob_cache_capacity(
  ctitandb_options_t* opt, size_t capacity, char **errptr);

extern C_ROCKSDB_LIBRARY_API size_t ctitandb_options_get_blob_cache_capacity(
  ctitandb_options_t* opt);

extern C_ROCKSDB_LIBRARY_API void ctitandb_options_set_discardable_ratio(
    ctitandb_options_t* options, double ratio);

extern void C_ROCKSDB_LIBRARY_API ctitandb_options_set_sample_ratio(ctitandb_options_t* options,
                                              double ratio);

extern void C_ROCKSDB_LIBRARY_API ctitandb_options_set_blob_run_mode(ctitandb_options_t* options, int mode);

/* TitanReadOptions */

extern C_ROCKSDB_LIBRARY_API ctitandb_readoptions_t* ctitandb_readoptions_create();

extern C_ROCKSDB_LIBRARY_API void ctitandb_readoptions_destroy(ctitandb_readoptions_t* opts);

extern C_ROCKSDB_LIBRARY_API unsigned char ctitandb_readoptions_key_only(ctitandb_readoptions_t* opts);

extern C_ROCKSDB_LIBRARY_API void ctitandb_readoptions_set_key_only(ctitandb_readoptions_t* opts,
                                        unsigned char v);

/* Titan Iterator */

extern C_ROCKSDB_LIBRARY_API crocksdb_iterator_t* ctitandb_create_iterator(
    crocksdb_t* db,
    const crocksdb_readoptions_t* options,
    const ctitandb_readoptions_t* titan_options);

extern C_ROCKSDB_LIBRARY_API crocksdb_iterator_t* ctitandb_create_iterator_cf(
    crocksdb_t* db,
    const crocksdb_readoptions_t* options,
    const ctitandb_readoptions_t* titan_options,
    crocksdb_column_family_handle_t* column_family);

extern C_ROCKSDB_LIBRARY_API void ctitandb_create_iterators(
    crocksdb_t *db,
    crocksdb_readoptions_t* options,
    ctitandb_readoptions_t* titan_options,
    crocksdb_column_family_handle_t** column_families,
    crocksdb_iterator_t** iterators,
    size_t size,
    char** errptr);

extern C_ROCKSDB_LIBRARY_API void ctitandb_delete_files_in_range(
    crocksdb_t* db,
    const char* start_key, size_t start_key_len,
    const char* limit_key, size_t limit_key_len,
    unsigned char include_end, char** errptr);

extern C_ROCKSDB_LIBRARY_API void ctitandb_delete_files_in_range_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    const char* start_key, size_t start_key_len,
    const char* limit_key, size_t limit_key_len,
    unsigned char include_end, char** errptr);

extern C_ROCKSDB_LIBRARY_API void ctitandb_delete_files_in_ranges_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* cf,
    const char* const* start_keys, const size_t* start_keys_lens,
    const char* const* limit_keys, const size_t* limit_keys_lens,
    size_t num_ranges, unsigned char include_end, char** errptr);

#ifdef __cplusplus
}  /* end extern "C" */
#endif

#endif  /* C_ROCKSDB_INCLUDE_CWRAPPER_H_ */
