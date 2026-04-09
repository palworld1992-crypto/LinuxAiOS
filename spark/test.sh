#!/bin/bash
export PATH="/usr/libexec/spark/bin:$PATH"
cd "$(dirname "$0")"
gprbuild -P spark.gpr 2>&1 | tail -5
cd tests
make clean && make all 2>&1 | tail -5
echo ""
echo "=== Running Ada FFI tests ==="
./test_ada_dilithium 2>&1
echo ""
./test_ada_dilithium2 2>&1
echo ""
./test_ada_dilithium_verify 2>&1
echo ""
echo "=== All Ada tests completed ==="
