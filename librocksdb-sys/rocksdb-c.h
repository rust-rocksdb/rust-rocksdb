#include "rocksdb/include/rocksdb/c.h"

#ifdef __cplusplus
extern "C" {
#endif

extern ROCKSDB_LIBRARY_API rocksdb_transaction_t** rocksdb_transactiondb_get_prepared_transactions(
    rocksdb_transactiondb_t* txn_db, size_t* cnt);

extern ROCKSDB_LIBRARY_API void rocksdb_transaction_set_name(
    rocksdb_transaction_t* txn, const char* name, size_t name_len, char** errptr);

extern ROCKSDB_LIBRARY_API char* rocksdb_transaction_get_name(
    rocksdb_transaction_t* txn, size_t* name_len);

extern ROCKSDB_LIBRARY_API void rocksdb_transaction_prepare(
    rocksdb_transaction_t* txn, char** errptr);

extern ROCKSDB_LIBRARY_API rocksdb_writebatch_wi_t* rocksdb_transaction_get_writebatch_wi(
    rocksdb_transaction_t* txn);

extern ROCKSDB_LIBRARY_API void rocksdb_transaction_rebuild_from_writebatch(
    rocksdb_transaction_t* txn, rocksdb_writebatch_t *writebatch, char** errptr);

// This rocksdb_writebatch_wi_t should be freed with rocksdb_free
extern ROCKSDB_LIBRARY_API void rocksdb_transaction_rebuild_from_writebatch_wi(
    rocksdb_transaction_t* txn, rocksdb_writebatch_wi_t *wi, char** errptr);

extern ROCKSDB_LIBRARY_API rocksdb_pinnableslice_t* rocksdb_transaction_get_pinned(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    const char* key, size_t klen, char** errptr);

extern ROCKSDB_LIBRARY_API rocksdb_pinnableslice_t* rocksdb_transaction_get_pinned_cf(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    rocksdb_column_family_handle_t* column_family, const char* key, size_t klen,
    char** errptr);

extern ROCKSDB_LIBRARY_API rocksdb_pinnableslice_t* rocksdb_transaction_get_pinned_for_update(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    const char* key, size_t klen, unsigned char exclusive, char** errptr);

extern ROCKSDB_LIBRARY_API rocksdb_pinnableslice_t* rocksdb_transaction_get_pinned_for_update_cf(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    rocksdb_column_family_handle_t* column_family, const char* key, size_t klen,
    unsigned char exclusive, char** errptr);

extern ROCKSDB_LIBRARY_API rocksdb_pinnableslice_t* rocksdb_transactiondb_get_pinned(
    rocksdb_transactiondb_t* txn_db, const rocksdb_readoptions_t* options,
    const char* key, size_t klen, char** errptr);

extern ROCKSDB_LIBRARY_API rocksdb_pinnableslice_t* rocksdb_transactiondb_get_pinned_cf(
    rocksdb_transactiondb_t* txn_db, const rocksdb_readoptions_t* options,
    rocksdb_column_family_handle_t* column_family, const char* key,
    size_t keylen, char** errptr);

extern ROCKSDB_LIBRARY_API void rocksdb_transactiondb_flush(
    rocksdb_transactiondb_t* txn_db, const rocksdb_flushoptions_t* options, char** errptr);

extern ROCKSDB_LIBRARY_API void rocksdb_transactiondb_flush_cf(
    rocksdb_transactiondb_t* txn_db, const rocksdb_flushoptions_t* options,
    rocksdb_column_family_handle_t* column_family, char** errptr);

extern ROCKSDB_LIBRARY_API void rocksdb_transactiondb_flush_wal(
    rocksdb_transactiondb_t* txn_db, unsigned char sync, char** errptr);

extern ROCKSDB_LIBRARY_API void rocksdb_transaction_options_set_skip_prepare(
    rocksdb_transaction_options_t* opt, unsigned char v);

#ifdef __cplusplus
}  /* end extern "C" */
#endif