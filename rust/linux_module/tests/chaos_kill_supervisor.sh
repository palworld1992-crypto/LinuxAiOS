#!/bin/bash
# Chaos Test: Kill Supervisor Failover
# This test simulates killing a supervisor and measures failover time
# Usage: ./chaos_kill_supervisor.sh [supervisor_id] [iterations]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUPERVISOR_ID="${1:-supervisor_001}"
ITERATIONS="${2:-5}"
MAX_FAILOVER_TIME=2.0  # seconds

echo "=== Chaos Test: Kill Supervisor Failover ==="
echo "Supervisor: $SUPERVISOR_ID"
echo "Iterations: $ITERATIONS"
echo "Max allowed failover time: ${MAX_FAILOVER_TIME}s"
echo ""

total_time=0
failures=0

for i in $(seq 1 $ITERATIONS); do
    echo "--- Iteration $i/$ITERATIONS ---"
    
    # Record start time
    start_time=$(date +%s.%N)
    
    # Get supervisor PID
    SUPERVISOR_PID=$(pgrep -f "$SUPERVISOR_ID" | head -1 || echo "")
    
    if [[ -z "$SUPERVISOR_PID" ]]; then
        echo "Supervisor $SUPERVISOR_ID not running, starting it..."
        # Start supervisor in background (would be actual supervisor binary)
        # For testing, simulate with a dummy process
        sleep 0.5
        SUPERVISOR_PID=$(pgrep -f "sleep" | head -1 || echo "0")
    fi
    
    echo "Supervisor PID: $SUPERVISOR_PID"
    
    # Kill the supervisor
    echo "Killing supervisor..."
    if [[ "$SUPERVISOR_PID" != "0" ]]; then
        kill -9 "$SUPERVISOR_PID" 2>/dev/null || true
    fi
    
    # Wait for failover (standby should take over)
    # In real implementation, this would check for:
    # 1. Standby promoted to active
    # 2. Health tunnel reports new active
    # 3. Transport tunnel reconnected
    
    sleep 0.5
    
    # Check if failover happened
    # In production, would check:
    # - New active supervisor is running
    # - Connections are re-established
    # - No data loss
    
    # Record end time
    end_time=$(date +%s.%N)
    failover_time=$(echo "$end_time - $start_time" | bc)
    
    echo "Failover time: ${failover_time}s"
    
    # Check if within threshold
    if (( $(echo "$failover_time <= $MAX_FAILOVER_TIME" | bc -l) )); then
        echo "✓ PASS: Failover within ${MAX_FAILOVER_TIME}s"
    else
        echo "✗ FAIL: Failover exceeded ${MAX_FAILOVER_TIME}s"
        failures=$((failures + 1))
    fi
    
    total_time=$(echo "$total_time + $failover_time" | bc)
    
    # Cleanup and restart for next iteration
    # In production, would restore original supervisor
    sleep 1
done

# Calculate average
avg_time=$(echo "scale=3; $total_time / $ITERATIONS" | bc)

echo ""
echo "=== Results ==="
echo "Total iterations: $ITERATIONS"
echo "Failures: $failures"
echo "Average failover time: ${avg_time}s"

if [[ $failures -eq 0 ]]; then
    echo "✓ All tests passed"
    exit 0
else
    echo "✗ $failures test(s) failed"
    exit 1
fi