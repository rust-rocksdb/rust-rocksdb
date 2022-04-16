#include "rocksdb-c.h"

// Include origin rocksdb C-API.
#include "rocksdb/db/c.cc"

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

}   // C