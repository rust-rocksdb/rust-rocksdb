#include "rocksdb-c.h"

// Include origin rocksdb C-API.
#include "rocksdb/db/c.cc"
#include <vector>

// Implement some missing functions.
extern "C" {

void rocksdb_transactiondb_flush_wal(
    rocksdb_transactiondb_t* txn_db, unsigned char sync, char** errptr) {
  SaveError(errptr, txn_db->rep->FlushWAL(sync));
}

void rocksdb_transactiondb_flush(
    rocksdb_transactiondb_t* txn_db, const rocksdb_flushoptions_t* options,
    char** errptr) {
  SaveError(errptr, txn_db->rep->Flush(options->rep));
}

void rocksdb_transactiondb_flush_cf(
    rocksdb_transactiondb_t* txn_db, const rocksdb_flushoptions_t* options,
    rocksdb_column_family_handle_t* column_family, char** errptr) {
  SaveError(errptr, txn_db->rep->Flush(options->rep, column_family->rep));
}

rocksdb_pinnableslice_t* rocksdb_transactiondb_get_pinned(
    rocksdb_transactiondb_t* txn_db, const rocksdb_readoptions_t* options,
    const char* key, size_t klen, char** errptr) {
  rocksdb_pinnableslice_t* v = new (rocksdb_pinnableslice_t);
  Status s = txn_db->rep->Get(options->rep, txn_db->rep->DefaultColumnFamily(), Slice(key, klen), &v->rep);
  if (!s.ok()) {
    delete (v);
    if (!s.IsNotFound()) {
      SaveError(errptr, s);
    }
    return nullptr;
  }
  return v;
}

rocksdb_pinnableslice_t* rocksdb_transactiondb_get_pinned_cf(
    rocksdb_transactiondb_t* txn_db, const rocksdb_readoptions_t* options,
    rocksdb_column_family_handle_t* column_family, const char* key,
    size_t keylen, char** errptr) {
  rocksdb_pinnableslice_t* v = new (rocksdb_pinnableslice_t);
  Status s = txn_db->rep->Get(options->rep, column_family->rep, Slice(key, keylen), &v->rep);
  if (!s.ok()) {
    delete (v);
    if (!s.IsNotFound()) {
      SaveError(errptr, s);
    }
    return nullptr;
  }
  return v;
}

rocksdb_pinnableslice_t* rocksdb_transaction_get_pinned(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    const char* key, size_t klen, char** errptr) {
  rocksdb_pinnableslice_t* v = new (rocksdb_pinnableslice_t);
  Status s = txn->rep->Get(options->rep, Slice(key, klen), &v->rep);
  if (!s.ok()) {
    delete (v);
    if (!s.IsNotFound()) {
      SaveError(errptr, s);
    }
    return nullptr;
  }
  return v;
}

rocksdb_pinnableslice_t* rocksdb_transaction_get_pinned_cf(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    rocksdb_column_family_handle_t* column_family, const char* key, size_t klen,
    char** errptr) {
  rocksdb_pinnableslice_t* v = new (rocksdb_pinnableslice_t);
  Status s = txn->rep->Get(options->rep, column_family->rep, Slice(key, klen), &v->rep);
  if (!s.ok()) {
    delete (v);
    if (!s.IsNotFound()) {
      SaveError(errptr, s);
    }
    return nullptr;
  }
  return v;
}

rocksdb_pinnableslice_t* rocksdb_transaction_get_pinned_for_update(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    const char* key, size_t klen, unsigned char exclusive, char** errptr) {
  rocksdb_pinnableslice_t* v = new (rocksdb_pinnableslice_t);
  Status s = txn->rep->GetForUpdate(options->rep, Slice(key, klen), v->rep.GetSelf(), exclusive);
  v->rep.PinSelf();
  if (!s.ok()) {
    delete (v);
    if (!s.IsNotFound()) {
      SaveError(errptr, s);
    }
    return nullptr;
  }
  return v;
}

rocksdb_pinnableslice_t* rocksdb_transaction_get_pinned_for_update_cf(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    rocksdb_column_family_handle_t* column_family, const char* key, size_t klen,
    unsigned char exclusive, char** errptr) {
  rocksdb_pinnableslice_t* v = new (rocksdb_pinnableslice_t);
  Status s = txn->rep->GetForUpdate(options->rep, column_family->rep, Slice(key, klen), &v->rep, exclusive);
  if (!s.ok()) {
    delete (v);
    if (!s.IsNotFound()) {
      SaveError(errptr, s);
    }
    return nullptr;
  }
  return v;
}

void rocksdb_transaction_options_set_skip_prepare(
    rocksdb_transaction_options_t* opt, unsigned char v) {
  opt->rep.skip_prepare = v;
}

rocksdb_transaction_t** rocksdb_transactiondb_get_prepared_transactions(
    rocksdb_transactiondb_t* txn_db, size_t* cnt) {
  std::vector<Transaction*> txns;
  txn_db->rep->GetAllPreparedTransactions(&txns);
  *cnt = txns.size();
  if (txns.empty()) {
    return nullptr;
  } else {
    rocksdb_transaction_t** buf = 
      (rocksdb_transaction_t**)malloc(txns.size() * sizeof(rocksdb_transaction_t*));
    for (size_t i = 0; i < txns.size(); i++) {
      buf[i] = new rocksdb_transaction_t;
      buf[i]->rep = txns[i];
    }
    return buf;
  }
}

void rocksdb_transaction_set_name(rocksdb_transaction_t* txn, const char* name,
                                  size_t name_len, char** errptr) {
  std::string str = std::string(name, name_len);
  SaveError(errptr, txn->rep->SetName(str));
}

char* rocksdb_transaction_get_name(rocksdb_transaction_t* txn, size_t* name_len) {
  auto name = txn->rep->GetName();
  *name_len = name.size();
  return CopyString(name);
}

void rocksdb_transaction_prepare(rocksdb_transaction_t* txn, char** errptr) {
  SaveError(errptr, txn->rep->Prepare());
}

rocksdb_writebatch_wi_t* rocksdb_transaction_get_writebatch_wi(
    rocksdb_transaction_t* txn) {
  rocksdb_writebatch_wi_t *wi = (rocksdb_writebatch_wi_t*)malloc(sizeof(rocksdb_writebatch_wi_t));
  wi->rep = txn->rep->GetWriteBatch();

  return wi;
}

void rocksdb_transaction_rebuild_from_writebatch(
    rocksdb_transaction_t* txn, rocksdb_writebatch_t *writebatch, char** errptr) {
  SaveError(errptr, txn->rep->RebuildFromWriteBatch(&writebatch->rep));
}

void rocksdb_transaction_rebuild_from_writebatch_wi(
    rocksdb_transaction_t* txn, rocksdb_writebatch_wi_t *wi, char** errptr) {
  SaveError(errptr, txn->rep->RebuildFromWriteBatch(wi->rep->GetWriteBatch()));
}

void rocksdb_transaction_multi_get(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options, size_t num_keys,
    const char* const* keys_list, const size_t* keys_list_sizes, char** values_list,
    size_t* values_list_sizes, char** errs) {
  std::vector<Slice> keys(num_keys);
  for (size_t i = 0; i < num_keys; i++) {
    keys[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  std::vector<std::string> values(num_keys);
  std::vector<Status> statuses = txn->rep->MultiGet(options->rep, keys, &values);
  for (size_t i = 0; i < num_keys; i++) {
    if (statuses[i].ok()) {
      values_list[i] = CopyString(values[i]);
      values_list_sizes[i] = values[i].size();
      errs[i] = nullptr;
    } else {
      values_list[i] = nullptr;
      values_list_sizes[i] = 0;
      if (!statuses[i].IsNotFound()) {
        errs[i] = strdup(statuses[i].ToString().c_str());
      } else {
        errs[i] = nullptr;
      }
    }
  }
}

void rocksdb_transaction_multi_get_cf(
    rocksdb_transaction_t* txn, const rocksdb_readoptions_t* options,
    const rocksdb_column_family_handle_t* const* column_families,
    size_t num_keys, const char* const* keys_list, const size_t* keys_list_sizes,
    char** values_list, size_t* values_list_sizes, char** errs) {
  std::vector<Slice> keys(num_keys);
  std::vector<ColumnFamilyHandle*> cfs(num_keys);
  for (size_t i = 0; i < num_keys; i++) {
    keys[i] = Slice(keys_list[i], keys_list_sizes[i]);
    cfs[i] = column_families[i]->rep;
  }
  std::vector<std::string> values(num_keys);
  std::vector<Status> statuses = txn->rep->MultiGet(options->rep, cfs, keys, &values);
  for (size_t i = 0; i < num_keys; i++) {
    if (statuses[i].ok()) {
      values_list[i] = CopyString(values[i]);
      values_list_sizes[i] = values[i].size();
      errs[i] = nullptr;
    } else {
      values_list[i] = nullptr;
      values_list_sizes[i] = 0;
      if (!statuses[i].IsNotFound()) {
        errs[i] = strdup(statuses[i].ToString().c_str());
      } else {
        errs[i] = nullptr;
      }
    }
  }
}

void rocksdb_transactiondb_multi_get(
    rocksdb_transactiondb_t* txn_db, const rocksdb_readoptions_t* options, size_t num_keys,
    const char* const* keys_list, const size_t* keys_list_sizes, char** values_list,
    size_t* values_list_sizes, char** errs) {
  std::vector<Slice> keys(num_keys);
  for (size_t i = 0; i < num_keys; i++) {
    keys[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  std::vector<std::string> values(num_keys);
  std::vector<Status> statuses = txn_db->rep->MultiGet(options->rep, keys, &values);
  for (size_t i = 0; i < num_keys; i++) {
    if (statuses[i].ok()) {
      values_list[i] = CopyString(values[i]);
      values_list_sizes[i] = values[i].size();
      errs[i] = nullptr;
    } else {
      values_list[i] = nullptr;
      values_list_sizes[i] = 0;
      if (!statuses[i].IsNotFound()) {
        errs[i] = strdup(statuses[i].ToString().c_str());
      } else {
        errs[i] = nullptr;
      }
    }
  }
}

void rocksdb_transactiondb_multi_get_cf(
    rocksdb_transactiondb_t* txn_db, const rocksdb_readoptions_t* options,
    const rocksdb_column_family_handle_t* const* column_families,
    size_t num_keys, const char* const* keys_list, const size_t* keys_list_sizes,
    char** values_list, size_t* values_list_sizes, char** errs) {
  std::vector<Slice> keys(num_keys);
  std::vector<ColumnFamilyHandle*> cfs(num_keys);
  for (size_t i = 0; i < num_keys; i++) {
    keys[i] = Slice(keys_list[i], keys_list_sizes[i]);
    cfs[i] = column_families[i]->rep;
  }
  std::vector<std::string> values(num_keys);
  std::vector<Status> statuses = txn_db->rep->MultiGet(options->rep, cfs, keys, &values);
  for (size_t i = 0; i < num_keys; i++) {
    if (statuses[i].ok()) {
      values_list[i] = CopyString(values[i]);
      values_list_sizes[i] = values[i].size();
      errs[i] = nullptr;
    } else {
      values_list[i] = nullptr;
      values_list_sizes[i] = 0;
      if (!statuses[i].IsNotFound()) {
        errs[i] = strdup(statuses[i].ToString().c_str());
      } else {
        errs[i] = nullptr;
      }
    }
  }
}

}   // C