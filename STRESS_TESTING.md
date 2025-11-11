# Stress Testing and Memory Profiling Framework

## Overview

The TrainRS stress testing framework (`src/stress_testing.rs`) provides comprehensive memory profiling and stress testing capabilities to ensure production stability and prevent memory leaks during bulk import operations.

## Architecture

### Core Components

#### MemoryMetrics
Captures platform-specific memory usage:
- **RSS (Resident Set Size)**: Actual physical memory used
- **VMS (Virtual Memory Size)**: Total virtual memory allocated
- **Peak RSS**: Maximum RSS reached during execution
- **Allocation Count**: Number of allocations

```rust
pub struct MemoryMetrics {
    pub rss_bytes: u64,
    pub vms_bytes: u64,
    pub peak_rss_bytes: u64,
    pub allocation_count: u64,
}
```

**Platform Support:**
- Linux: Reads `/proc/self/status` for accurate memory metrics
- macOS: Uses `ps` command with `-A` flag
- Fallback: Uses estimated values when system calls fail

#### StressTestMetrics
Aggregates performance and memory data across an entire stress test:

```rust
pub struct StressTestMetrics {
    pub files_processed: u64,
    pub successful_imports: u64,
    pub failed_imports: u64,
    pub total_workouts: u64,
    pub total_duration: Duration,
    pub memory_start: MemoryMetrics,
    pub memory_end: MemoryMetrics,
    pub memory_snapshots: Vec<(Duration, MemoryMetrics)>,
    pub processing_times_ms: Vec<u128>,
    pub peak_memory_mb: f64,
}
```

**Key Metrics Calculated:**
- `throughput_files_per_sec()`: Files processed per second
- `avg_processing_time_ms()`: Average file processing time
- `memory_growth_mb()`: Total memory increase (end - start)
- `success_rate_percent()`: Percentage of successful imports
- `has_memory_leak()`: Detects potential memory leaks

#### Memory Leak Detection

Detects memory leaks using growth trend analysis:

```rust
pub fn has_memory_leak(&self) -> bool {
    // Returns true if:
    // 1. Memory growth > 15% from start to end
    // 2. Consistent upward trend across snapshots
    // 3. No significant drops (indicating cleanup)
}
```

**Algorithm:**
1. Analyzes memory snapshots across test duration
2. Calculates growth trend (slope of memory vs time)
3. Flags leak if growth consistently exceeds 15% threshold
4. Requires multiple snapshots for statistical confidence

#### StressTestScenario
Represents a single stress test execution:

```rust
pub struct StressTestScenario {
    config: StressTestConfig,
    start_time: Option<Instant>,
    metrics: StressTestMetrics,
}
```

**Lifecycle:**
1. Create: `StressTestHarness::create_scenario(config)`
2. Start: `scenario.start()` - captures initial memory
3. Record: `scenario.record_success()` / `scenario.record_failure()`
4. Finish: `scenario.finish()` - returns final metrics

#### StressTestConfig
Configures test parameters:

```rust
pub struct StressTestConfig {
    pub num_files: usize,              // How many files to process
    pub workouts_per_file: usize,      // Workouts per file
    pub parallel_processing: bool,     // Use threading
    pub num_threads: Option<usize>,    // Thread count (if parallel)
    pub enable_caching: bool,          // Use cache optimization
    pub snapshot_interval: Duration,   // Memory sample frequency
    pub timing_sample_rate: usize,     // Sample 1 in N processing times
}
```

**Default Config (Smoke Test):**
- 100 files, 1 workout/file
- Parallel processing enabled
- Caching enabled
- 10% sampling for processing times

#### StressTestHarness
Provides pre-configured test scenarios:

```rust
pub impl StressTestHarness {
    // Smoke test: 100 files, quick validation
    pub fn smoke_test() -> StressTestScenario

    // Standard test: 1000 files, realistic workload
    pub fn standard_test() -> StressTestScenario

    // Heavy test: 10000 files, sustained stress
    pub fn heavy_test() -> StressTestScenario

    // Extreme test: 50000 files, maximum stress
    pub fn extreme_test() -> StressTestScenario
}
```

### Test Scenarios

| Scenario | Files | Threads | Use Case |
|----------|-------|---------|----------|
| Smoke | 100 | Default | Quick validation, CI/CD |
| Standard | 1,000 | 4 | Normal workload testing |
| Heavy | 10,000 | 8 | Stress testing |
| Extreme | 50,000 | 16 | Maximum load testing |

## Usage Examples

### Basic Stress Test

```rust
use trainrs::stress_testing::StressTestHarness;

// Run standard stress test
let mut scenario = StressTestHarness::standard_test();
scenario.start();

// Simulate workouts
for i in 0..1000 {
    let time_ms = simulate_import(i);
    if success {
        scenario.record_success(5, time_ms);
    } else {
        scenario.record_failure();
    }
}

let metrics = scenario.finish();
println!("{}", metrics.pretty_print());
```

### Custom Configuration

