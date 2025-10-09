# Performance Benchmarks and Baselines

This document describes the performance benchmarks, baselines, and regression detection for TrainRS.

## Overview

TrainRS uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for comprehensive performance benchmarking with automated regression detection. Benchmarks run automatically in CI on every push and pull request.

## Performance Targets

### Core Calculations

| Benchmark | Target | Baseline | Notes |
|-----------|--------|----------|-------|
| TSS Calculation (1 workout) | < 100 μs | TBD | Power-based TSS for 1-hour workout |
| TSS Calculation (100 workouts) | < 10 ms | TBD | Batch processing |
| PMC Series (30 days) | < 1 ms | TBD | CTL/ATL/TSB calculation |
| PMC Series (365 days) | < 10 ms | TBD | Full year analysis |
| Normalized Power (1 hour) | < 5 ms | TBD | 3600 data points at 1Hz |
| Power Curve (4 hours) | < 50 ms | TBD | MMP curve calculation |

### Data Processing

| Benchmark | Target | Baseline | Notes |
|-----------|--------|----------|-------|
| FIT Parsing (1MB) | > 50 MB/s | TBD | Throughput target |
| FIT Parsing (50MB) | > 50 MB/s | TBD | Large file handling |
| JSON Serialization (100 workouts) | < 10 ms | TBD | Export performance |
| JSON Deserialization (100 workouts) | < 15 ms | TBD | Import performance |

### Database Operations

| Benchmark | Target | Baseline | Notes |
|-----------|--------|----------|-------|
| Insert (10 workouts) | < 10 ms | TBD | Batch insert |
| Insert (1000 workouts) | < 500 ms | TBD | Large batch |
| Query Date Range (100 workouts) | < 10 ms | TBD | Simple query |
| Query Date Range (10k workouts) | < 100 ms | TBD | Large dataset |
| Time-Series Query (365 days) | < 50 ms | TBD | Annual aggregation |

### Batch Operations

| Benchmark | Target | Baseline | Notes |
|-----------|--------|----------|-------|
| Batch Import (10 files) | < 1 sec | TBD | Multiple file import |
| Batch Import (100 files) | < 10 sec | TBD | Large batch |
| Memory per Workout | < 5 MB | TBD | Memory efficiency |
| Dataset Creation (10k workouts) | < 2 sec | TBD | Memory allocation |

### Analysis Functions

| Benchmark | Target | Baseline | Notes |
|-----------|--------|----------|-------|
| Zone Analysis (Cycling) | < 5 ms | TBD | Power + HR zones |
| Pace Analysis (Running) | < 5 ms | TBD | Pace zones + GAP |
| Elevation Analysis | < 10 ms | TBD | Grade-adjusted calculations |
| Multisport Load (500 workouts) | < 50 ms | TBD | Cross-sport aggregation |

## Regression Detection

### Thresholds

- **Alert Threshold**: 5% performance degradation
- **Fail Threshold**: 10% performance degradation (configurable)
- **Improvement Recognition**: 5% performance improvement

### CI Integration

Performance benchmarks run on:
- Every push to `main` branch
- Every pull request
- Manual workflow dispatch

### Regression Alerts

When a performance regression >5% is detected:
1. GitHub comment added to PR with benchmark comparison
2. Issue creator (@jpequegn) is notified via `@mention`
3. PR can still be merged (warning only) unless >10% regression
4. Benchmark results stored for historical tracking

## Running Benchmarks Locally

### Run All Benchmarks

```bash
cargo bench --bench performance_benchmarks
```

### Run Specific Benchmark Group

```bash
# TSS calculations only
cargo bench --bench performance_benchmarks -- "TSS Calculation"

# Database operations only
cargo bench --bench performance_benchmarks -- "Database Operations"

# FIT parsing only
cargo bench --bench performance_benchmarks -- "FIT File Parsing"
```

### View Results

Criterion generates HTML reports in `target/criterion/`:

```bash
# Open the main report
open target/criterion/report/index.html
```

## Benchmark Results Storage

- **Main Branch**: Results automatically pushed to `gh-pages` branch
- **Pull Requests**: Compared against main branch baseline
- **Historical Data**: Tracked over time for trend analysis

