# benchmarks/tpcc/Makefile
#
# Local entry point for the TPC-C harness. The top-level
# `make -f Makefile.ai-blaise bench-tpcc` proxies into here.

.PHONY: run quick full clean

run: quick

quick:
	BENCH_QUICK=1 ./run.sh

full:
	BENCH_QUICK=0 \
	BENCH_DURATION_SECS=$${BENCH_DURATION_SECS:-600} \
	BENCH_CLIENTS=$${BENCH_CLIENTS:-32} \
	BENCH_SCALE=$${BENCH_SCALE:-10} \
	BENCH_RESULT_TAG=$${BENCH_RESULT_TAG:-release} \
	./run.sh

clean:
	rm -rf ../results/tpcc-*