```rust
use trainrs::stress_testing::{StressTestHarness, StressTestConfig};
use std::time::Duration;

let config = StressTestConfig {
    num_files: 5000,
    workouts_per_file: 3,
    parallel_processing: true,
    num_threads: Some(8),
    enable_caching: true,
    snapshot_interval: Duration::from_secs(10),
    timing_sample_rate: 5, // Sample 20% of times
};

let mut scenario = StressTestHarness::create_scenario(config);
scenario.start();
// ... run test ...
let metrics = scenario.finish();
```

### Memory Leak Detection

```rust
let metrics = scenario.finish();

if metrics.has_memory_leak() {
    eprintln!("⚠️ Potential memory leak detected!");
    eprintln!("Memory growth: {:.2}MB", metrics.memory_growth_mb());
} else {
    println!("✓ Memory usage stable");
}
```

## Test Coverage

### Unit Tests (5 tests)
Located in `src/stress_testing.rs`:
- `test_memory_metrics_conversion`: Unit conversions (MB, KB)
- `test_stress_test_config_defaults`: Default configuration values
- `test_stress_test_scenarios`: Scenario creation and setup
- `test_memory_leak_detection`: Leak detection algorithm
- `test_stress_test_metrics_throughput`: Throughput calculations

### Integration Tests (19 tests)
Located in `tests/stress_testing_integration.rs`:
- Configuration builders and defaults
- Scenario creation and lifecycle
- Success/failure tracking
- Memory metrics accumulation
- Leak detection (clean and leaking scenarios)
- Concurrent scenarios
- Error rate calculation
- Metrics formatting and calculation

## Metrics Interpretation

### Throughput
```
throughput_files_per_sec = files_processed / total_seconds
```
Indicates processing speed. Higher is better (typically 10-100 files/sec).

### Memory Growth
```
memory_growth_mb = (memory_end - memory_start) / 1024 / 1024
```
Should stay below 10-15% of initial memory for leak-free operation.

### Success Rate
```
success_rate = (successful / total) * 100
```
Should be ≥99% for production readiness.

### Average Processing Time
```
avg_time_ms = sum(processing_times) / files_processed
```
Calculated from sampled timing data. Smaller is better.

## Performance Characteristics

### Memory Overhead
- Base overhead: ~1-2MB for framework
- Per-file overhead: ~100-500 bytes
- Snapshots: ~1KB per snapshot

### Sampling Strategy
- **Processing Times**: Configurable sampling (default 10%)
  - Reduces overhead while maintaining accuracy
  - Statistically valid for distributions

- **Memory Snapshots**: Captured at intervals
  - Default: every 50 files (configurable)
  - Enables trend analysis for leak detection

## Verification Checklist

When running stress tests:

✓ Verify all tests pass (both unit and integration)
✓ Check memory growth is <15% of initial
✓ Confirm success rate ≥99%
✓ Validate throughput meets expectations
✓ Review leak detection flags (should be false)
✓ Check for resource cleanup (final metrics)

## Integration with CI/CD

```yaml
# Example GitHub Actions test
- name: Run stress tests
  run: |
    cargo test --lib stress_testing
    cargo test --test stress_testing_integration

- name: Run heavy load test
  run: |
    STRESS_TEST_HEAVY=1 cargo test --test stress_test_heavy
```

## Advanced Configuration

### Tuning for Specific Scenarios

**For quick CI/CD validation:**
```rust
StressTestHarness::smoke_test()  // 100 files, fast
```

**For continuous integration:**
```rust
StressTestHarness::standard_test()  // 1000 files, balanced
```

**For weekly performance regression:**
```rust
StressTestHarness::heavy_test()  // 10000 files, comprehensive
```

**For production readiness validation:**
```rust
StressTestHarness::extreme_test()  // 50000 files, maximum stress
```

### Custom Thresholds

Modify memory leak threshold in `has_memory_leak()`:
- Current: 15% growth
- Aggressive: 5% growth (catch smaller leaks)
- Relaxed: 25% growth (allow temporary increases)

## Troubleshooting

### High Memory Growth
- Check for unbounded collections
- Verify caching isn't growing indefinitely
- Confirm parallel threads are cleaning up

### Low Throughput
- Increase parallelism (`num_threads`)
- Check I/O bottlenecks
- Verify cache is enabled

### Intermittent Failures
- Memory snapshots may miss short spikes
- Increase `snapshot_interval` for better granularity
- Check system load (other processes)

### False Positives
- Small test sets (<100 files) may show higher growth
- Use standard_test or larger for reliable detection
- Warmup runs help stabilize memory

## Future Enhancements

- [ ] Heap allocation profiling with `valgrind` integration
- [ ] Real-time memory graphing
- [ ] Automatic threshold tuning
- [ ] Comparison with baseline metrics
- [ ] Per-module memory attribution
- [ ] Cache efficiency metrics
- [ ] Thread pool performance analysis

## References

- Issue #132: Memory Leak Detection & Stress Testing
- Related: Issue #133 (Parallel Processing), Issue #134 (Caching)
- Test files: `tests/stress_testing_integration.rs`
- Implementation: `src/stress_testing.rs`
