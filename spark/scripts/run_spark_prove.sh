#!/bin/bash
# SPARK Formal Verification Runner
# Run this script to verify all SPARK code in the project
# Usage: ./scripts/run_spark_prove.sh [--verbose] [--jobs N]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SPARK_DIR="$SCRIPT_DIR/.."
VERBOSE=""
JOBS="-j$(nproc)"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --verbose)
            VERBOSE="-v"
            shift
            ;;
        --jobs)
            JOBS="-j$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "=== SPARK Formal Verification Runner ==="
echo "SPARK Directory: $SPARK_DIR"

# Check if SPARK tools are available
if ! command -v gnatprove &> /dev/null; then
    echo "ERROR: gnatprove (SPARK Prover) not found"
    echo "Please install GNAT SPARK Pro from AdaCore"
    exit 1
fi

# Function to verify a directory
verify_dir() {
    local dir="$1"
    local name="$2"
    
    if [[ ! -d "$dir" ]]; then
        echo "Skipping $name (directory not found)"
        return
    fi
    
    echo ""
    echo "=== Verifying $name ==="
    echo "Directory: $dir"
    
    # Find all .ads files and verify
    local ads_files
    ads_files=$(find "$dir" -maxdepth 2 -name "*.ads" 2>/dev/null || true)
    
    if [[ -z "$ads_files" ]]; then
        echo "No .ads files found in $dir"
        return
    fi
    
    # Run SPARK proof on each .ads file
    for ads_file in $ads_files; do
        local basename
        basename=$(basename "$ads_file" .ads)
        echo "Proving $basename..."
        
        if gnatprove -P "$ads_file" $VERBOSE $JOBS --output=ide 2>&1; then
            echo "  ✓ $basename verified"
        else
            echo "  ✗ $basename FAILED"
        fi
    done
}

# Verify each SPARK module
verify_dir "$SPARK_DIR/kms" "KMS (Key Management Service)"
verify_dir "$SPARK_DIR/identity" "Identity Manager"
verify_dir "$SPARK_DIR/crypto" "Crypto Engine"
verify_dir "$SPARK_DIR/core_ledger" "Core Ledger"
verify_dir "$SPARK_DIR/idl_registry" "IDL Registry"

echo ""
echo "=== SPARK Verification Complete ==="
echo "For detailed output, run: gnatprove -P <project_file> -v"