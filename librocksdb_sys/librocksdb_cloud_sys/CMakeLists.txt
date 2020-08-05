# Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.
cmake_minimum_required(VERSION 3.0)
project(rocksdb_cloud)
enable_language(CXX)
enable_language(C)

# Manage absence of rocksdb code dependence

if (NOT ROCKSDB_GIT_REPO)
  set(ROCKSDB_GIT_REPO "https://github.com/tikv/rocksdb.git")
endif()

if (NOT ROCKSDB_GIT_BRANCH)
  set(ROCKSDB_GIT_BRANCH "6.4.tikv")
endif()

if (NOT DEFINED ROCKSDB_DIR)
  if (GIT_FOUND)
    if (WIN32)
      execute_process(COMMAND $ENV{COMSPEC} /C ${GIT_EXECUTABLE} clone --branch=${ROCKSDB_GIT_BRANCH} ${ROCKSDB_GIT_REPO})
    else()
      execute_process(COMMAND ${GIT_EXECUTABLE} clone --branch=${ROCKSDB_GIT_BRANCH} ${ROCKSDB_GIT_REPO})
    endif()
    set(ROCKSDB_DIR "${CMAKE_BINARY_DIR}/rocksdb")
  endif()
endif()

if (NOT DEFINED ROCKSDB_DIR)
  message(FATAL_ERROR "ROCKSDB_DIR is not defined.")
endif()

# Append rocksdb modules
list(APPEND CMAKE_MODULE_PATH "${ROCKSDB_DIR}/cmake/modules/")
include("cmake_modules/rocksdb_flags.cmake")

include_directories("${ROCKSDB_DIR}")
include_directories("${ROCKSDB_DIR}/include")
include_directories("rocksdb-cloud")
include_directories("rocksdb-cloud/include")

file(GLOB SOURCES rocksdb-cloud/cloud/*.cc)

add_library(rocksdb_cloud STATIC ${SOURCES} $<TARGET_OBJECTS:cloud_env>)

option(WITH_CLOUD_TESTS "Build with tests." ON)

include_directories(SYSTEM ${ROCKSDB_DIR}/third-party/gtest-1.7.0/fused-src)

add_library(cloud_env OBJECT "rocksdb-cloud/cloud/cloud_env.cc")

include(GNUInstallDirs)
install(DIRECTORY include/rocksdb/cloud
  COMPONENT devel
  DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
)
install(TARGETS rocksdb_cloud
  COMPONENT devel
  ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
  INCLUDES DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
)