## Performance Optimization Guidelines

### When to Optimize

1. Benchmark shows >5% regression
2. User-reported performance issues
3. Profiling reveals hotspots
4. New features add computational overhead

### Optimization Process

1. **Measure First**: Run benchmarks to establish baseline
2. **Profile**: Use `cargo flamegraph` or `perf` to find hotspots
3. **Optimize**: Make targeted changes
4. **Measure Again**: Verify improvement with benchmarks
5. **Document**: Update this file with new baselines

### Common Optimizations

- Use `rayon` for parallel processing of large datasets
- Cache expensive calculations (Normalized Power, Power Curves)
- Use database indexes for common queries
- Minimize allocations in hot paths
- Use `Decimal` arithmetic efficiently (avoid unnecessary conversions)

## Benchmark Categories

### 1. TSS Calculation Benchmarks

Tests the core Training Stress Score calculation with varying dataset sizes:
- 1, 10, 100, 1000 workouts
- Power-based, HR-based, and pace-based TSS
- Measures: throughput (workouts/second)

### 2. PMC Calculation Benchmarks

Tests Performance Management Chart calculations:
- 7, 30, 90, 365 days of training data
- CTL, ATL, TSB calculations
- Measures: time per calculation, throughput

### 3. Power Analysis Benchmarks

Tests power-specific calculations:
- Power curves for 30min to 4-hour workouts
- Normalized Power for 6min to 10-hour datasets
- MMP (Mean Maximal Power) curves
- Measures: time per analysis, data points/second

### 4. Zone Analysis Benchmarks

Tests training zone calculations:
- Cycling (power + heart rate zones)
- Running (pace + heart rate zones)
- Swimming (pace zones)
- Measures: time per workout analysis

### 5. Running Analysis Benchmarks

Tests running-specific calculations:
- Pace analysis for 30min to 2-hour runs
- Elevation analysis with grade-adjusted pace
- Stride metrics and running dynamics
- Measures: time per analysis

### 6. Multisport Analysis Benchmarks

Tests cross-sport aggregation:
- 10, 50, 100, 500 workouts across sports
- Combined training load
- Sport-specific metrics aggregation
- Measures: throughput

### 7. Data Serialization Benchmarks

Tests export/import performance:
- JSON serialization/deserialization
- 10, 100, 1000 workout datasets
- Measures: MB/s throughput

### 8. Memory Usage Benchmarks

Tests memory efficiency:
- Large dataset creation (1k, 5k, 10k workouts)
- Memory allocation patterns
- Measures: time and memory per operation

### 9. FIT File Parsing Benchmarks

Tests file import performance:
- Small (1MB), medium (10MB), large (50MB) files
- Parse throughput measurement
- Measures: MB/s parsing speed

### 10. Database Operations Benchmarks

Tests database performance:
- Batch insert (10, 100, 1k, 10k workouts)
- Date range queries (100, 1k, 10k workouts)
- Measures: operations/second, latency

### 11. Batch Import Benchmarks

Tests multi-file import:
- 10, 50, 100 file batches
- Parallel processing efficiency
- Measures: files/second throughput

### 12. Time-Series Query Benchmarks

Tests temporal queries:
- 30, 90, 365 day ranges
- Weekly/monthly aggregations
- Measures: query latency

## Establishing Baselines

Initial baselines will be established after merging this feature. To set baselines:

1. Merge performance benchmark PR to main
2. CI will run benchmarks and store results
3. Update this document with baseline measurements
4. Future runs compare against these baselines

## Future Improvements

- [ ] Add memory profiling benchmarks
- [ ] Benchmark parallel processing efficiency
- [ ] Add real FIT file parsing benchmarks (with fixtures)
- [ ] Benchmark caching strategies
- [ ] Add comparative benchmarks (vs. other tools)
- [ ] Track performance trends over releases
- [ ] Add benchmark for VO2max estimation
- [ ] Add benchmark for training effect calculation
- [ ] Benchmark validation rule performance

## References

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/index.html)
- [GitHub Action Benchmark](https://github.com/benchmark-action/github-action-benchmark)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

## License

Performance benchmarks are part of TrainRS and licensed under the same terms as the main project.
