# Memory Leak Detection and Stress Testing

Comprehensive guide for memory leak detection and stress testing in TrainRS.

## Overview

This document covers:
- Memory leak detection methodology
- Stress testing procedures
- CI/CD integration
- Acceptance criteria
- Troubleshooting guide

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Memory Leak Tests](#memory-leak-tests)
3. [Stress Tests](#stress-tests)
4. [Valgrind Integration](#valgrind-integration)
5. [CI/CD Integration](#cicd-integration)
6. [Acceptance Criteria](#acceptance-criteria)
7. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Run All Tests Locally

```bash
# Memory leak tests (native tracking)
cargo test --release --test memory_leak_tests -- --ignored --nocapture

# Stress tests
./scripts/stress_test.sh

# Valgrind memory leak detection (Linux only)
./scripts/valgrind_check.sh
```

### Individual Test Execution

```bash
# Specific memory leak test
cargo test --release --test memory_leak_tests test_no_memory_leak_sequential_import -- --ignored --nocapture

# Specific stress test
cargo test --release --test stress_tests test_large_workout_file -- --ignored --nocapture
```

---

## Memory Leak Tests

Located in: `tests/memory_leak_tests.rs`

### Test Categories

#### 1. Sequential Import Test
**Test**: `test_no_memory_leak_sequential_import`
**Purpose**: Verify no memory leaks during 1000 sequential workout imports
**Acceptance**: <10MB memory growth
**Duration**: ~30 seconds

```bash
cargo test --release --test memory_leak_tests test_no_memory_leak_sequential_import -- --ignored --nocapture
```

#### 2. Batch Import Stability Test
**Test**: `test_batch_import_stable_memory`
**Purpose**: Verify memory stabilizes after initial allocation in repeated batch operations
**Acceptance**: <5MB growth per iteration after first iteration
**Duration**: ~20 seconds

```bash
cargo test --release --test memory_leak_tests test_batch_import_stable_memory -- --ignored --nocapture
```

#### 3. TSS Calculation Memory Test
**Test**: `test_tss_calculation_no_leak`
**Purpose**: Verify TSS calculation doesn't leak memory over 1000 iterations
**Acceptance**: <5MB growth
**Duration**: ~10 seconds

#### 4. PMC Calculation Memory Test
**Test**: `test_pmc_calculation_no_leak`
**Purpose**: Verify PMC calculation over 365 days doesn't leak memory
**Acceptance**: <5MB growth for 10 iterations
**Duration**: ~15 seconds

#### 5. Large Workout Data Test
**Test**: `test_large_workout_data_memory`
**Purpose**: Verify large workouts (10K data points) use reasonable memory
**Acceptance**: <50MB for workout with 10K data points
**Duration**: ~5 seconds

#### 6. HashMap Operations Test
**Test**: `test_hashmap_operations_no_leak`
**Purpose**: Verify HashMap operations don't leak over 10K iterations
**Acceptance**: <5MB growth
**Duration**: ~5 seconds

#### 7. String Allocations Test
**Test**: `test_string_allocations_no_leak`
**Purpose**: Verify string allocations don't accumulate over 100K iterations
**Acceptance**: <5MB growth
**Duration**: ~5 seconds

### Memory Tracking Implementation

#### Linux (most accurate)
Uses `/proc/self/status` to read VmRSS (Resident Set Size)

#### macOS
Uses `ps` command to read RSS

#### Windows
Not currently supported for native memory tracking (Valgrind unavailable)

---

## Stress Tests

Located in: `tests/stress_tests.rs`

### Test Categories

#### 1. Large File Handling
**Test**: `test_large_workout_file`
**Purpose**: Handle workout with 500K data points (~500MB)
**Acceptance**: <30 seconds processing time, no crashes
**Duration**: ~15 seconds

```bash
cargo test --release --test stress_tests test_large_workout_file -- --ignored --nocapture
```

#### 2. Rapid Import Burst
**Test**: `test_rapid_import_burst`
**Purpose**: Create 100 workouts in rapid sequence
**Acceptance**: <10 seconds for 100 files
**Duration**: ~5 seconds

```bash
cargo test --release --test stress_tests test_rapid_import_burst -- --ignored --nocapture
```

#### 3. Concurrent Operations
**Test**: `test_concurrent_operations`
**Purpose**: Process workouts concurrently across 4 threads
**Acceptance**: No panics, all 100 workouts processed correctly
**Duration**: ~5 seconds

```bash
cargo test --release --test stress_tests test_concurrent_operations -- --ignored --nocapture
```

#### 4. Extremely Long Workout
**Test**: `test_extremely_long_workout`
**Purpose**: Handle 30-hour ultra-endurance workout
**Acceptance**: No crashes, data integrity maintained
**Duration**: ~10 seconds

#### 5. Memory Pressure Simulation
**Test**: `test_memory_pressure`
**Purpose**: 10 batches of 50 large workouts to simulate high memory pressure
**Acceptance**: No crashes, memory released properly
**Duration**: ~30 seconds

#### 6. Rapid Allocation Cycles
**Test**: `test_rapid_allocation_cycles`
**Purpose**: 1000 allocation/deallocation cycles
**Acceptance**: Stable performance, no degradation
**Duration**: ~10 seconds

#### 7. Data Integrity Test
**Test**: `test_data_integrity_large_dataset`
**Purpose**: Verify data integrity with 100K data points
**Acceptance**: All data points present and correct
**Duration**: ~10 seconds

### Edge Case Tests (not ignored, run in standard test suite)

#### 1. Missing Fields
**Test**: `test_workout_with_missing_fields`
**Purpose**: Graceful handling of workouts with missing optional fields

#### 2. Empty Workout
**Test**: `test_empty_workout`
**Purpose**: Handle edge case of 0-duration workout

#### 3. Single Data Point
**Test**: `test_single_datapoint_workout`
**Purpose**: Handle minimal workout with 1 data point

#### 4. Invalid String Data
**Test**: `test_invalid_string_data`
**Purpose**: Handle special characters, null bytes, Unicode

---

## Valgrind Integration

### Installation

```bash
# Linux
sudo apt-get install valgrind

# macOS (note: may have compatibility issues on Apple Silicon)
brew install valgrind
```

### Running Valgrind

```bash
# Run all memory leak tests with Valgrind
./scripts/valgrind_check.sh

# Run specific test file
./scripts/valgrind_check.sh memory_leak_tests
```

### Valgrind Configuration

The script runs Valgrind with:
- `--leak-check=full`: Detailed leak information
- `--show-leak-kinds=all`: Show all types of leaks
- `--track-origins=yes`: Track origin of uninitialized values
- `--verbose`: Detailed output
- `--error-exitcode=1`: Exit with error code on leaks

### Interpreting Results

#### ✅ Clean Result
```
definitely lost: 0 bytes in 0 blocks
indirectly lost: 0 bytes in 0 blocks
possibly lost: 0 bytes in 0 blocks
still reachable: X bytes in Y blocks
```

#### ⚠️ Possible Leak
```
possibly lost: 32 bytes in 1 blocks
```
May be false positive, investigate further.

#### ❌ Definite Leak
```
definitely lost: 1,024 bytes in 4 blocks
```
Must be fixed before release.

### Valgrind Reports

Reports are saved to: `target/valgrind-reports/valgrind-report-YYYYMMDD-HHMMSS.txt`

---

## CI/CD Integration

### Workflow: `.github/workflows/memory-stress-tests.yml`

#### Trigger Conditions
1. **Weekly Schedule**: Sundays at 2 AM UTC
2. **Manual Dispatch**: Via GitHub Actions UI
3. **Main Branch Pushes**: When memory/stress test files change

#### Jobs

**1. Memory Leak Detection**
- Runs native memory leak tests
- Executes Valgrind analysis
- Uploads Valgrind reports as artifacts
- Fails if leaks detected

**2. Stress Testing**
- Runs comprehensive stress test suite
- Uploads detailed reports
- Validates all stress scenarios

**3. Edge Case Validation**
- Tests missing fields, empty workouts, etc.
- Fast execution, catches regressions

**4. Performance Baseline**
- Measures performance of critical tests
- Ensures no performance degradation
- Times out tests to prevent hangs

#### Artifacts

Artifacts are retained for 30 days:
- **Valgrind Reports**: `valgrind-reports/`
- **Stress Test Reports**: `stress-test-reports/`

Download from GitHub Actions run page.

---

## Acceptance Criteria

### Memory Leak Tests

| Test | Max Growth | Duration |
|------|-----------|----------|
| Sequential Import (1000 files) | <10MB | <60s |
| Batch Iterations (10 batches) | <5MB/iteration | <30s |
| TSS Calculation (1000x) | <5MB | <15s |
| PMC Calculation (10x 365 days) | <5MB | <20s |
| Large Workout (10K points) | <50MB | <10s |
| HashMap Operations (10K) | <5MB | <10s |
| String Allocations (100K) | <5MB | <10s |

### Stress Tests

| Test | Acceptance Criteria | Timeout |
|------|-------------------|---------|
| Large Workout (500K points) | No crashes, <30s | 60s |
| Rapid Import (100 files) | All imported, <10s | 30s |
| Concurrent (4 threads) | 100 workouts correct | 30s |
| Ultra-Long (30 hours) | Data integrity | 60s |
| Memory Pressure (500 workouts) | No crashes | 120s |
| Allocation Cycles (1000x) | Stable performance | 60s |
| Data Integrity (100K points) | 100% accuracy | 60s |

### Valgrind Criteria

- **Definitely lost**: 0 bytes
- **Indirectly lost**: 0 bytes
- **Possibly lost**: <100 bytes (investigate)
- **Still reachable**: Any (cleaned up at exit)

---

## Troubleshooting

### Common Issues

#### 1. Valgrind Not Available

**Symptom**: `valgrind: command not found`

**Solution**:
```bash
# Linux
sudo apt-get install valgrind

# macOS
brew install valgrind
```

**macOS Note**: Valgrind may not work on Apple Silicon (M1/M2/M3). Use native memory tests instead.

#### 2. Tests Timeout

**Symptom**: Test exceeds timeout limit

**Solutions**:
- Check if running in debug mode (use `--release`)
- Verify system resources available
- Review test parameters (reduce iterations if needed)

```bash
# Ensure release mode
cargo test --release --test stress_tests test_name -- --ignored
```

#### 3. High Memory Usage

**Symptom**: Memory usage exceeds thresholds

**Investigation**:
1. Check for large allocations not being freed
2. Review data structure sizes
3. Look for accumulating collections
4. Use Valgrind for detailed analysis

```bash
# Run Valgrind for detailed analysis
./scripts/valgrind_check.sh
```

#### 4. Intermittent Failures

**Symptom**: Tests pass sometimes, fail others

**Causes**:
- System resource contention
- Timing-dependent issues
- Race conditions in concurrent tests

**Solutions**:
- Run with `--test-threads=1` for sequential execution
- Increase timeouts
- Add synchronization to concurrent tests

```bash
# Force sequential execution
cargo test --release --test memory_leak_tests -- --ignored --test-threads=1
```

#### 5. False Positive Leaks

**Symptom**: Valgrind reports leaks in standard library

**Explanation**: Some "still reachable" leaks are normal (global allocators, static data)

**Action**: Focus on "definitely lost" and "indirectly lost" categories

---

## Performance Optimization Tips

### 1. Use Release Mode
Always run with `--release` flag for accurate performance measurements:
```bash
cargo test --release --test stress_tests -- --ignored
```

### 2. Single-Threaded for Memory Tests
Memory tracking is more accurate with single-threaded execution:
```bash
cargo test --release --test memory_leak_tests -- --ignored --test-threads=1
```

### 3. Reduce System Load
Close unnecessary applications before running tests for consistent results.

### 4. Warm-Up Runs
First run may be slower due to disk caching. Run twice for accurate measurements.

---

## Reporting Issues

When reporting memory or performance issues, include:

1. **System Information**:
   - OS and version
   - RAM available
   - CPU model

2. **Test Output**:
   - Full test output with `--nocapture`
   - Timing information
   - Memory growth measurements

3. **Valgrind Report** (if applicable):
   - Full Valgrind output
   - Leak summary
   - Stack traces

4. **Reproduction Steps**:
   - Exact command used
   - Any modifications to test parameters
   - Consistency of failure

---

## References

- [Valgrind Documentation](https://valgrind.org/docs/manual/quick-start.html)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [TrainRS Benchmark Workflow Setup](BENCHMARK_WORKFLOW_SETUP.md)

---

## License

Test files in this directory are used solely for testing purposes. See main repository LICENSE for details.
