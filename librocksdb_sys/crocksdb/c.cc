//  Copyright (c) 2011-present, Facebook, Inc.  All rights reserved.
//  This source code is licensed under the BSD-style license found in the
//  LICENSE file in the root directory of this source tree. An additional grant
//  of patent rights can be found in the PATENTS file in the same directory.
//
// Copyright (c) 2011 The LevelDB Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

#include "rocksdb/c.h"

#include <stdlib.h>
#include "rocksdb/cache.h"
#include "rocksdb/compaction_filter.h"
#include "rocksdb/comparator.h"
#include "rocksdb/convenience.h"
#include "rocksdb/db.h"
#include "rocksdb/env.h"
#include "rocksdb/filter_policy.h"
#include "rocksdb/iterator.h"
#include "rocksdb/merge_operator.h"
#include "rocksdb/options.h"
#include "rocksdb/status.h"
#include "rocksdb/write_batch.h"
#include "rocksdb/memtablerep.h"
#include "rocksdb/universal_compaction.h"
#include "rocksdb/statistics.h"
#include "rocksdb/slice_transform.h"
#include "rocksdb/table.h"
#include "rocksdb/rate_limiter.h"
#include "rocksdb/utilities/backupable_db.h"

using rocksdb::Cache;
using rocksdb::ColumnFamilyDescriptor;
using rocksdb::ColumnFamilyHandle;
using rocksdb::ColumnFamilyOptions;
using rocksdb::CompactionFilter;
using rocksdb::CompactionFilterFactory;
using rocksdb::CompactionFilterContext;
using rocksdb::CompactionOptionsFIFO;
using rocksdb::Comparator;
using rocksdb::CompressionType;
using rocksdb::WALRecoveryMode;
using rocksdb::DB;
using rocksdb::DBOptions;
using rocksdb::Env;
using rocksdb::EnvOptions;
using rocksdb::InfoLogLevel;
using rocksdb::FileLock;
using rocksdb::FilterPolicy;
using rocksdb::FlushOptions;
using rocksdb::IngestExternalFileOptions;
using rocksdb::Iterator;
using rocksdb::Logger;
using rocksdb::MergeOperator;
using rocksdb::NewBloomFilterPolicy;
using rocksdb::NewLRUCache;
using rocksdb::Options;
using rocksdb::BlockBasedTableOptions;
using rocksdb::CuckooTableOptions;
using rocksdb::RandomAccessFile;
using rocksdb::Range;
using rocksdb::ReadOptions;
using rocksdb::SequentialFile;
using rocksdb::Slice;
using rocksdb::SliceParts;
using rocksdb::SliceTransform;
using rocksdb::Snapshot;
using rocksdb::SstFileWriter;
using rocksdb::Status;
using rocksdb::WritableFile;
using rocksdb::WriteBatch;
using rocksdb::WriteOptions;
using rocksdb::LiveFileMetaData;
using rocksdb::BackupEngine;
using rocksdb::BackupableDBOptions;
using rocksdb::BackupInfo;
using rocksdb::RestoreOptions;
using rocksdb::CompactRangeOptions;
using rocksdb::RateLimiter;
using rocksdb::NewGenericRateLimiter;
using rocksdb::HistogramData;

using std::shared_ptr;

