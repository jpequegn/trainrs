#!/usr/bin/env bash
#
# Valgrind Memory Leak Detection Script
#
# Usage: ./scripts/valgrind_check.sh [test_name]
#
# Examples:
#   ./scripts/valgrind_check.sh                    # Run all tests
#   ./scripts/valgrind_check.sh memory_leak_tests  # Run specific test file
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REPORT_DIR="target/valgrind-reports"
REPORT_FILE="$REPORT_DIR/valgrind-report-$(date +%Y%m%d-%H%M%S).txt"

# Create report directory
mkdir -p "$REPORT_DIR"

echo "========================================="
echo "TrainRS Valgrind Memory Leak Detection"
echo "========================================="
echo ""

# Check if valgrind is installed
if ! command -v valgrind &> /dev/null; then
    echo -e "${RED}Error: Valgrind is not installed${NC}"
    echo ""
    echo "Install with:"
    echo "  Linux:  sudo apt-get install valgrind"
    echo "  macOS:  brew install valgrind"
    echo ""
    exit 1
fi

# Build in release mode for accurate results
echo -e "${YELLOW}Building in release mode...${NC}"
cargo build --release --tests

# Determine which test to run
TEST_NAME="${1:-memory_leak_tests}"

echo ""
echo -e "${YELLOW}Running Valgrind on: $TEST_NAME${NC}"
echo "Report will be saved to: $REPORT_FILE"
echo ""

# Run valgrind with comprehensive options
valgrind \
    --leak-check=full \
    --show-leak-kinds=all \
    --track-origins=yes \
    --verbose \
    --log-file="$REPORT_FILE" \
    --error-exitcode=1 \
    cargo test --release --test "$TEST_NAME" -- --test-threads=1 --ignored --nocapture

VALGRIND_EXIT_CODE=$?

echo ""
echo "========================================="
echo "Valgrind Analysis Complete"
echo "========================================="
echo ""

# Parse the report for key information
if [ -f "$REPORT_FILE" ]; then
    echo -e "${YELLOW}Summary from report:${NC}"
    echo ""

    # Extract key statistics
    DEFINITELY_LOST=$(grep "definitely lost:" "$REPORT_FILE" | tail -1 || echo "")
    INDIRECTLY_LOST=$(grep "indirectly lost:" "$REPORT_FILE" | tail -1 || echo "")
    POSSIBLY_LOST=$(grep "possibly lost:" "$REPORT_FILE" | tail -1 || echo "")
    STILL_REACHABLE=$(grep "still reachable:" "$REPORT_FILE" | tail -1 || echo "")

    if [ -n "$DEFINITELY_LOST" ]; then
        echo "$DEFINITELY_LOST"
    fi
    if [ -n "$INDIRECTLY_LOST" ]; then
        echo "$INDIRECTLY_LOST"
    fi
    if [ -n "$POSSIBLY_LOST" ]; then
        echo "$POSSIBLY_LOST"
    fi
    if [ -n "$STILL_REACHABLE" ]; then
        echo "$STILL_REACHABLE"
    fi

    echo ""

    # Check for leaks
    if grep -q "definitely lost: 0 bytes in 0 blocks" "$REPORT_FILE" && \
       grep -q "indirectly lost: 0 bytes in 0 blocks" "$REPORT_FILE"; then
        echo -e "${GREEN}✓ No memory leaks detected!${NC}"
        EXIT_CODE=0
    else
        echo -e "${RED}✗ Memory leaks detected${NC}"
        echo ""
        echo "Review full report at: $REPORT_FILE"
        EXIT_CODE=1
    fi
else
    echo -e "${RED}Error: Report file not generated${NC}"
    EXIT_CODE=1
fi

echo ""
echo "Full report: $REPORT_FILE"
echo ""

exit $EXIT_CODE
