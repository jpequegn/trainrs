#!/usr/bin/env bash
#
# Stress Testing Script
#
# Usage: ./scripts/stress_test.sh
#
# Runs comprehensive stress tests and generates report
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
REPORT_DIR="target/stress-test-reports"
REPORT_FILE="$REPORT_DIR/stress-test-$(date +%Y%m%d-%H%M%S).txt"

mkdir -p "$REPORT_DIR"

echo "========================================="
echo "TrainRS Stress Testing Suite"
echo "========================================="
echo ""

# Build in release mode
echo -e "${YELLOW}Building in release mode...${NC}"
cargo build --release --tests
echo ""

# Function to run test and capture results
run_test() {
    local test_name=$1
    local description=$2

    echo -e "${BLUE}Running: $description${NC}"
    echo "Test: $test_name" | tee -a "$REPORT_FILE"

    START_TIME=$(date +%s)

    if timeout 300 cargo test --release --test stress_tests "$test_name" -- --ignored --nocapture >> "$REPORT_FILE" 2>&1; then
        END_TIME=$(date +%s)
        DURATION=$((END_TIME - START_TIME))
        echo -e "${GREEN}✓ PASS${NC} ($DURATION seconds)"
        echo "Result: PASS (${DURATION}s)" >> "$REPORT_FILE"
        return 0
    else
        END_TIME=$(date +%s)
        DURATION=$((END_TIME - START_TIME))
        echo -e "${RED}✗ FAIL${NC} ($DURATION seconds)"
        echo "Result: FAIL (${DURATION}s)" >> "$REPORT_FILE"
        return 1
    fi

    echo "" >> "$REPORT_FILE"
}

# Initialize report
echo "TrainRS Stress Test Report" > "$REPORT_FILE"
echo "Generated: $(date)" >> "$REPORT_FILE"
echo "=========================================" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Run all stress tests
echo -e "${YELLOW}Starting stress tests...${NC}"
echo ""

TESTS=(
    "test_large_workout_file:Large workout file (500K data points)"
    "test_rapid_import_burst:Rapid import burst (100 files)"
    "test_concurrent_operations:Concurrent operations (4 threads)"
    "test_extremely_long_workout:Extremely long workout (30 hours)"
    "test_memory_pressure:Memory pressure simulation"
    "test_rapid_allocation_cycles:Rapid allocation/deallocation"
    "test_data_integrity_large_dataset:Data integrity (100K points)"
)

for test_spec in "${TESTS[@]}"; do
    IFS=':' read -r test_name description <<< "$test_spec"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    if run_test "$test_name" "$description"; then
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi

    echo ""
done

# Also run the edge case tests (not ignored)
echo -e "${YELLOW}Running edge case tests...${NC}"
echo ""

EDGE_CASE_TESTS=(
    "test_workout_with_missing_fields:Missing fields handling"
    "test_empty_workout:Empty workout edge case"
    "test_single_datapoint_workout:Single data point"
    "test_invalid_string_data:Invalid string data"
)

for test_spec in "${EDGE_CASE_TESTS[@]}"; do
    IFS=':' read -r test_name description <<< "$test_spec"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    echo -e "${BLUE}Running: $description${NC}"
    echo "Test: $test_name" >> "$REPORT_FILE"

    if cargo test --release --test stress_tests "$test_name" -- --nocapture >> "$REPORT_FILE" 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
        echo "Result: PASS" >> "$REPORT_FILE"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}✗ FAIL${NC}"
        echo "Result: FAIL" >> "$REPORT_FILE"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi

    echo "" >> "$REPORT_FILE"
    echo ""
done

# Generate summary
echo "=========================================" >> "$REPORT_FILE"
echo "Summary" >> "$REPORT_FILE"
echo "=========================================" >> "$REPORT_FILE"
echo "Total tests: $TOTAL_TESTS" >> "$REPORT_FILE"
echo "Passed: $PASSED_TESTS" >> "$REPORT_FILE"
echo "Failed: $FAILED_TESTS" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Display summary
echo "========================================="
echo "Stress Test Summary"
echo "========================================="
echo -e "Total tests:  $TOTAL_TESTS"
echo -e "Passed:       ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:       ${RED}$FAILED_TESTS${NC}"
echo ""
echo "Full report: $REPORT_FILE"
echo "========================================="
echo ""

# Exit with failure if any tests failed
if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}Some tests failed. Review the report for details.${NC}"
    exit 1
else
    echo -e "${GREEN}All stress tests passed!${NC}"
    exit 0
fi
