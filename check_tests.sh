#!/bin/bash

# Count total test files and run tests for each module
total_tests=0
passed_tests=0
failed_tests=0

# Find all Cargo.toml files in rust modules
for cargo_file in rust/*/Cargo.toml; do
    if [ -f "$cargo_file" ]; then
        module_dir=$(dirname "$cargo_file")
        module_name=$(basename "$module_dir")
        echo "Checking tests for module: $module_name"
        
        # Run tests for this module
        cd "$module_dir"
        if cargo test -- --nocapture 2>/dev/null; then
            # Count tests
            test_output=$(cargo test -- --nocapture 2>&1 | tail -10)
            echo "$test_output"
            
            # Extract test counts
            if [[ $test_output =~ ([0-9]+)\ passed ]]; then
                passed=${BASH_REMATCH[1]}
                passed_tests=$((passed_tests + passed))
            fi
            if [[ $test_output =~ ([0-9]+)\ failed ]]; then
                failed=${BASH_REMATCH[1]}
                failed_tests=$((failed_tests + failed))
            fi
            if [[ $test_output =~ ([0-9]+)\ tests ]]; then
                total=${BASH_REMATCH[1]}
                total_tests=$((total_tests + total))
            fi
        else
            echo "Tests failed for $module_name"
            # Try to get test counts even if failed
            test_output=$(cargo test -- --nocapture 2>&1 | tail -10)
            echo "$test_output"
            
            # Extract test counts
            if [[ $test_output =~ ([0-9]+)\ passed ]]; then
                passed=${BASH_REMATCH[1]}
                passed_tests=$((passed_tests + passed))
            fi
            if [[ $test_output =~ ([0-9]+)\ failed ]]; then
                failed=${BASH_REMATCH[1]}
                failed_tests=$((failed_tests + failed))
            fi
            if [[ $test_output =~ ([0-9]+)\ tests ]]; then
                total=${BASH_REMATCH[1]}
                total_tests=$((total_tests + total))
            fi
        fi
        cd - > /dev/null
        echo "------------------------"
    fi
done

echo "Total tests: $total_tests"
echo "Passed tests: $passed_tests"
echo "Failed tests: $failed_tests"

if [ $failed_tests -eq 0 ]; then
    echo "All tests pass!"
else
    echo "Some tests failed!"
fi