extern "C" {

struct crocksdb_t                 { DB*               rep; };
struct crocksdb_backup_engine_t   { BackupEngine*     rep; };
struct crocksdb_backup_engine_info_t { std::vector<BackupInfo> rep; };
struct crocksdb_restore_options_t { RestoreOptions rep; };
struct crocksdb_iterator_t        { Iterator*         rep; };
struct crocksdb_writebatch_t      { WriteBatch        rep; };
struct crocksdb_snapshot_t        { const Snapshot*   rep; };
struct crocksdb_flushoptions_t    { FlushOptions      rep; };
struct crocksdb_fifo_compaction_options_t { CompactionOptionsFIFO rep; };
struct crocksdb_readoptions_t {
   ReadOptions rep;
   Slice upper_bound; // stack variable to set pointer to in ReadOptions
};
struct crocksdb_writeoptions_t    { WriteOptions      rep; };
struct crocksdb_options_t         { Options           rep; };
struct crocksdb_compactoptions_t {
  CompactRangeOptions rep;
};
struct crocksdb_block_based_table_options_t  { BlockBasedTableOptions rep; };
struct crocksdb_cuckoo_table_options_t  { CuckooTableOptions rep; };
struct crocksdb_seqfile_t         { SequentialFile*   rep; };
struct crocksdb_randomfile_t      { RandomAccessFile* rep; };
struct crocksdb_writablefile_t    { WritableFile*     rep; };
struct crocksdb_filelock_t        { FileLock*         rep; };
struct crocksdb_logger_t          { shared_ptr<Logger>  rep; };
struct crocksdb_cache_t           { shared_ptr<Cache>   rep; };
struct crocksdb_livefiles_t       { std::vector<LiveFileMetaData> rep; };
struct crocksdb_column_family_handle_t  { ColumnFamilyHandle* rep; };
struct crocksdb_envoptions_t      { EnvOptions        rep; };
struct crocksdb_ingestexternalfileoptions_t  { IngestExternalFileOptions rep; };
struct crocksdb_sstfilewriter_t   { SstFileWriter*    rep; };
struct crocksdb_ratelimiter_t     { RateLimiter*      rep; };
struct crocksdb_histogramdata_t   { HistogramData     rep; };

struct crocksdb_compactionfiltercontext_t {
  CompactionFilter::Context rep;
};

struct crocksdb_compactionfilter_t : public CompactionFilter {
  void* state_;
  void (*destructor_)(void*);
  unsigned char (*filter_)(
      void*,
      int level,
      const char* key, size_t key_length,
      const char* existing_value, size_t value_length,
      char** new_value, size_t *new_value_length,
      unsigned char* value_changed);
  const char* (*name_)(void*);
  unsigned char ignore_snapshots_;

  virtual ~crocksdb_compactionfilter_t() {
    (*destructor_)(state_);
  }

  virtual bool Filter(int level, const Slice& key, const Slice& existing_value,
                      std::string* new_value,
                      bool* value_changed) const override {
    char* c_new_value = nullptr;
    size_t new_value_length = 0;
    unsigned char c_value_changed = 0;
    unsigned char result = (*filter_)(
        state_,
        level,
        key.data(), key.size(),
        existing_value.data(), existing_value.size(),
        &c_new_value, &new_value_length, &c_value_changed);
    if (c_value_changed) {
      new_value->assign(c_new_value, new_value_length);
      *value_changed = true;
    }
    return result;
  }

  virtual const char* Name() const override { return (*name_)(state_); }

  virtual bool IgnoreSnapshots() const override { return ignore_snapshots_; }
};

struct crocksdb_compactionfilterfactory_t : public CompactionFilterFactory {
  void* state_;
  void (*destructor_)(void*);
  crocksdb_compactionfilter_t* (*create_compaction_filter_)(
      void*, crocksdb_compactionfiltercontext_t* context);
  const char* (*name_)(void*);

  virtual ~crocksdb_compactionfilterfactory_t() { (*destructor_)(state_); }

  virtual std::unique_ptr<CompactionFilter> CreateCompactionFilter(
      const CompactionFilter::Context& context) override {
    crocksdb_compactionfiltercontext_t ccontext;
    ccontext.rep = context;
    CompactionFilter* cf = (*create_compaction_filter_)(state_, &ccontext);
    return std::unique_ptr<CompactionFilter>(cf);
  }

  virtual const char* Name() const override { return (*name_)(state_); }
};

struct crocksdb_comparator_t : public Comparator {
  void* state_;
  void (*destructor_)(void*);
  int (*compare_)(
      void*,
      const char* a, size_t alen,
      const char* b, size_t blen);
  const char* (*name_)(void*);

  virtual ~crocksdb_comparator_t() {
    (*destructor_)(state_);
  }

  virtual int Compare(const Slice& a, const Slice& b) const override {
    return (*compare_)(state_, a.data(), a.size(), b.data(), b.size());
  }

  virtual const char* Name() const override { return (*name_)(state_); }

  // No-ops since the C binding does not support key shortening methods.
  virtual void FindShortestSeparator(std::string*,
                                     const Slice&) const override {}
  virtual void FindShortSuccessor(std::string* key) const override {}
};

struct crocksdb_filterpolicy_t : public FilterPolicy {
  void* state_;
  void (*destructor_)(void*);
  const char* (*name_)(void*);
  char* (*create_)(
      void*,
      const char* const* key_array, const size_t* key_length_array,
      int num_keys,
      size_t* filter_length);
  unsigned char (*key_match_)(
      void*,
      const char* key, size_t length,
      const char* filter, size_t filter_length);
  void (*delete_filter_)(
      void*,
      const char* filter, size_t filter_length);

  virtual ~crocksdb_filterpolicy_t() {
    (*destructor_)(state_);
  }

  virtual const char* Name() const override { return (*name_)(state_); }

  virtual void CreateFilter(const Slice* keys, int n,
                            std::string* dst) const override {
    std::vector<const char*> key_pointers(n);
    std::vector<size_t> key_sizes(n);
    for (int i = 0; i < n; i++) {
      key_pointers[i] = keys[i].data();
      key_sizes[i] = keys[i].size();
    }
    size_t len;
    char* filter = (*create_)(state_, &key_pointers[0], &key_sizes[0], n, &len);
    dst->append(filter, len);

    if (delete_filter_ != nullptr) {
      (*delete_filter_)(state_, filter, len);
    } else {
      free(filter);
    }
  }

  virtual bool KeyMayMatch(const Slice& key,
                           const Slice& filter) const override {
    return (*key_match_)(state_, key.data(), key.size(),
                         filter.data(), filter.size());
  }
};

struct crocksdb_mergeoperator_t : public MergeOperator {
  void* state_;
  void (*destructor_)(void*);
  const char* (*name_)(void*);
  char* (*full_merge_)(
      void*,
      const char* key, size_t key_length,
      const char* existing_value, size_t existing_value_length,
      const char* const* operands_list, const size_t* operands_list_length,
      int num_operands,
      unsigned char* success, size_t* new_value_length);
  char* (*partial_merge_)(void*, const char* key, size_t key_length,
                          const char* const* operands_list,
                          const size_t* operands_list_length, int num_operands,
                          unsigned char* success, size_t* new_value_length);
  void (*delete_value_)(
      void*,
      const char* value, size_t value_length);

  virtual ~crocksdb_mergeoperator_t() {
    (*destructor_)(state_);
  }

  virtual const char* Name() const override { return (*name_)(state_); }

  virtual bool FullMergeV2(const MergeOperationInput& merge_in,
                           MergeOperationOutput* merge_out) const override {
    size_t n = merge_in.operand_list.size();
    std::vector<const char*> operand_pointers(n);
    std::vector<size_t> operand_sizes(n);
    for (size_t i = 0; i < n; i++) {
      Slice operand(merge_in.operand_list[i]);
      operand_pointers[i] = operand.data();
      operand_sizes[i] = operand.size();
    }

    const char* existing_value_data = nullptr;
    size_t existing_value_len = 0;
    if (merge_in.existing_value != nullptr) {
      existing_value_data = merge_in.existing_value->data();
      existing_value_len = merge_in.existing_value->size();
    }

    unsigned char success;
    size_t new_value_len;
    char* tmp_new_value = (*full_merge_)(
        state_, merge_in.key.data(), merge_in.key.size(), existing_value_data,
        existing_value_len, &operand_pointers[0], &operand_sizes[0],
        static_cast<int>(n), &success, &new_value_len);
    merge_out->new_value.assign(tmp_new_value, new_value_len);

    if (delete_value_ != nullptr) {
      (*delete_value_)(state_, tmp_new_value, new_value_len);
    } else {
      free(tmp_new_value);
    }

    return success;
  }

  virtual bool PartialMergeMulti(const Slice& key,
                                 const std::deque<Slice>& operand_list,
                                 std::string* new_value,
                                 Logger* logger) const override {
    size_t operand_count = operand_list.size();
    std::vector<const char*> operand_pointers(operand_count);
    std::vector<size_t> operand_sizes(operand_count);
    for (size_t i = 0; i < operand_count; ++i) {
      Slice operand(operand_list[i]);
      operand_pointers[i] = operand.data();
      operand_sizes[i] = operand.size();
    }

    unsigned char success;
    size_t new_value_len;
    char* tmp_new_value = (*partial_merge_)(
        state_, key.data(), key.size(), &operand_pointers[0], &operand_sizes[0],
        static_cast<int>(operand_count), &success, &new_value_len);
    new_value->assign(tmp_new_value, new_value_len);

    if (delete_value_ != nullptr) {
      (*delete_value_)(state_, tmp_new_value, new_value_len);
    } else {
      free(tmp_new_value);
    }

    return success;
  }
};

struct crocksdb_env_t {
  Env* rep;
  bool is_default;
};

struct crocksdb_slicetransform_t : public SliceTransform {
  void* state_;
  void (*destructor_)(void*);
  const char* (*name_)(void*);
  char* (*transform_)(
      void*,
      const char* key, size_t length,
      size_t* dst_length);
  unsigned char (*in_domain_)(
      void*,
      const char* key, size_t length);
  unsigned char (*in_range_)(
      void*,
      const char* key, size_t length);

  virtual ~crocksdb_slicetransform_t() {
    (*destructor_)(state_);
  }

  virtual const char* Name() const override { return (*name_)(state_); }

  virtual Slice Transform(const Slice& src) const override {
    size_t len;
    char* dst = (*transform_)(state_, src.data(), src.size(), &len);
    return Slice(dst, len);
  }

  virtual bool InDomain(const Slice& src) const override {
    return (*in_domain_)(state_, src.data(), src.size());
  }

  virtual bool InRange(const Slice& src) const override {
    return (*in_range_)(state_, src.data(), src.size());
  }
};

struct crocksdb_universal_compaction_options_t {
  rocksdb::CompactionOptionsUniversal *rep;
};

static bool SaveError(char** errptr, const Status& s) {
  assert(errptr != nullptr);
  if (s.ok()) {
    return false;
  } else if (*errptr == nullptr) {
    *errptr = strdup(s.ToString().c_str());
  } else {
    // TODO(sanjay): Merge with existing error?
    // This is a bug if *errptr is not created by malloc()
    free(*errptr);
    *errptr = strdup(s.ToString().c_str());
  }
  return true;
}

static char* CopyString(const std::string& str) {
  char* result = reinterpret_cast<char*>(malloc(sizeof(char) * str.size()));
  memcpy(result, str.data(), sizeof(char) * str.size());
  return result;
}

crocksdb_t* crocksdb_open(
    const crocksdb_options_t* options,
    const char* name,
    char** errptr) {
  DB* db;
  if (SaveError(errptr, DB::Open(options->rep, std::string(name), &db))) {
    return nullptr;
  }
  crocksdb_t* result = new crocksdb_t;
  result->rep = db;
  return result;
}

crocksdb_t* crocksdb_open_for_read_only(
    const crocksdb_options_t* options,
    const char* name,
    unsigned char error_if_log_file_exist,
    char** errptr) {
  DB* db;
  if (SaveError(errptr, DB::OpenForReadOnly(options->rep, std::string(name), &db, error_if_log_file_exist))) {
    return nullptr;
  }
  crocksdb_t* result = new crocksdb_t;
  result->rep = db;
  return result;
}

crocksdb_backup_engine_t* crocksdb_backup_engine_open(
    const crocksdb_options_t* options, const char* path, char** errptr) {
  BackupEngine* be;
  if (SaveError(errptr, BackupEngine::Open(options->rep.env,
                                           BackupableDBOptions(path), &be))) {
    return nullptr;
  }
  crocksdb_backup_engine_t* result = new crocksdb_backup_engine_t;
  result->rep = be;
  return result;
}

void crocksdb_backup_engine_create_new_backup(crocksdb_backup_engine_t* be,
                                             crocksdb_t* db, char** errptr) {
  SaveError(errptr, be->rep->CreateNewBackup(db->rep));
}

void crocksdb_backup_engine_purge_old_backups(crocksdb_backup_engine_t* be,
                                             uint32_t num_backups_to_keep,
                                             char** errptr) {
  SaveError(errptr, be->rep->PurgeOldBackups(num_backups_to_keep));
}

crocksdb_restore_options_t* crocksdb_restore_options_create() {
  return new crocksdb_restore_options_t;
}

void crocksdb_restore_options_destroy(crocksdb_restore_options_t* opt) {
  delete opt;
}

void crocksdb_restore_options_set_keep_log_files(crocksdb_restore_options_t* opt,
                                                int v) {
  opt->rep.keep_log_files = v;
}

void crocksdb_backup_engine_restore_db_from_latest_backup(
    crocksdb_backup_engine_t* be, const char* db_dir, const char* wal_dir,
    const crocksdb_restore_options_t* restore_options, char** errptr) {
  SaveError(errptr, be->rep->RestoreDBFromLatestBackup(std::string(db_dir),
                                                       std::string(wal_dir),
                                                       restore_options->rep));
}

const crocksdb_backup_engine_info_t* crocksdb_backup_engine_get_backup_info(
    crocksdb_backup_engine_t* be) {
  crocksdb_backup_engine_info_t* result = new crocksdb_backup_engine_info_t;
  be->rep->GetBackupInfo(&result->rep);
  return result;
}

int crocksdb_backup_engine_info_count(const crocksdb_backup_engine_info_t* info) {
  return static_cast<int>(info->rep.size());
}

int64_t crocksdb_backup_engine_info_timestamp(
    const crocksdb_backup_engine_info_t* info, int index) {
  return info->rep[index].timestamp;
}

uint32_t crocksdb_backup_engine_info_backup_id(
    const crocksdb_backup_engine_info_t* info, int index) {
  return info->rep[index].backup_id;
}

uint64_t crocksdb_backup_engine_info_size(
    const crocksdb_backup_engine_info_t* info, int index) {
  return info->rep[index].size;
}

uint32_t crocksdb_backup_engine_info_number_files(
    const crocksdb_backup_engine_info_t* info, int index) {
  return info->rep[index].number_files;
}

void crocksdb_backup_engine_info_destroy(
    const crocksdb_backup_engine_info_t* info) {
  delete info;
}

void crocksdb_backup_engine_close(crocksdb_backup_engine_t* be) {
  delete be->rep;
  delete be;
}

void crocksdb_close(crocksdb_t* db) {
  delete db->rep;
  delete db;
}

void crocksdb_pause_bg_work(crocksdb_t* db) {
  db->rep->PauseBackgroundWork();
}

void crocksdb_continue_bg_work(crocksdb_t* db) {
  db->rep->ContinueBackgroundWork();
}

crocksdb_t* crocksdb_open_column_families(
    const crocksdb_options_t* db_options,
    const char* name,
    int num_column_families,
    const char** column_family_names,
    const crocksdb_options_t** column_family_options,
    crocksdb_column_family_handle_t** column_family_handles,
    char** errptr) {
  std::vector<ColumnFamilyDescriptor> column_families;
  for (int i = 0; i < num_column_families; i++) {
    column_families.push_back(ColumnFamilyDescriptor(
        std::string(column_family_names[i]),
        ColumnFamilyOptions(column_family_options[i]->rep)));
  }

  DB* db;
  std::vector<ColumnFamilyHandle*> handles;
  if (SaveError(errptr, DB::Open(DBOptions(db_options->rep),
          std::string(name), column_families, &handles, &db))) {
    return nullptr;
  }

  for (size_t i = 0; i < handles.size(); i++) {
    crocksdb_column_family_handle_t* c_handle = new crocksdb_column_family_handle_t;
    c_handle->rep = handles[i];
    column_family_handles[i] = c_handle;
  }
  crocksdb_t* result = new crocksdb_t;
  result->rep = db;
  return result;
}

crocksdb_t* crocksdb_open_for_read_only_column_families(
    const crocksdb_options_t* db_options,
    const char* name,
    int num_column_families,
    const char** column_family_names,
    const crocksdb_options_t** column_family_options,
    crocksdb_column_family_handle_t** column_family_handles,
    unsigned char error_if_log_file_exist,
    char** errptr) {
  std::vector<ColumnFamilyDescriptor> column_families;
  for (int i = 0; i < num_column_families; i++) {
    column_families.push_back(ColumnFamilyDescriptor(
        std::string(column_family_names[i]),
        ColumnFamilyOptions(column_family_options[i]->rep)));
  }

  DB* db;
  std::vector<ColumnFamilyHandle*> handles;
  if (SaveError(errptr, DB::OpenForReadOnly(DBOptions(db_options->rep),
          std::string(name), column_families, &handles, &db, error_if_log_file_exist))) {
    return nullptr;
  }

  for (size_t i = 0; i < handles.size(); i++) {
    crocksdb_column_family_handle_t* c_handle = new crocksdb_column_family_handle_t;
    c_handle->rep = handles[i];
    column_family_handles[i] = c_handle;
  }
  crocksdb_t* result = new crocksdb_t;
  result->rep = db;
  return result;
}

char** crocksdb_list_column_families(
    const crocksdb_options_t* options,
    const char* name,
    size_t* lencfs,
    char** errptr) {
  std::vector<std::string> fams;
  SaveError(errptr,
      DB::ListColumnFamilies(DBOptions(options->rep),
        std::string(name), &fams));

  *lencfs = fams.size();
  char** column_families = static_cast<char**>(malloc(sizeof(char*) * fams.size()));
  for (size_t i = 0; i < fams.size(); i++) {
    column_families[i] = strdup(fams[i].c_str());
  }
  return column_families;
}

void crocksdb_list_column_families_destroy(char** list, size_t len) {
  for (size_t i = 0; i < len; ++i) {
    free(list[i]);
  }
  free(list);
}

crocksdb_column_family_handle_t* crocksdb_create_column_family(
    crocksdb_t* db,
    const crocksdb_options_t* column_family_options,
    const char* column_family_name,
    char** errptr) {
  crocksdb_column_family_handle_t* handle = new crocksdb_column_family_handle_t;
  SaveError(errptr,
      db->rep->CreateColumnFamily(ColumnFamilyOptions(column_family_options->rep),
        std::string(column_family_name), &(handle->rep)));
  return handle;
}

void crocksdb_drop_column_family(
    crocksdb_t* db,
    crocksdb_column_family_handle_t* handle,
    char** errptr) {
  SaveError(errptr, db->rep->DropColumnFamily(handle->rep));
}

void crocksdb_column_family_handle_destroy(crocksdb_column_family_handle_t* handle) {
  delete handle->rep;
  delete handle;
}

void crocksdb_put(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    const char* key, size_t keylen,
    const char* val, size_t vallen,
    char** errptr) {
  SaveError(errptr,
            db->rep->Put(options->rep, Slice(key, keylen), Slice(val, vallen)));
}

void crocksdb_put_cf(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t keylen,
    const char* val, size_t vallen,
    char** errptr) {
  SaveError(errptr,
            db->rep->Put(options->rep, column_family->rep,
              Slice(key, keylen), Slice(val, vallen)));
}

void crocksdb_delete(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    const char* key, size_t keylen,
    char** errptr) {
  SaveError(errptr, db->rep->Delete(options->rep, Slice(key, keylen)));
}

void crocksdb_delete_cf(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t keylen,
    char** errptr) {
  SaveError(errptr, db->rep->Delete(options->rep, column_family->rep,
        Slice(key, keylen)));
}

void crocksdb_single_delete(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    const char* key, size_t keylen,
    char** errptr) {
  SaveError(errptr, db->rep->SingleDelete(options->rep, Slice(key, keylen)));
}

void crocksdb_single_delete_cf(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t keylen,
    char** errptr) {
  SaveError(errptr, db->rep->SingleDelete(options->rep, column_family->rep,
        Slice(key, keylen)));
}

void crocksdb_merge(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    const char* key, size_t keylen,
    const char* val, size_t vallen,
    char** errptr) {
  SaveError(errptr,
            db->rep->Merge(options->rep, Slice(key, keylen), Slice(val, vallen)));
}

void crocksdb_merge_cf(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t keylen,
    const char* val, size_t vallen,
    char** errptr) {
  SaveError(errptr,
            db->rep->Merge(options->rep, column_family->rep,
              Slice(key, keylen), Slice(val, vallen)));
}

void crocksdb_write(
    crocksdb_t* db,
    const crocksdb_writeoptions_t* options,
    crocksdb_writebatch_t* batch,
    char** errptr) {
  SaveError(errptr, db->rep->Write(options->rep, &batch->rep));
}

char* crocksdb_get(
    crocksdb_t* db,
    const crocksdb_readoptions_t* options,
    const char* key, size_t keylen,
    size_t* vallen,
    char** errptr) {
  char* result = nullptr;
  std::string tmp;
  Status s = db->rep->Get(options->rep, Slice(key, keylen), &tmp);
  if (s.ok()) {
    *vallen = tmp.size();
    result = CopyString(tmp);
  } else {
    *vallen = 0;
    if (!s.IsNotFound()) {
      SaveError(errptr, s);
    }
  }
  return result;
}

char* crocksdb_get_cf(
    crocksdb_t* db,
    const crocksdb_readoptions_t* options,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t keylen,
    size_t* vallen,
    char** errptr) {
  char* result = nullptr;
  std::string tmp;
  Status s = db->rep->Get(options->rep, column_family->rep,
      Slice(key, keylen), &tmp);
  if (s.ok()) {
    *vallen = tmp.size();
    result = CopyString(tmp);
  } else {
    *vallen = 0;
    if (!s.IsNotFound()) {
      SaveError(errptr, s);
    }
  }
  return result;
}

void crocksdb_multi_get(
    crocksdb_t* db,
    const crocksdb_readoptions_t* options,
    size_t num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes,
    char** values_list, size_t* values_list_sizes,
    char** errs) {
  std::vector<Slice> keys(num_keys);
  for (size_t i = 0; i < num_keys; i++) {
    keys[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  std::vector<std::string> values(num_keys);
  std::vector<Status> statuses = db->rep->MultiGet(options->rep, keys, &values);
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

void crocksdb_multi_get_cf(
    crocksdb_t* db,
    const crocksdb_readoptions_t* options,
    const crocksdb_column_family_handle_t* const* column_families,
    size_t num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes,
    char** values_list, size_t* values_list_sizes,
    char** errs) {
  std::vector<Slice> keys(num_keys);
  std::vector<ColumnFamilyHandle*> cfs(num_keys);
  for (size_t i = 0; i < num_keys; i++) {
    keys[i] = Slice(keys_list[i], keys_list_sizes[i]);
    cfs[i] = column_families[i]->rep;
  }
  std::vector<std::string> values(num_keys);
  std::vector<Status> statuses = db->rep->MultiGet(options->rep, cfs, keys, &values);
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

crocksdb_iterator_t* crocksdb_create_iterator(
    crocksdb_t* db,
    const crocksdb_readoptions_t* options) {
  crocksdb_iterator_t* result = new crocksdb_iterator_t;
  result->rep = db->rep->NewIterator(options->rep);
  return result;
}

crocksdb_iterator_t* crocksdb_create_iterator_cf(
    crocksdb_t* db,
    const crocksdb_readoptions_t* options,
    crocksdb_column_family_handle_t* column_family) {
  crocksdb_iterator_t* result = new crocksdb_iterator_t;
  result->rep = db->rep->NewIterator(options->rep, column_family->rep);
  return result;
}

void crocksdb_create_iterators(
    crocksdb_t *db,
    crocksdb_readoptions_t* opts,
    crocksdb_column_family_handle_t** column_families,
    crocksdb_iterator_t** iterators,
    size_t size,
    char** errptr) {
  std::vector<ColumnFamilyHandle*> column_families_vec;
  for (size_t i = 0; i < size; i++) {
    column_families_vec.push_back(column_families[i]->rep);
  }

  std::vector<Iterator*> res;
  Status status = db->rep->NewIterators(opts->rep, column_families_vec, &res);
  assert(res.size() == size);
  if (SaveError(errptr, status)) {
    return;
  }

  for (size_t i = 0; i < size; i++) {
    iterators[i] = new crocksdb_iterator_t;
    iterators[i]->rep = res[i];
  }
}

const crocksdb_snapshot_t* crocksdb_create_snapshot(
    crocksdb_t* db) {
  crocksdb_snapshot_t* result = new crocksdb_snapshot_t;
  result->rep = db->rep->GetSnapshot();
  return result;
}

void crocksdb_release_snapshot(
    crocksdb_t* db,
    const crocksdb_snapshot_t* snapshot) {
  db->rep->ReleaseSnapshot(snapshot->rep);
  delete snapshot;
}

char* crocksdb_property_value(
    crocksdb_t* db,
    const char* propname) {
  std::string tmp;
  if (db->rep->GetProperty(Slice(propname), &tmp)) {
    // We use strdup() since we expect human readable output.
    return strdup(tmp.c_str());
  } else {
    return nullptr;
  }
}

char* crocksdb_property_value_cf(
    crocksdb_t* db,
    crocksdb_column_family_handle_t* column_family,
    const char* propname) {
  std::string tmp;
  if (db->rep->GetProperty(column_family->rep, Slice(propname), &tmp)) {
    // We use strdup() since we expect human readable output.
    return strdup(tmp.c_str());
  } else {
    return nullptr;
  }
}

void crocksdb_approximate_sizes(
    crocksdb_t* db,
    int num_ranges,
    const char* const* range_start_key, const size_t* range_start_key_len,
    const char* const* range_limit_key, const size_t* range_limit_key_len,
    uint64_t* sizes) {
  Range* ranges = new Range[num_ranges];
  for (int i = 0; i < num_ranges; i++) {
    ranges[i].start = Slice(range_start_key[i], range_start_key_len[i]);
    ranges[i].limit = Slice(range_limit_key[i], range_limit_key_len[i]);
  }
  db->rep->GetApproximateSizes(ranges, num_ranges, sizes);
  delete[] ranges;
}

void crocksdb_approximate_sizes_cf(
    crocksdb_t* db,
    crocksdb_column_family_handle_t* column_family,
    int num_ranges,
    const char* const* range_start_key, const size_t* range_start_key_len,
    const char* const* range_limit_key, const size_t* range_limit_key_len,
    uint64_t* sizes) {
  Range* ranges = new Range[num_ranges];
  for (int i = 0; i < num_ranges; i++) {
    ranges[i].start = Slice(range_start_key[i], range_start_key_len[i]);
    ranges[i].limit = Slice(range_limit_key[i], range_limit_key_len[i]);
  }
  db->rep->GetApproximateSizes(column_family->rep, ranges, num_ranges, sizes);
  delete[] ranges;
}

void crocksdb_delete_file(
    crocksdb_t* db,
    const char* name) {
  db->rep->DeleteFile(name);
}

const crocksdb_livefiles_t* crocksdb_livefiles(
    crocksdb_t* db) {
  crocksdb_livefiles_t* result = new crocksdb_livefiles_t;
  db->rep->GetLiveFilesMetaData(&result->rep);
  return result;
}

void crocksdb_compact_range(
    crocksdb_t* db,
    const char* start_key, size_t start_key_len,
    const char* limit_key, size_t limit_key_len) {
  Slice a, b;
  db->rep->CompactRange(
      CompactRangeOptions(),
      // Pass nullptr Slice if corresponding "const char*" is nullptr
      (start_key ? (a = Slice(start_key, start_key_len), &a) : nullptr),
      (limit_key ? (b = Slice(limit_key, limit_key_len), &b) : nullptr));
}

void crocksdb_compact_range_cf(
    crocksdb_t* db,
    crocksdb_column_family_handle_t* column_family,
    const char* start_key, size_t start_key_len,
    const char* limit_key, size_t limit_key_len) {
  Slice a, b;
  db->rep->CompactRange(
      CompactRangeOptions(), column_family->rep,
      // Pass nullptr Slice if corresponding "const char*" is nullptr
      (start_key ? (a = Slice(start_key, start_key_len), &a) : nullptr),
      (limit_key ? (b = Slice(limit_key, limit_key_len), &b) : nullptr));
}

void crocksdb_compact_range_opt(crocksdb_t* db, crocksdb_compactoptions_t* opt,
                               const char* start_key, size_t start_key_len,
                               const char* limit_key, size_t limit_key_len) {
  Slice a, b;
  db->rep->CompactRange(
      opt->rep,
      // Pass nullptr Slice if corresponding "const char*" is nullptr
      (start_key ? (a = Slice(start_key, start_key_len), &a) : nullptr),
      (limit_key ? (b = Slice(limit_key, limit_key_len), &b) : nullptr));
}

void crocksdb_compact_range_cf_opt(crocksdb_t* db,
                                  crocksdb_column_family_handle_t* column_family,
                                  crocksdb_compactoptions_t* opt,
                                  const char* start_key, size_t start_key_len,
                                  const char* limit_key, size_t limit_key_len) {
  Slice a, b;
  db->rep->CompactRange(
      opt->rep, column_family->rep,
      // Pass nullptr Slice if corresponding "const char*" is nullptr
      (start_key ? (a = Slice(start_key, start_key_len), &a) : nullptr),
      (limit_key ? (b = Slice(limit_key, limit_key_len), &b) : nullptr));
}

void crocksdb_flush(
    crocksdb_t* db,
    const crocksdb_flushoptions_t* options,
    char** errptr) {
  SaveError(errptr, db->rep->Flush(options->rep));
}

void crocksdb_disable_file_deletions(
    crocksdb_t* db,
    char** errptr) {
  SaveError(errptr, db->rep->DisableFileDeletions());
}

void crocksdb_enable_file_deletions(
    crocksdb_t* db,
    unsigned char force,
    char** errptr) {
  SaveError(errptr, db->rep->EnableFileDeletions(force));
}

void crocksdb_destroy_db(
    const crocksdb_options_t* options,
    const char* name,
    char** errptr) {
  SaveError(errptr, DestroyDB(name, options->rep));
}

void crocksdb_repair_db(
    const crocksdb_options_t* options,
    const char* name,
    char** errptr) {
  SaveError(errptr, RepairDB(name, options->rep));
}

void crocksdb_iter_destroy(crocksdb_iterator_t* iter) {
  delete iter->rep;
  delete iter;
}

unsigned char crocksdb_iter_valid(const crocksdb_iterator_t* iter) {
  return iter->rep->Valid();
}

void crocksdb_iter_seek_to_first(crocksdb_iterator_t* iter) {
  iter->rep->SeekToFirst();
}

void crocksdb_iter_seek_to_last(crocksdb_iterator_t* iter) {
  iter->rep->SeekToLast();
}

void crocksdb_iter_seek(crocksdb_iterator_t* iter, const char* k, size_t klen) {
  iter->rep->Seek(Slice(k, klen));
}

void crocksdb_iter_seek_for_prev(crocksdb_iterator_t* iter, const char* k,
                                size_t klen) {
  iter->rep->SeekForPrev(Slice(k, klen));
}

void crocksdb_iter_next(crocksdb_iterator_t* iter) {
  iter->rep->Next();
}

void crocksdb_iter_prev(crocksdb_iterator_t* iter) {
  iter->rep->Prev();
}

const char* crocksdb_iter_key(const crocksdb_iterator_t* iter, size_t* klen) {
  Slice s = iter->rep->key();
  *klen = s.size();
  return s.data();
}

const char* crocksdb_iter_value(const crocksdb_iterator_t* iter, size_t* vlen) {
  Slice s = iter->rep->value();
  *vlen = s.size();
  return s.data();
}

void crocksdb_iter_get_error(const crocksdb_iterator_t* iter, char** errptr) {
  SaveError(errptr, iter->rep->status());
}

crocksdb_writebatch_t* crocksdb_writebatch_create() {
  return new crocksdb_writebatch_t;
}

crocksdb_writebatch_t* crocksdb_writebatch_create_from(const char* rep,
                                                     size_t size) {
  crocksdb_writebatch_t* b = new crocksdb_writebatch_t;
  b->rep = WriteBatch(std::string(rep, size));
  return b;
}

void crocksdb_writebatch_destroy(crocksdb_writebatch_t* b) {
  delete b;
}

void crocksdb_writebatch_clear(crocksdb_writebatch_t* b) {
  b->rep.Clear();
}

int crocksdb_writebatch_count(crocksdb_writebatch_t* b) {
  return b->rep.Count();
}

void crocksdb_writebatch_put(
    crocksdb_writebatch_t* b,
    const char* key, size_t klen,
    const char* val, size_t vlen) {
  b->rep.Put(Slice(key, klen), Slice(val, vlen));
}

void crocksdb_writebatch_put_cf(
    crocksdb_writebatch_t* b,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t klen,
    const char* val, size_t vlen) {
  b->rep.Put(column_family->rep, Slice(key, klen), Slice(val, vlen));
}

void crocksdb_writebatch_putv(
    crocksdb_writebatch_t* b,
    int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes,
    int num_values, const char* const* values_list,
    const size_t* values_list_sizes) {
  std::vector<Slice> key_slices(num_keys);
  for (int i = 0; i < num_keys; i++) {
    key_slices[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  std::vector<Slice> value_slices(num_values);
  for (int i = 0; i < num_values; i++) {
    value_slices[i] = Slice(values_list[i], values_list_sizes[i]);
  }
  b->rep.Put(SliceParts(key_slices.data(), num_keys),
             SliceParts(value_slices.data(), num_values));
}

void crocksdb_writebatch_putv_cf(
    crocksdb_writebatch_t* b,
    crocksdb_column_family_handle_t* column_family,
    int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes,
    int num_values, const char* const* values_list,
    const size_t* values_list_sizes) {
  std::vector<Slice> key_slices(num_keys);
  for (int i = 0; i < num_keys; i++) {
    key_slices[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  std::vector<Slice> value_slices(num_values);
  for (int i = 0; i < num_values; i++) {
    value_slices[i] = Slice(values_list[i], values_list_sizes[i]);
  }
  b->rep.Put(column_family->rep, SliceParts(key_slices.data(), num_keys),
             SliceParts(value_slices.data(), num_values));
}

void crocksdb_writebatch_merge(
    crocksdb_writebatch_t* b,
    const char* key, size_t klen,
    const char* val, size_t vlen) {
  b->rep.Merge(Slice(key, klen), Slice(val, vlen));
}

void crocksdb_writebatch_merge_cf(
    crocksdb_writebatch_t* b,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t klen,
    const char* val, size_t vlen) {
  b->rep.Merge(column_family->rep, Slice(key, klen), Slice(val, vlen));
}

void crocksdb_writebatch_mergev(
    crocksdb_writebatch_t* b,
    int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes,
    int num_values, const char* const* values_list,
    const size_t* values_list_sizes) {
  std::vector<Slice> key_slices(num_keys);
  for (int i = 0; i < num_keys; i++) {
    key_slices[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  std::vector<Slice> value_slices(num_values);
  for (int i = 0; i < num_values; i++) {
    value_slices[i] = Slice(values_list[i], values_list_sizes[i]);
  }
  b->rep.Merge(SliceParts(key_slices.data(), num_keys),
               SliceParts(value_slices.data(), num_values));
}

void crocksdb_writebatch_mergev_cf(
    crocksdb_writebatch_t* b,
    crocksdb_column_family_handle_t* column_family,
    int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes,
    int num_values, const char* const* values_list,
    const size_t* values_list_sizes) {
  std::vector<Slice> key_slices(num_keys);
  for (int i = 0; i < num_keys; i++) {
    key_slices[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  std::vector<Slice> value_slices(num_values);
  for (int i = 0; i < num_values; i++) {
    value_slices[i] = Slice(values_list[i], values_list_sizes[i]);
  }
  b->rep.Merge(column_family->rep, SliceParts(key_slices.data(), num_keys),
               SliceParts(value_slices.data(), num_values));
}

void crocksdb_writebatch_delete(
    crocksdb_writebatch_t* b,
    const char* key, size_t klen) {
  b->rep.Delete(Slice(key, klen));
}

void crocksdb_writebatch_delete_cf(
    crocksdb_writebatch_t* b,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t klen) {
  b->rep.Delete(column_family->rep, Slice(key, klen));
}

void crocksdb_writebatch_single_delete(
    crocksdb_writebatch_t* b,
    const char* key, size_t klen) {
  b->rep.SingleDelete(Slice(key, klen));
}

void crocksdb_writebatch_single_delete_cf(
    crocksdb_writebatch_t* b,
    crocksdb_column_family_handle_t* column_family,
    const char* key, size_t klen) {
  b->rep.SingleDelete(column_family->rep, Slice(key, klen));
}

void crocksdb_writebatch_deletev(
    crocksdb_writebatch_t* b,
    int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes) {
  std::vector<Slice> key_slices(num_keys);
  for (int i = 0; i < num_keys; i++) {
    key_slices[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  b->rep.Delete(SliceParts(key_slices.data(), num_keys));
}

void crocksdb_writebatch_deletev_cf(
    crocksdb_writebatch_t* b,
    crocksdb_column_family_handle_t* column_family,
    int num_keys, const char* const* keys_list,
    const size_t* keys_list_sizes) {
  std::vector<Slice> key_slices(num_keys);
  for (int i = 0; i < num_keys; i++) {
    key_slices[i] = Slice(keys_list[i], keys_list_sizes[i]);
  }
  b->rep.Delete(column_family->rep, SliceParts(key_slices.data(), num_keys));
}

void crocksdb_writebatch_delete_range(crocksdb_writebatch_t* b,
                                     const char* start_key,
                                     size_t start_key_len, const char* end_key,
                                     size_t end_key_len) {
  b->rep.DeleteRange(Slice(start_key, start_key_len),
                     Slice(end_key, end_key_len));
}

void crocksdb_writebatch_delete_range_cf(
    crocksdb_writebatch_t* b, crocksdb_column_family_handle_t* column_family,
    const char* start_key, size_t start_key_len, const char* end_key,
    size_t end_key_len) {
  b->rep.DeleteRange(column_family->rep, Slice(start_key, start_key_len),
                     Slice(end_key, end_key_len));
}

void crocksdb_writebatch_delete_rangev(crocksdb_writebatch_t* b, int num_keys,
                                      const char* const* start_keys_list,
                                      const size_t* start_keys_list_sizes,
                                      const char* const* end_keys_list,
                                      const size_t* end_keys_list_sizes) {
  std::vector<Slice> start_key_slices(num_keys);
  std::vector<Slice> end_key_slices(num_keys);
  for (int i = 0; i < num_keys; i++) {
    start_key_slices[i] = Slice(start_keys_list[i], start_keys_list_sizes[i]);
    end_key_slices[i] = Slice(end_keys_list[i], end_keys_list_sizes[i]);
  }
  b->rep.DeleteRange(SliceParts(start_key_slices.data(), num_keys),
                     SliceParts(end_key_slices.data(), num_keys));
}

void crocksdb_writebatch_delete_rangev_cf(
    crocksdb_writebatch_t* b, crocksdb_column_family_handle_t* column_family,
    int num_keys, const char* const* start_keys_list,
    const size_t* start_keys_list_sizes, const char* const* end_keys_list,
    const size_t* end_keys_list_sizes) {
  std::vector<Slice> start_key_slices(num_keys);
  std::vector<Slice> end_key_slices(num_keys);
  for (int i = 0; i < num_keys; i++) {
    start_key_slices[i] = Slice(start_keys_list[i], start_keys_list_sizes[i]);
    end_key_slices[i] = Slice(end_keys_list[i], end_keys_list_sizes[i]);
  }
  b->rep.DeleteRange(column_family->rep,
                     SliceParts(start_key_slices.data(), num_keys),
                     SliceParts(end_key_slices.data(), num_keys));
}

void crocksdb_writebatch_put_log_data(
    crocksdb_writebatch_t* b,
    const char* blob, size_t len) {
  b->rep.PutLogData(Slice(blob, len));
}

void crocksdb_writebatch_iterate(
    crocksdb_writebatch_t* b,
    void* state,
    void (*put)(void*, const char* k, size_t klen, const char* v, size_t vlen),
    void (*deleted)(void*, const char* k, size_t klen)) {
  class H : public WriteBatch::Handler {
   public:
    void* state_;
    void (*put_)(void*, const char* k, size_t klen, const char* v, size_t vlen);
    void (*deleted_)(void*, const char* k, size_t klen);
    virtual void Put(const Slice& key, const Slice& value) override {
      (*put_)(state_, key.data(), key.size(), value.data(), value.size());
    }
    virtual void Delete(const Slice& key) override {
      (*deleted_)(state_, key.data(), key.size());
    }
  };
  H handler;
  handler.state_ = state;
  handler.put_ = put;
  handler.deleted_ = deleted;
  b->rep.Iterate(&handler);
}

const char* crocksdb_writebatch_data(crocksdb_writebatch_t* b, size_t* size) {
  *size = b->rep.GetDataSize();
  return b->rep.Data().c_str();
}

void crocksdb_writebatch_set_save_point(crocksdb_writebatch_t* b) {
  b->rep.SetSavePoint();
}

void crocksdb_writebatch_rollback_to_save_point(crocksdb_writebatch_t* b, char** errptr) {
  SaveError(errptr, b->rep.RollbackToSavePoint());
}

crocksdb_block_based_table_options_t*
crocksdb_block_based_options_create() {
  return new crocksdb_block_based_table_options_t;
}

void crocksdb_block_based_options_destroy(
    crocksdb_block_based_table_options_t* options) {
  delete options;
}

void crocksdb_block_based_options_set_block_size(
    crocksdb_block_based_table_options_t* options, size_t block_size) {
  options->rep.block_size = block_size;
}

void crocksdb_block_based_options_set_block_size_deviation(
    crocksdb_block_based_table_options_t* options, int block_size_deviation) {
  options->rep.block_size_deviation = block_size_deviation;
}

void crocksdb_block_based_options_set_block_restart_interval(
    crocksdb_block_based_table_options_t* options, int block_restart_interval) {
  options->rep.block_restart_interval = block_restart_interval;
}

void crocksdb_block_based_options_set_filter_policy(
    crocksdb_block_based_table_options_t* options,
    crocksdb_filterpolicy_t* filter_policy) {
  options->rep.filter_policy.reset(filter_policy);
}

void crocksdb_block_based_options_set_no_block_cache(
    crocksdb_block_based_table_options_t* options,
    unsigned char no_block_cache) {
  options->rep.no_block_cache = no_block_cache;
}

void crocksdb_block_based_options_set_block_cache(
    crocksdb_block_based_table_options_t* options,
    crocksdb_cache_t* block_cache) {
  if (block_cache) {
    options->rep.block_cache = block_cache->rep;
  }
}

void crocksdb_block_based_options_set_block_cache_compressed(
    crocksdb_block_based_table_options_t* options,
    crocksdb_cache_t* block_cache_compressed) {
  if (block_cache_compressed) {
    options->rep.block_cache_compressed = block_cache_compressed->rep;
  }
}

void crocksdb_block_based_options_set_whole_key_filtering(
    crocksdb_block_based_table_options_t* options, unsigned char v) {
  options->rep.whole_key_filtering = v;
}

void crocksdb_block_based_options_set_format_version(
    crocksdb_block_based_table_options_t* options, int v) {
  options->rep.format_version = v;
}

void crocksdb_block_based_options_set_index_type(
    crocksdb_block_based_table_options_t* options, int v) {
  options->rep.index_type = static_cast<BlockBasedTableOptions::IndexType>(v);
}

void crocksdb_block_based_options_set_hash_index_allow_collision(
    crocksdb_block_based_table_options_t* options, unsigned char v) {
  options->rep.hash_index_allow_collision = v;
}

void crocksdb_block_based_options_set_cache_index_and_filter_blocks(
    crocksdb_block_based_table_options_t* options, unsigned char v) {
  options->rep.cache_index_and_filter_blocks = v;
}

void crocksdb_block_based_options_set_pin_l0_filter_and_index_blocks_in_cache(
    crocksdb_block_based_table_options_t* options, unsigned char v) {
  options->rep.pin_l0_filter_and_index_blocks_in_cache = v;
}

void crocksdb_block_based_options_set_skip_table_builder_flush(
    crocksdb_block_based_table_options_t* options, unsigned char v) {
  options->rep.skip_table_builder_flush = v;
}

void crocksdb_options_set_block_based_table_factory(
    crocksdb_options_t *opt,
    crocksdb_block_based_table_options_t* table_options) {
  if (table_options) {
    opt->rep.table_factory.reset(
        rocksdb::NewBlockBasedTableFactory(table_options->rep));
  }
}


crocksdb_cuckoo_table_options_t*
crocksdb_cuckoo_options_create() {
  return new crocksdb_cuckoo_table_options_t;
}

void crocksdb_cuckoo_options_destroy(
    crocksdb_cuckoo_table_options_t* options) {
  delete options;
}

void crocksdb_cuckoo_options_set_hash_ratio(
    crocksdb_cuckoo_table_options_t* options, double v) {
  options->rep.hash_table_ratio = v;
}

void crocksdb_cuckoo_options_set_max_search_depth(
    crocksdb_cuckoo_table_options_t* options, uint32_t v) {
  options->rep.max_search_depth = v;
}

void crocksdb_cuckoo_options_set_cuckoo_block_size(
    crocksdb_cuckoo_table_options_t* options, uint32_t v) {
  options->rep.cuckoo_block_size = v;
}

void crocksdb_cuckoo_options_set_identity_as_first_hash(
    crocksdb_cuckoo_table_options_t* options, unsigned char v) {
  options->rep.identity_as_first_hash = v;
}

void crocksdb_cuckoo_options_set_use_module_hash(
    crocksdb_cuckoo_table_options_t* options, unsigned char v) {
  options->rep.use_module_hash = v;
}

void crocksdb_options_set_cuckoo_table_factory(
    crocksdb_options_t *opt,
    crocksdb_cuckoo_table_options_t* table_options) {
  if (table_options) {
    opt->rep.table_factory.reset(
        rocksdb::NewCuckooTableFactory(table_options->rep));
  }
}


crocksdb_options_t* crocksdb_options_create() {
  return new crocksdb_options_t;
}

void crocksdb_options_destroy(crocksdb_options_t* options) {
  delete options;
}

void crocksdb_options_increase_parallelism(
    crocksdb_options_t* opt, int total_threads) {
  opt->rep.IncreaseParallelism(total_threads);
}

void crocksdb_options_optimize_for_point_lookup(
    crocksdb_options_t* opt, uint64_t block_cache_size_mb) {
  opt->rep.OptimizeForPointLookup(block_cache_size_mb);
}

void crocksdb_options_optimize_level_style_compaction(
    crocksdb_options_t* opt, uint64_t memtable_memory_budget) {
  opt->rep.OptimizeLevelStyleCompaction(memtable_memory_budget);
}

void crocksdb_options_optimize_universal_style_compaction(
    crocksdb_options_t* opt, uint64_t memtable_memory_budget) {
  opt->rep.OptimizeUniversalStyleCompaction(memtable_memory_budget);
}

void crocksdb_options_set_compaction_filter(
    crocksdb_options_t* opt,
    crocksdb_compactionfilter_t* filter) {
  opt->rep.compaction_filter = filter;
}

void crocksdb_options_set_compaction_filter_factory(
    crocksdb_options_t* opt, crocksdb_compactionfilterfactory_t* factory) {
  opt->rep.compaction_filter_factory =
      std::shared_ptr<CompactionFilterFactory>(factory);
}

void crocksdb_options_compaction_readahead_size(
    crocksdb_options_t* opt, size_t s) {
  opt->rep.compaction_readahead_size = s;
}

void crocksdb_options_set_comparator(
    crocksdb_options_t* opt,
    crocksdb_comparator_t* cmp) {
  opt->rep.comparator = cmp;
}

void crocksdb_options_set_merge_operator(
    crocksdb_options_t* opt,
    crocksdb_mergeoperator_t* merge_operator) {
  opt->rep.merge_operator = std::shared_ptr<MergeOperator>(merge_operator);
}


void crocksdb_options_set_create_if_missing(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.create_if_missing = v;
}

void crocksdb_options_set_create_missing_column_families(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.create_missing_column_families = v;
}

void crocksdb_options_set_error_if_exists(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.error_if_exists = v;
}

void crocksdb_options_set_paranoid_checks(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.paranoid_checks = v;
}

void crocksdb_options_set_env(crocksdb_options_t* opt, crocksdb_env_t* env) {
  opt->rep.env = (env ? env->rep : nullptr);
}

void crocksdb_options_set_info_log(crocksdb_options_t* opt, crocksdb_logger_t* l) {
  if (l) {
    opt->rep.info_log = l->rep;
  }
}

void crocksdb_options_set_info_log_level(
    crocksdb_options_t* opt, int v) {
  opt->rep.info_log_level = static_cast<InfoLogLevel>(v);
}

void crocksdb_options_set_db_write_buffer_size(crocksdb_options_t* opt,
                                              size_t s) {
  opt->rep.db_write_buffer_size = s;
}

void crocksdb_options_set_write_buffer_size(crocksdb_options_t* opt, size_t s) {
  opt->rep.write_buffer_size = s;
}

void crocksdb_options_set_max_open_files(crocksdb_options_t* opt, int n) {
  opt->rep.max_open_files = n;
}

void crocksdb_options_set_max_total_wal_size(crocksdb_options_t* opt, uint64_t n) {
  opt->rep.max_total_wal_size = n;
}

void crocksdb_options_set_target_file_size_base(
    crocksdb_options_t* opt, uint64_t n) {
  opt->rep.target_file_size_base = n;
}

void crocksdb_options_set_target_file_size_multiplier(
    crocksdb_options_t* opt, int n) {
  opt->rep.target_file_size_multiplier = n;
}

void crocksdb_options_set_max_bytes_for_level_base(
    crocksdb_options_t* opt, uint64_t n) {
  opt->rep.max_bytes_for_level_base = n;
}

void crocksdb_options_set_level_compaction_dynamic_level_bytes(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.level_compaction_dynamic_level_bytes = v;
}

void crocksdb_options_set_max_bytes_for_level_multiplier(crocksdb_options_t* opt,
                                                        double n) {
  opt->rep.max_bytes_for_level_multiplier = n;
}

void crocksdb_options_set_max_compaction_bytes(crocksdb_options_t* opt,
                                              uint64_t n) {
  opt->rep.max_compaction_bytes = n;
}

void crocksdb_options_set_max_bytes_for_level_multiplier_additional(
    crocksdb_options_t* opt, int* level_values, size_t num_levels) {
  opt->rep.max_bytes_for_level_multiplier_additional.resize(num_levels);
  for (size_t i = 0; i < num_levels; ++i) {
    opt->rep.max_bytes_for_level_multiplier_additional[i] = level_values[i];
  }
}

void crocksdb_options_enable_statistics(crocksdb_options_t* opt) {
  opt->rep.statistics = rocksdb::CreateDBStatistics();
}

void crocksdb_options_set_num_levels(crocksdb_options_t* opt, int n) {
  opt->rep.num_levels = n;
}

void crocksdb_options_set_level0_file_num_compaction_trigger(
    crocksdb_options_t* opt, int n) {
  opt->rep.level0_file_num_compaction_trigger = n;
}

void crocksdb_options_set_level0_slowdown_writes_trigger(
    crocksdb_options_t* opt, int n) {
  opt->rep.level0_slowdown_writes_trigger = n;
}

void crocksdb_options_set_level0_stop_writes_trigger(
    crocksdb_options_t* opt, int n) {
  opt->rep.level0_stop_writes_trigger = n;
}

void crocksdb_options_set_max_mem_compaction_level(crocksdb_options_t* opt,
                                                  int n) {}

void crocksdb_options_set_wal_recovery_mode(crocksdb_options_t* opt,int mode) {
  opt->rep.wal_recovery_mode = static_cast<WALRecoveryMode>(mode);
}

void crocksdb_options_set_compression(crocksdb_options_t* opt, int t) {
  opt->rep.compression = static_cast<CompressionType>(t);
}

void crocksdb_options_set_compression_per_level(crocksdb_options_t* opt,
                                               int* level_values,
                                               size_t num_levels) {
  opt->rep.compression_per_level.resize(num_levels);
  for (size_t i = 0; i < num_levels; ++i) {
    opt->rep.compression_per_level[i] =
      static_cast<CompressionType>(level_values[i]);
  }
}

void crocksdb_options_set_compression_options(crocksdb_options_t* opt, int w_bits,
                                             int level, int strategy,
                                             int max_dict_bytes) {
  opt->rep.compression_opts.window_bits = w_bits;
  opt->rep.compression_opts.level = level;
  opt->rep.compression_opts.strategy = strategy;
  opt->rep.compression_opts.max_dict_bytes = max_dict_bytes;
}

void crocksdb_options_set_prefix_extractor(
    crocksdb_options_t* opt, crocksdb_slicetransform_t* prefix_extractor) {
  opt->rep.prefix_extractor.reset(prefix_extractor);
}

void crocksdb_options_set_memtable_insert_with_hint_prefix_extractor(
    crocksdb_options_t* opt, crocksdb_slicetransform_t* prefix_extractor) {
  opt->rep.memtable_insert_with_hint_prefix_extractor.reset(prefix_extractor);
}

void crocksdb_options_set_disable_data_sync(
    crocksdb_options_t* opt, int disable_data_sync) {
  opt->rep.disableDataSync = disable_data_sync;
}

void crocksdb_options_set_use_fsync(
    crocksdb_options_t* opt, int use_fsync) {
  opt->rep.use_fsync = use_fsync;
}

void crocksdb_options_set_db_log_dir(
    crocksdb_options_t* opt, const char* db_log_dir) {
  opt->rep.db_log_dir = db_log_dir;
}

void crocksdb_options_set_wal_dir(
    crocksdb_options_t* opt, const char* v) {
  opt->rep.wal_dir = v;
}

void crocksdb_options_set_WAL_ttl_seconds(crocksdb_options_t* opt, uint64_t ttl) {
  opt->rep.WAL_ttl_seconds = ttl;
}

void crocksdb_options_set_WAL_size_limit_MB(
    crocksdb_options_t* opt, uint64_t limit) {
  opt->rep.WAL_size_limit_MB = limit;
}

void crocksdb_options_set_manifest_preallocation_size(
    crocksdb_options_t* opt, size_t v) {
  opt->rep.manifest_preallocation_size = v;
}

// noop
void crocksdb_options_set_purge_redundant_kvs_while_flush(crocksdb_options_t* opt,
                                                         unsigned char v) {}

void crocksdb_options_set_allow_mmap_reads(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.allow_mmap_reads = v;
}

void crocksdb_options_set_allow_mmap_writes(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.allow_mmap_writes = v;
}

void crocksdb_options_set_is_fd_close_on_exec(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.is_fd_close_on_exec = v;
}

void crocksdb_options_set_skip_log_error_on_recovery(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.skip_log_error_on_recovery = v;
}

void crocksdb_options_set_stats_dump_period_sec(
    crocksdb_options_t* opt, unsigned int v) {
  opt->rep.stats_dump_period_sec = v;
}

void crocksdb_options_set_advise_random_on_open(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.advise_random_on_open = v;
}

void crocksdb_options_set_access_hint_on_compaction_start(
    crocksdb_options_t* opt, int v) {
  switch(v) {
    case 0:
      opt->rep.access_hint_on_compaction_start = rocksdb::Options::NONE;
      break;
    case 1:
      opt->rep.access_hint_on_compaction_start = rocksdb::Options::NORMAL;
      break;
    case 2:
      opt->rep.access_hint_on_compaction_start = rocksdb::Options::SEQUENTIAL;
      break;
    case 3:
      opt->rep.access_hint_on_compaction_start = rocksdb::Options::WILLNEED;
      break;
  }
}

void crocksdb_options_set_use_adaptive_mutex(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.use_adaptive_mutex = v;
}

void crocksdb_options_set_bytes_per_sync(
    crocksdb_options_t* opt, uint64_t v) {
  opt->rep.bytes_per_sync = v;
}

void crocksdb_options_set_allow_concurrent_memtable_write(crocksdb_options_t* opt,
                                                         unsigned char v) {
  opt->rep.allow_concurrent_memtable_write = v;
}

void crocksdb_options_set_enable_write_thread_adaptive_yield(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.enable_write_thread_adaptive_yield = v;
}

void crocksdb_options_set_verify_checksums_in_compaction(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.verify_checksums_in_compaction = v;
}

void crocksdb_options_set_max_sequential_skip_in_iterations(
    crocksdb_options_t* opt, uint64_t v) {
  opt->rep.max_sequential_skip_in_iterations = v;
}

void crocksdb_options_set_max_write_buffer_number(crocksdb_options_t* opt, int n) {
  opt->rep.max_write_buffer_number = n;
}

void crocksdb_options_set_min_write_buffer_number_to_merge(crocksdb_options_t* opt, int n) {
  opt->rep.min_write_buffer_number_to_merge = n;
}

void crocksdb_options_set_max_write_buffer_number_to_maintain(
    crocksdb_options_t* opt, int n) {
  opt->rep.max_write_buffer_number_to_maintain = n;
}

void crocksdb_options_set_max_background_compactions(crocksdb_options_t* opt, int n) {
  opt->rep.max_background_compactions = n;
}

void crocksdb_options_set_base_background_compactions(crocksdb_options_t* opt,
                                                     int n) {
  opt->rep.base_background_compactions = n;
}

void crocksdb_options_set_max_background_flushes(crocksdb_options_t* opt, int n) {
  opt->rep.max_background_flushes = n;
}

void crocksdb_options_set_max_log_file_size(crocksdb_options_t* opt, size_t v) {
  opt->rep.max_log_file_size = v;
}

void crocksdb_options_set_log_file_time_to_roll(crocksdb_options_t* opt, size_t v) {
  opt->rep.log_file_time_to_roll = v;
}

void crocksdb_options_set_keep_log_file_num(crocksdb_options_t* opt, size_t v) {
  opt->rep.keep_log_file_num = v;
}

void crocksdb_options_set_recycle_log_file_num(crocksdb_options_t* opt,
                                              size_t v) {
  opt->rep.recycle_log_file_num = v;
}

void crocksdb_options_set_soft_rate_limit(crocksdb_options_t* opt, double v) {
  opt->rep.soft_rate_limit = v;
}

void crocksdb_options_set_hard_rate_limit(crocksdb_options_t* opt, double v) {
  opt->rep.hard_rate_limit = v;
}

void crocksdb_options_set_rate_limit_delay_max_milliseconds(
    crocksdb_options_t* opt, unsigned int v) {
  opt->rep.rate_limit_delay_max_milliseconds = v;
}

void crocksdb_options_set_max_manifest_file_size(
    crocksdb_options_t* opt, size_t v) {
  opt->rep.max_manifest_file_size = v;
}

void crocksdb_options_set_table_cache_numshardbits(
    crocksdb_options_t* opt, int v) {
  opt->rep.table_cache_numshardbits = v;
}

void crocksdb_options_set_table_cache_remove_scan_count_limit(
    crocksdb_options_t* opt, int v) {
  // this option is deprecated
}

void crocksdb_options_set_arena_block_size(
    crocksdb_options_t* opt, size_t v) {
  opt->rep.arena_block_size = v;
}

void crocksdb_options_set_disable_auto_compactions(crocksdb_options_t* opt, int disable) {
  opt->rep.disable_auto_compactions = disable;
}

void crocksdb_options_set_delete_obsolete_files_period_micros(
    crocksdb_options_t* opt, uint64_t v) {
  opt->rep.delete_obsolete_files_period_micros = v;
}

void crocksdb_options_prepare_for_bulk_load(crocksdb_options_t* opt) {
  opt->rep.PrepareForBulkLoad();
}

void crocksdb_options_set_memtable_vector_rep(crocksdb_options_t *opt) {
  opt->rep.memtable_factory.reset(new rocksdb::VectorRepFactory);
}

void crocksdb_options_set_memtable_prefix_bloom_size_ratio(
    crocksdb_options_t* opt, double v) {
  opt->rep.memtable_prefix_bloom_size_ratio = v;
}

void crocksdb_options_set_memtable_huge_page_size(crocksdb_options_t* opt,
                                                 size_t v) {
  opt->rep.memtable_huge_page_size = v;
}

void crocksdb_options_set_hash_skip_list_rep(
    crocksdb_options_t *opt, size_t bucket_count,
    int32_t skiplist_height, int32_t skiplist_branching_factor) {
  rocksdb::MemTableRepFactory* factory = rocksdb::NewHashSkipListRepFactory(
      bucket_count, skiplist_height, skiplist_branching_factor);
  opt->rep.memtable_factory.reset(factory);
}

void crocksdb_options_set_hash_link_list_rep(
    crocksdb_options_t *opt, size_t bucket_count) {
  opt->rep.memtable_factory.reset(rocksdb::NewHashLinkListRepFactory(bucket_count));
}

void crocksdb_options_set_plain_table_factory(
    crocksdb_options_t *opt, uint32_t user_key_len, int bloom_bits_per_key,
    double hash_table_ratio, size_t index_sparseness) {
  rocksdb::PlainTableOptions options;
  options.user_key_len = user_key_len;
  options.bloom_bits_per_key = bloom_bits_per_key;
  options.hash_table_ratio = hash_table_ratio;
  options.index_sparseness = index_sparseness;

  rocksdb::TableFactory* factory = rocksdb::NewPlainTableFactory(options);
  opt->rep.table_factory.reset(factory);
}

void crocksdb_options_set_max_successive_merges(
    crocksdb_options_t* opt, size_t v) {
  opt->rep.max_successive_merges = v;
}

void crocksdb_options_set_min_partial_merge_operands(
    crocksdb_options_t* opt, uint32_t v) {
  opt->rep.min_partial_merge_operands = v;
}

void crocksdb_options_set_bloom_locality(
    crocksdb_options_t* opt, uint32_t v) {
  opt->rep.bloom_locality = v;
}

void crocksdb_options_set_inplace_update_support(
    crocksdb_options_t* opt, unsigned char v) {
  opt->rep.inplace_update_support = v;
}

void crocksdb_options_set_inplace_update_num_locks(
    crocksdb_options_t* opt, size_t v) {
  opt->rep.inplace_update_num_locks = v;
}

void crocksdb_options_set_report_bg_io_stats(
    crocksdb_options_t* opt, int v) {
  opt->rep.report_bg_io_stats = v;
}

void crocksdb_options_set_compaction_readahead_size(
    crocksdb_options_t* opt, size_t v) {
  opt->rep.compaction_readahead_size = v;
}

void crocksdb_options_set_compaction_style(crocksdb_options_t *opt, int style) {
  opt->rep.compaction_style = static_cast<rocksdb::CompactionStyle>(style);
}

void crocksdb_options_set_universal_compaction_options(crocksdb_options_t *opt, crocksdb_universal_compaction_options_t *uco) {
  opt->rep.compaction_options_universal = *(uco->rep);
}

void crocksdb_options_set_fifo_compaction_options(
    crocksdb_options_t* opt,
    crocksdb_fifo_compaction_options_t* fifo) {
  opt->rep.compaction_options_fifo = fifo->rep;
}

char *crocksdb_options_statistics_get_string(crocksdb_options_t *opt) {
  rocksdb::Statistics *statistics = opt->rep.statistics.get();
  if (statistics) {
    return strdup(statistics->ToString().c_str());
  }
  return nullptr;
}

uint64_t crocksdb_options_statistics_get_ticker_count(crocksdb_options_t* opt,
                                                      uint32_t ticker_type) {
  rocksdb::Statistics* statistics = opt->rep.statistics.get();
  if (statistics) {
    return statistics->getTickerCount(ticker_type);
  }
  return 0;
}

uint64_t crocksdb_options_statistics_get_and_reset_ticker_count(crocksdb_options_t* opt,
                                                                uint32_t ticker_type) {
  rocksdb::Statistics* statistics = opt->rep.statistics.get();
  if (statistics) {
    return statistics->getAndResetTickerCount(ticker_type);
  }
  return 0;
}

char* crocksdb_options_statistics_get_histogram_string(crocksdb_options_t* opt,
                                                       uint32_t type) {
  rocksdb::Statistics* statistics = opt->rep.statistics.get();
  if (statistics) {
    return strdup(statistics->getHistogramString(type).c_str());
  }
  return nullptr;
}

unsigned char crocksdb_options_statistics_get_histogram(
    crocksdb_options_t* opt,
    uint32_t type,
    double* median,
    double* percentile95,
    double* percentile99,
    double* average,
    double* standard_deviation) {
  rocksdb::Statistics* statistics = opt->rep.statistics.get();
  if (statistics) {
    crocksdb_histogramdata_t data;
    statistics->histogramData(type, &data.rep);
    *median = data.rep.median;
    *percentile95 = data.rep.percentile95;
    *percentile99 = data.rep.percentile99;
    *average = data.rep.average;
    *standard_deviation = data.rep.standard_deviation;
    return 1;
  }
  return 0;
}

void crocksdb_options_set_ratelimiter(crocksdb_options_t *opt, crocksdb_ratelimiter_t *limiter) {
  opt->rep.rate_limiter.reset(limiter->rep);
  limiter->rep = nullptr;
}

crocksdb_ratelimiter_t* crocksdb_ratelimiter_create(
    int64_t rate_bytes_per_sec,
    int64_t refill_period_us,
    int32_t fairness) {
  crocksdb_ratelimiter_t* rate_limiter = new crocksdb_ratelimiter_t;
  rate_limiter->rep = NewGenericRateLimiter(rate_bytes_per_sec,
                                            refill_period_us, fairness);
  return rate_limiter;
}

void crocksdb_ratelimiter_destroy(crocksdb_ratelimiter_t *limiter) {
  if (limiter->rep) {
	delete limiter->rep;
  }
  delete limiter;
}

/*
TODO:
DB::OpenForReadOnly
DB::KeyMayExist
DB::GetOptions
DB::GetSortedWalFiles
DB::GetLatestSequenceNumber
DB::GetUpdatesSince
DB::GetDbIdentity
DB::RunManualCompaction
custom cache
table_properties_collectors
*/

crocksdb_compactionfilter_t* crocksdb_compactionfilter_create(
    void* state,
    void (*destructor)(void*),
    unsigned char (*filter)(
        void*,
        int level,
        const char* key, size_t key_length,
        const char* existing_value, size_t value_length,
        char** new_value, size_t *new_value_length,
        unsigned char* value_changed),
    const char* (*name)(void*)) {
  crocksdb_compactionfilter_t* result = new crocksdb_compactionfilter_t;
  result->state_ = state;
  result->destructor_ = destructor;
  result->filter_ = filter;
  result->ignore_snapshots_ = false;
  result->name_ = name;
  return result;
}

void crocksdb_compactionfilter_set_ignore_snapshots(
  crocksdb_compactionfilter_t* filter,
  unsigned char whether_ignore) {
  filter->ignore_snapshots_ = whether_ignore;
}

void crocksdb_compactionfilter_destroy(crocksdb_compactionfilter_t* filter) {
  delete filter;
}

unsigned char crocksdb_compactionfiltercontext_is_full_compaction(
    crocksdb_compactionfiltercontext_t* context) {
  return context->rep.is_full_compaction;
}

unsigned char crocksdb_compactionfiltercontext_is_manual_compaction(
    crocksdb_compactionfiltercontext_t* context) {
  return context->rep.is_manual_compaction;
}

crocksdb_compactionfilterfactory_t* crocksdb_compactionfilterfactory_create(
    void* state, void (*destructor)(void*),
    crocksdb_compactionfilter_t* (*create_compaction_filter)(
        void*, crocksdb_compactionfiltercontext_t* context),
    const char* (*name)(void*)) {
  crocksdb_compactionfilterfactory_t* result =
      new crocksdb_compactionfilterfactory_t;
  result->state_ = state;
  result->destructor_ = destructor;
  result->create_compaction_filter_ = create_compaction_filter;
  result->name_ = name;
  return result;
}

void crocksdb_compactionfilterfactory_destroy(
    crocksdb_compactionfilterfactory_t* factory) {
  delete factory;
}

crocksdb_comparator_t* crocksdb_comparator_create(
    void* state,
    void (*destructor)(void*),
    int (*compare)(
        void*,
        const char* a, size_t alen,
        const char* b, size_t blen),
    const char* (*name)(void*)) {
  crocksdb_comparator_t* result = new crocksdb_comparator_t;
  result->state_ = state;
  result->destructor_ = destructor;
  result->compare_ = compare;
  result->name_ = name;
  return result;
}

void crocksdb_comparator_destroy(crocksdb_comparator_t* cmp) {
  delete cmp;
}

crocksdb_filterpolicy_t* crocksdb_filterpolicy_create(
    void* state,
    void (*destructor)(void*),
    char* (*create_filter)(
        void*,
        const char* const* key_array, const size_t* key_length_array,
        int num_keys,
        size_t* filter_length),
    unsigned char (*key_may_match)(
        void*,
        const char* key, size_t length,
        const char* filter, size_t filter_length),
    void (*delete_filter)(
        void*,
        const char* filter, size_t filter_length),
    const char* (*name)(void*)) {
  crocksdb_filterpolicy_t* result = new crocksdb_filterpolicy_t;
  result->state_ = state;
  result->destructor_ = destructor;
  result->create_ = create_filter;
  result->key_match_ = key_may_match;
  result->delete_filter_ = delete_filter;
  result->name_ = name;
  return result;
}

void crocksdb_filterpolicy_destroy(crocksdb_filterpolicy_t* filter) {
  delete filter;
}

crocksdb_filterpolicy_t* crocksdb_filterpolicy_create_bloom_format(int bits_per_key, bool original_format) {
  // Make a crocksdb_filterpolicy_t, but override all of its methods so
  // they delegate to a NewBloomFilterPolicy() instead of user
  // supplied C functions.
  struct Wrapper : public crocksdb_filterpolicy_t {
    const FilterPolicy* rep_;
    ~Wrapper() { delete rep_; }
    const char* Name() const override { return rep_->Name(); }
    void CreateFilter(const Slice* keys, int n,
                      std::string* dst) const override {
      return rep_->CreateFilter(keys, n, dst);
    }
    bool KeyMayMatch(const Slice& key, const Slice& filter) const override {
      return rep_->KeyMayMatch(key, filter);
    }
    static void DoNothing(void*) { }
  };
  Wrapper* wrapper = new Wrapper;
  wrapper->rep_ = NewBloomFilterPolicy(bits_per_key, original_format);
  wrapper->state_ = nullptr;
  wrapper->delete_filter_ = nullptr;
  wrapper->destructor_ = &Wrapper::DoNothing;
  return wrapper;
}

crocksdb_filterpolicy_t* crocksdb_filterpolicy_create_bloom_full(int bits_per_key) {
  return crocksdb_filterpolicy_create_bloom_format(bits_per_key, false);
}

crocksdb_filterpolicy_t* crocksdb_filterpolicy_create_bloom(int bits_per_key) {
  return crocksdb_filterpolicy_create_bloom_format(bits_per_key, true);
}

crocksdb_mergeoperator_t* crocksdb_mergeoperator_create(
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
    const char* (*name)(void*)) {
  crocksdb_mergeoperator_t* result = new crocksdb_mergeoperator_t;
  result->state_ = state;
  result->destructor_ = destructor;
  result->full_merge_ = full_merge;
  result->partial_merge_ = partial_merge;
  result->delete_value_ = delete_value;
  result->name_ = name;
  return result;
}

void crocksdb_mergeoperator_destroy(crocksdb_mergeoperator_t* merge_operator) {
  delete merge_operator;
}

crocksdb_readoptions_t* crocksdb_readoptions_create() {
  return new crocksdb_readoptions_t;
}

void crocksdb_readoptions_destroy(crocksdb_readoptions_t* opt) {
  delete opt;
}

void crocksdb_readoptions_set_verify_checksums(
    crocksdb_readoptions_t* opt,
    unsigned char v) {
  opt->rep.verify_checksums = v;
}

void crocksdb_readoptions_set_fill_cache(
    crocksdb_readoptions_t* opt, unsigned char v) {
  opt->rep.fill_cache = v;
}

void crocksdb_readoptions_set_snapshot(
    crocksdb_readoptions_t* opt,
    const crocksdb_snapshot_t* snap) {
  opt->rep.snapshot = (snap ? snap->rep : nullptr);
}

void crocksdb_readoptions_set_iterate_upper_bound(
    crocksdb_readoptions_t* opt,
    const char* key, size_t keylen) {
  if (key == nullptr) {
    opt->upper_bound = Slice();
    opt->rep.iterate_upper_bound = nullptr;

  } else {
    opt->upper_bound = Slice(key, keylen);
    opt->rep.iterate_upper_bound = &opt->upper_bound;
  }
}

void crocksdb_readoptions_set_read_tier(
    crocksdb_readoptions_t* opt, int v) {
  opt->rep.read_tier = static_cast<rocksdb::ReadTier>(v);
}

void crocksdb_readoptions_set_tailing(
    crocksdb_readoptions_t* opt, unsigned char v) {
  opt->rep.tailing = v;
}

void crocksdb_readoptions_set_readahead_size(
    crocksdb_readoptions_t* opt, size_t v) {
  opt->rep.readahead_size = v;
}

void crocksdb_readoptions_set_total_order_seek(crocksdb_readoptions_t* opt,
    unsigned char v) {
  opt->rep.total_order_seek = v;
}

crocksdb_writeoptions_t* crocksdb_writeoptions_create() {
  return new crocksdb_writeoptions_t;
}

void crocksdb_writeoptions_destroy(crocksdb_writeoptions_t* opt) {
  delete opt;
}

void crocksdb_writeoptions_set_sync(
    crocksdb_writeoptions_t* opt, unsigned char v) {
  opt->rep.sync = v;
}

void crocksdb_writeoptions_disable_WAL(crocksdb_writeoptions_t* opt, int disable) {
  opt->rep.disableWAL = disable;
}

crocksdb_compactoptions_t* crocksdb_compactoptions_create() {
  return new crocksdb_compactoptions_t;
}

void crocksdb_compactoptions_destroy(crocksdb_compactoptions_t* opt) {
  delete opt;
}

void crocksdb_compactoptions_set_exclusive_manual_compaction(
    crocksdb_compactoptions_t* opt, unsigned char v) {
  opt->rep.exclusive_manual_compaction = v;
}

void crocksdb_compactoptions_set_change_level(crocksdb_compactoptions_t* opt,
                                             unsigned char v) {
  opt->rep.change_level = v;
}

void crocksdb_compactoptions_set_target_level(crocksdb_compactoptions_t* opt,
                                             int n) {
  opt->rep.target_level = n;
}

crocksdb_flushoptions_t* crocksdb_flushoptions_create() {
  return new crocksdb_flushoptions_t;
}

void crocksdb_flushoptions_destroy(crocksdb_flushoptions_t* opt) {
  delete opt;
}

void crocksdb_flushoptions_set_wait(
    crocksdb_flushoptions_t* opt, unsigned char v) {
  opt->rep.wait = v;
}

crocksdb_cache_t* crocksdb_cache_create_lru(size_t capacity) {
  crocksdb_cache_t* c = new crocksdb_cache_t;
  c->rep = NewLRUCache(capacity);
  return c;
}

void crocksdb_cache_destroy(crocksdb_cache_t* cache) {
  delete cache;
}

void crocksdb_cache_set_capacity(crocksdb_cache_t* cache, size_t capacity) {
  cache->rep->SetCapacity(capacity);
}

crocksdb_env_t* crocksdb_create_default_env() {
  crocksdb_env_t* result = new crocksdb_env_t;
  result->rep = Env::Default();
  result->is_default = true;
  return result;
}

crocksdb_env_t* crocksdb_create_mem_env() {
  crocksdb_env_t* result = new crocksdb_env_t;
  result->rep = rocksdb::NewMemEnv(Env::Default());
  result->is_default = false;
  return result;
}

void crocksdb_env_set_background_threads(crocksdb_env_t* env, int n) {
  env->rep->SetBackgroundThreads(n);
}

void crocksdb_env_set_high_priority_background_threads(crocksdb_env_t* env, int n) {
  env->rep->SetBackgroundThreads(n, Env::HIGH);
}

void crocksdb_env_join_all_threads(crocksdb_env_t* env) {
  env->rep->WaitForJoin();
}

void crocksdb_env_destroy(crocksdb_env_t* env) {
  if (!env->is_default) delete env->rep;
  delete env;
}

crocksdb_envoptions_t* crocksdb_envoptions_create() {
  crocksdb_envoptions_t* opt = new crocksdb_envoptions_t;
  return opt;
}

void crocksdb_envoptions_destroy(crocksdb_envoptions_t* opt) { delete opt; }

crocksdb_sstfilewriter_t* crocksdb_sstfilewriter_create(
    const crocksdb_envoptions_t* env, const crocksdb_options_t* io_options) {
  crocksdb_sstfilewriter_t* writer = new crocksdb_sstfilewriter_t;
  writer->rep =
      new SstFileWriter(env->rep, io_options->rep, io_options->rep.comparator);
  return writer;
}

crocksdb_sstfilewriter_t* crocksdb_sstfilewriter_create_with_comparator(
    const crocksdb_envoptions_t* env, const crocksdb_options_t* io_options,
    const crocksdb_comparator_t* comparator) {
  crocksdb_sstfilewriter_t* writer = new crocksdb_sstfilewriter_t;
  writer->rep =
      new SstFileWriter(env->rep, io_options->rep, io_options->rep.comparator);
  return writer;
}

void crocksdb_sstfilewriter_open(crocksdb_sstfilewriter_t* writer,
                                const char* name, char** errptr) {
  SaveError(errptr, writer->rep->Open(std::string(name)));
}

void crocksdb_sstfilewriter_add(crocksdb_sstfilewriter_t* writer, const char* key,
                               size_t keylen, const char* val, size_t vallen,
                               char** errptr) {
  SaveError(errptr, writer->rep->Add(Slice(key, keylen), Slice(val, vallen)));
}

void crocksdb_sstfilewriter_finish(crocksdb_sstfilewriter_t* writer,
                                  char** errptr) {
  SaveError(errptr, writer->rep->Finish(NULL));
}

void crocksdb_sstfilewriter_destroy(crocksdb_sstfilewriter_t* writer) {
  delete writer->rep;
  delete writer;
}

crocksdb_ingestexternalfileoptions_t*
crocksdb_ingestexternalfileoptions_create() {
  crocksdb_ingestexternalfileoptions_t* opt =
      new crocksdb_ingestexternalfileoptions_t;
  return opt;
}

void crocksdb_ingestexternalfileoptions_set_move_files(
    crocksdb_ingestexternalfileoptions_t* opt, unsigned char move_files) {
  opt->rep.move_files = move_files;
}

void crocksdb_ingestexternalfileoptions_set_snapshot_consistency(
    crocksdb_ingestexternalfileoptions_t* opt,
    unsigned char snapshot_consistency) {
  opt->rep.snapshot_consistency = snapshot_consistency;
}

void crocksdb_ingestexternalfileoptions_set_allow_global_seqno(
    crocksdb_ingestexternalfileoptions_t* opt,
    unsigned char allow_global_seqno) {
  opt->rep.allow_global_seqno = allow_global_seqno;
}

void crocksdb_ingestexternalfileoptions_set_allow_blocking_flush(
    crocksdb_ingestexternalfileoptions_t* opt,
    unsigned char allow_blocking_flush) {
  opt->rep.allow_blocking_flush = allow_blocking_flush;
}

void crocksdb_ingestexternalfileoptions_destroy(
    crocksdb_ingestexternalfileoptions_t* opt) {
  delete opt;
}

void crocksdb_ingest_external_file(
    crocksdb_t* db, const char* const* file_list, const size_t list_len,
    const crocksdb_ingestexternalfileoptions_t* opt, char** errptr) {
  std::vector<std::string> files(list_len);
  for (size_t i = 0; i < list_len; ++i) {
    files[i] = std::string(file_list[i]);
  }
  SaveError(errptr, db->rep->IngestExternalFile(files, opt->rep));
}

void crocksdb_ingest_external_file_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* handle,
    const char* const* file_list, const size_t list_len,
    const crocksdb_ingestexternalfileoptions_t* opt, char** errptr) {
  std::vector<std::string> files(list_len);
  for (size_t i = 0; i < list_len; ++i) {
    files[i] = std::string(file_list[i]);
  }
  SaveError(errptr, db->rep->IngestExternalFile(handle->rep, files, opt->rep));
}

crocksdb_slicetransform_t* crocksdb_slicetransform_create(
    void* state,
    void (*destructor)(void*),
    char* (*transform)(
        void*,
        const char* key, size_t length,
        size_t* dst_length),
    unsigned char (*in_domain)(
        void*,
        const char* key, size_t length),
    unsigned char (*in_range)(
        void*,
        const char* key, size_t length),
    const char* (*name)(void*)) {
  crocksdb_slicetransform_t* result = new crocksdb_slicetransform_t;
  result->state_ = state;
  result->destructor_ = destructor;
  result->transform_ = transform;
  result->in_domain_ = in_domain;
  result->in_range_ = in_range;
  result->name_ = name;
  return result;
}

void crocksdb_slicetransform_destroy(crocksdb_slicetransform_t* st) {
  delete st;
}

crocksdb_slicetransform_t* crocksdb_slicetransform_create_fixed_prefix(size_t prefixLen) {
  struct Wrapper : public crocksdb_slicetransform_t {
    const SliceTransform* rep_;
    ~Wrapper() { delete rep_; }
    const char* Name() const override { return rep_->Name(); }
    Slice Transform(const Slice& src) const override {
      return rep_->Transform(src);
    }
    bool InDomain(const Slice& src) const override {
      return rep_->InDomain(src);
    }
    bool InRange(const Slice& src) const override { return rep_->InRange(src); }
    static void DoNothing(void*) { }
  };
  Wrapper* wrapper = new Wrapper;
  wrapper->rep_ = rocksdb::NewFixedPrefixTransform(prefixLen);
  wrapper->state_ = nullptr;
  wrapper->destructor_ = &Wrapper::DoNothing;
  return wrapper;
}

crocksdb_slicetransform_t* crocksdb_slicetransform_create_noop() {
  struct Wrapper : public crocksdb_slicetransform_t {
    const SliceTransform* rep_;
    ~Wrapper() { delete rep_; }
    const char* Name() const override { return rep_->Name(); }
    Slice Transform(const Slice& src) const override {
      return rep_->Transform(src);
    }
    bool InDomain(const Slice& src) const override {
      return rep_->InDomain(src);
    }
    bool InRange(const Slice& src) const override { return rep_->InRange(src); }
    static void DoNothing(void*) { }
  };
  Wrapper* wrapper = new Wrapper;
  wrapper->rep_ = rocksdb::NewNoopTransform();
  wrapper->state_ = nullptr;
  wrapper->destructor_ = &Wrapper::DoNothing;
  return wrapper;
}

crocksdb_universal_compaction_options_t* crocksdb_universal_compaction_options_create() {
  crocksdb_universal_compaction_options_t* result = new crocksdb_universal_compaction_options_t;
  result->rep = new rocksdb::CompactionOptionsUniversal;
  return result;
}

void crocksdb_universal_compaction_options_set_size_ratio(
  crocksdb_universal_compaction_options_t* uco, int ratio) {
  uco->rep->size_ratio = ratio;
}

void crocksdb_universal_compaction_options_set_min_merge_width(
  crocksdb_universal_compaction_options_t* uco, int w) {
  uco->rep->min_merge_width = w;
}

void crocksdb_universal_compaction_options_set_max_merge_width(
  crocksdb_universal_compaction_options_t* uco, int w) {
  uco->rep->max_merge_width = w;
}

void crocksdb_universal_compaction_options_set_max_size_amplification_percent(
  crocksdb_universal_compaction_options_t* uco, int p) {
  uco->rep->max_size_amplification_percent = p;
}

void crocksdb_universal_compaction_options_set_compression_size_percent(
  crocksdb_universal_compaction_options_t* uco, int p) {
  uco->rep->compression_size_percent = p;
}

void crocksdb_universal_compaction_options_set_stop_style(
  crocksdb_universal_compaction_options_t* uco, int style) {
  uco->rep->stop_style = static_cast<rocksdb::CompactionStopStyle>(style);
}

void crocksdb_universal_compaction_options_destroy(
  crocksdb_universal_compaction_options_t* uco) {
  delete uco->rep;
  delete uco;
}

crocksdb_fifo_compaction_options_t* crocksdb_fifo_compaction_options_create() {
  crocksdb_fifo_compaction_options_t* result = new crocksdb_fifo_compaction_options_t;
  result->rep =  CompactionOptionsFIFO();
  return result;
}

void crocksdb_fifo_compaction_options_set_max_table_files_size(
    crocksdb_fifo_compaction_options_t* fifo_opts, uint64_t size) {
  fifo_opts->rep.max_table_files_size = size;
}

void crocksdb_fifo_compaction_options_destroy(
    crocksdb_fifo_compaction_options_t* fifo_opts) {
  delete fifo_opts;
}

void crocksdb_options_set_min_level_to_compress(crocksdb_options_t* opt, int level) {
  if (level >= 0) {
    assert(level <= opt->rep.num_levels);
    opt->rep.compression_per_level.resize(opt->rep.num_levels);
    for (int i = 0; i < level; i++) {
      opt->rep.compression_per_level[i] = rocksdb::kNoCompression;
    }
    for (int i = level; i < opt->rep.num_levels; i++) {
      opt->rep.compression_per_level[i] = opt->rep.compression;
    }
  }
}

int crocksdb_livefiles_count(
  const crocksdb_livefiles_t* lf) {
  return static_cast<int>(lf->rep.size());
}

const char* crocksdb_livefiles_name(
  const crocksdb_livefiles_t* lf,
  int index) {
  return lf->rep[index].name.c_str();
}

int crocksdb_livefiles_level(
  const crocksdb_livefiles_t* lf,
  int index) {
  return lf->rep[index].level;
}

size_t crocksdb_livefiles_size(
  const crocksdb_livefiles_t* lf,
  int index) {
  return lf->rep[index].size;
}

const char* crocksdb_livefiles_smallestkey(
  const crocksdb_livefiles_t* lf,
  int index,
  size_t* size) {
  *size = lf->rep[index].smallestkey.size();
  return lf->rep[index].smallestkey.data();
}

const char* crocksdb_livefiles_largestkey(
  const crocksdb_livefiles_t* lf,
  int index,
  size_t* size) {
  *size = lf->rep[index].largestkey.size();
  return lf->rep[index].largestkey.data();
}

extern void crocksdb_livefiles_destroy(
  const crocksdb_livefiles_t* lf) {
  delete lf;
}

void crocksdb_get_options_from_string(const crocksdb_options_t* base_options,
                                     const char* opts_str,
                                     crocksdb_options_t* new_options,
                                     char** errptr) {
  SaveError(errptr,
            GetOptionsFromString(base_options->rep, std::string(opts_str),
                                 &new_options->rep));
}

void crocksdb_delete_file_in_range(crocksdb_t* db, const char* start_key,
                                  size_t start_key_len, const char* limit_key,
                                  size_t limit_key_len, char** errptr) {
  Slice a, b;
  SaveError(
      errptr,
      DeleteFilesInRange(
          db->rep, db->rep->DefaultColumnFamily(),
          (start_key ? (a = Slice(start_key, start_key_len), &a) : nullptr),
          (limit_key ? (b = Slice(limit_key, limit_key_len), &b) : nullptr)));
}

void crocksdb_delete_file_in_range_cf(
    crocksdb_t* db, crocksdb_column_family_handle_t* column_family,
    const char* start_key, size_t start_key_len, const char* limit_key,
    size_t limit_key_len, char** errptr) {
  Slice a, b;
  SaveError(
      errptr,
      DeleteFilesInRange(
          db->rep, column_family->rep,
          (start_key ? (a = Slice(start_key, start_key_len), &a) : nullptr),
          (limit_key ? (b = Slice(limit_key, limit_key_len), &b) : nullptr)));
}

void crocksdb_free(void* ptr) { free(ptr); }

}  // end extern "C"
