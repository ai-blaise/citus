# benchmarks/timescale-ingest/Makefile
#
# Local entry point for the Timescale ingest harness. The top-level
# `make -f Makefile.ai-blaise bench-timescale-ingest` proxies into here.

.PHONY: run quick full clean

run: quick

quick:
	BENCH_QUICK=1 BENCH_ROWS=$${BENCH_ROWS:-50000} python3 ingest.py

full:
	BENCH_QUICK=0 \
	BENCH_DURATION_SECS=$${BENCH_DURATION_SECS:-600} \
	BENCH_ROWS=$${BENCH_ROWS:-10000000} \
	BENCH_SERIES=$${BENCH_SERIES:-1024} \
	BENCH_RESULT_TAG=$${BENCH_RESULT_TAG:-release} \
	python3 ingest.py

clean:
	rm -f ../results/timescale-ingest-*.json
