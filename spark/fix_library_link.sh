#!/bin/bash
# Fix libscc.so to include DT_NEEDED for liboqs_wrapper.so
# This is needed because gprbuild doesn't properly add DT_NEEDED entries for dependent libraries

LIB_DIR="$(dirname "$0")/lib"
LIBSCC="$LIB_DIR/libscc.so"
LIBOQS_WRAPPER="$LIB_DIR/liboqs_wrapper.so"

if [ ! -f "$LIBSCC" ]; then
    echo "Error: $LIBSCC not found"
    exit 1
fi

if [ ! -f "$LIBOQS_WRAPPER" ]; then
    echo "Error: $LIBOQS_WRAPPER not found"
    exit 1
fi

# Check if patchelf is available
if command -v patchelf &> /dev/null; then
    echo "Using patchelf to add DT_NEEDED entry..."
    patchelf --add-needed liboqs_wrapper.so "$LIBSCC"
else
    echo "patchelf not available, using ELF manipulation..."
    # Read current NEEDED entries
    NEEDED_COUNT=$(readelf -d "$LIBSCC" 2>/dev/null | grep -c NEEDED || echo 0)
    
    # Check if liboqs_wrapper.so is already in NEEDED
    if readelf -d "$LIBSCC" 2>/dev/null | grep -q "liboqs_wrapper.so"; then
        echo "liboqs_wrapper.so already in DT_NEEDED"
        exit 0
    fi
    
    echo "Error: patchelf required but not installed"
    echo "Install with: pacman -S patchelf (or apt install patchelf)"
    echo ""
    echo "Alternative: set LD_PRELOAD=/path/to/liboqs_wrapper.so when running"
    exit 1
fi

echo "Successfully added liboqs_wrapper.so to DT_NEEDED"
readelf -d "$LIBSCC" | grep NEEDED