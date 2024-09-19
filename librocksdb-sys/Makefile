include rocksdb/src.mk

rocksdb_lib_sources.txt: rocksdb/src.mk
	@echo -n "${LIB_SOURCES}" | tr ' ' '\n' > rocksdb_lib_sources.txt

gen_lib_sources: rocksdb_lib_sources.txt

gen_statistics: rocksdb/monitoring/statistics.cc
	bash gen_statistics.bash $< Ticker     > ../src/statistics_enum_ticker.rs
	bash gen_statistics.bash $< Histogram  > ../src/statistics_enum_histogram.rs
