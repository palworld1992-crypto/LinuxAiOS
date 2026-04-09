#!/bin/bash
# rust/scc/tests/run_tests.sh
# Wrapper script to run Rust tests with LD_PRELOAD for liboqs_wrapper.so

set -e

# Get absolute paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_DIR="$(cd "$SCC_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$RUST_DIR/.." && pwd)"
LIBOQS_WRAPPER="$PROJECT_ROOT/spark/lib/liboqs_wrapper.so"
LIBSCC="$PROJECT_ROOT/spark/lib/libscc.so"

# Check if libraries exist
if [ ! -f "$LIBOQS_WRAPPER" ]; then
    echo "Error: $LIBOQS_WRAPPER not found"
    echo "Please build the Ada library first: cd $PROJECT_ROOT/spark && gprbuild -P spark.gpr"
    exit 1
fi

if [ ! -f "$LIBSCC" ]; then
    echo "Error: $LIBSCC not found"
    echo "Please build the Ada library first: cd $PROJECT_ROOT/spark && gprbuild -P spark.gpr"
    exit 1
fi

echo "Using LD_PRELOAD=$LIBOQS_WRAPPER"
echo "Using LD_LIBRARY_PATH=$PROJECT_ROOT/spark/lib"

# Set library path and LD_PRELOAD
export LD_LIBRARY_PATH="$PROJECT_ROOT/spark/lib:$LD_LIBRARY_PATH"
export LD_PRELOAD="$LIBOQS_WRAPPER"

# Run cargo test with all arguments passed through
cd "$PROJECT_ROOT/rust/scc"
exec cargo test "$@"