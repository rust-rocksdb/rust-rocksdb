#include "rocksdb/c.h"

#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "rocksdb/db.h"
#include "rocksdb/options.h"
#include "rocksdb/status.h"

using ROCKSDB_NAMESPACE::ColumnFamilyDescriptor;
using ROCKSDB_NAMESPACE::ColumnFamilyOptions;
using ROCKSDB_NAMESPACE::DBOptions;
using ROCKSDB_NAMESPACE::Options;
using ROCKSDB_NAMESPACE::RepairDB;
using ROCKSDB_NAMESPACE::Status;

struct rocksdb_options_t {
  Options rep;
};

namespace {

bool SaveError(char** errptr, const Status& status) {
  if (status.ok()) {
    return false;
  }

  if (errptr == nullptr) {
    return true;
  }

  if (*errptr != nullptr) {
    std::free(*errptr);
  }
  *errptr = ::strdup(status.ToString().c_str());
  return true;
}

std::vector<ColumnFamilyDescriptor> BuildColumnFamilies(
    int num_column_families, const char* const* column_family_names,
    const rocksdb_options_t* const* column_family_options) {
  std::vector<ColumnFamilyDescriptor> column_families;
  column_families.reserve(static_cast<size_t>(num_column_families));
  for (int i = 0; i < num_column_families; ++i) {
    column_families.emplace_back(
        std::string(column_family_names[i]),
        ColumnFamilyOptions(column_family_options[i]->rep));
  }
  return column_families;
}

}  // namespace

extern "C" {

void rocksdb_repair_db_cf_descriptors(
    const rocksdb_options_t* db_options, const char* name,
    int num_column_families, const char* const* column_family_names,
    const rocksdb_options_t* const* column_family_options, char** errptr) {
  const auto column_families = BuildColumnFamilies(
      num_column_families, column_family_names, column_family_options);
  SaveError(errptr,
            RepairDB(std::string(name), DBOptions(db_options->rep), column_families));
}

}  // extern "C"
