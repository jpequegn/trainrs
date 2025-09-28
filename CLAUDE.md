# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building and Testing
```bash
# Standard development build
cargo build

# Optimized release build
cargo build --release

# Run all tests
cargo test

# Run specific test module
cargo test tss::tests
cargo test pmc::tests

# Run single test
cargo test test_power_tss_calculation

# Run tests with output
cargo test -- --nocapture

# Check compilation without building
cargo check

# Run benchmarks
cargo bench
```

### Code Quality
```bash
# Check for clippy warnings
cargo clippy

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Running the CLI
```bash
# Build and run
cargo run -- --help

# Import workout data
cargo run -- import --file workouts.csv --format csv

# Calculate training metrics
cargo run -- calculate --from 2024-08-01 --to 2024-08-31

# Display current status
cargo run -- display --format table
```

## Architecture Overview

TrainRS is a sports science-focused training analysis CLI tool built around precise calculation of training load metrics. The architecture follows a modular design centered on domain-specific calculations.

### Core Calculation Engines

**TSS (Training Stress Score) Engine** (`src/tss.rs`):
- Primary calculation engine implementing the sports science formula: `TSS = (duration_hours × IF²) × 100`
- Supports multi-modal TSS calculation: power-based (cycling), heart rate-based (hrTSS), and pace-based (rTSS/sTSS)
- Uses intelligent fallback hierarchy: power → pace → heart rate → estimation
- Implements normalized power calculation with 30-second rolling averages and fourth-root computation
- Critical: Uses `rust_decimal::Decimal` for financial-grade precision in sports calculations

**PMC (Performance Management Chart) Engine** (`src/pmc.rs`):
- Implements Chronic Training Load (CTL), Acute Training Load (ATL), and Training Stress Balance (TSB)
- Uses exponentially weighted moving averages: CTL (42-day), ATL (7-day)
- Processes time-series TSS data into fitness/fatigue/form metrics

**Training Zones Engine** (`src/zones.rs`):
- Multi-sport zone calculation: heart rate, power, and pace zones
- Supports sport-specific threshold definitions (FTP, LTHR, threshold pace)

### Data Architecture

**Core Models** (`src/models.rs`):
- `Workout`: Complete workout record with time-series data (`DataPoint` vector)
- `AthleteProfile`: Athlete-specific thresholds (FTP, LTHR, threshold pace)
- `WorkoutSummary`: Calculated metrics (TSS, IF, normalized power)
- All numeric calculations use `rust_decimal::Decimal` for precision

**Multi-Sport Support** (`src/multisport.rs`):
- Sport-specific calculation routing
- Unified interface across cycling, running, swimming, triathlon

### Import/Export System

**Import Pipeline** (`src/import/`):
- Multi-format support: CSV, JSON, FIT files
- Streaming import for large datasets (`src/import/streaming.rs`)
- Data validation and normalization (`src/import/validation.rs`)

**Export Pipeline** (`src/export/`):
- Multiple output formats: CSV, JSON, text reports
- Structured data export for external analysis

### Data Management

**Database Layer** (`src/database.rs`):
- SQLite-based storage with schema management
- Supports workout data, athlete profiles, and calculated metrics

**Data Integrity** (`src/data_management.rs`):
- Duplicate detection and cleanup
- Data integrity validation
- Backup and archival systems

## Testing Strategy

### Test Structure
- **Unit Tests**: Embedded in module files (e.g., `src/tss.rs` contains TSS calculation tests)
- **Integration Tests**: `tests/integration_tests.rs` for complete workflow testing
- **Property-Based Tests**: Using `proptest` for TSS calculation validation
- **Benchmarks**: `benches/performance_benchmarks.rs` for performance regression testing

### Critical Test Areas
- **TSS Calculation Accuracy**: Tests validate against known sports science formulas
- **Overflow Prevention**: Property tests ensure no integer overflow in power calculations
- **Multi-Sport Compatibility**: Integration tests verify calculation accuracy across sports
- **Data Precision**: Tests validate `Decimal` precision in financial-grade calculations

### Running Specific Test Categories
```bash
# TSS calculation tests (critical)
cargo test tss::tests

# Property-based tests (may take longer)
cargo test test_power_tss_properties

# Integration tests
cargo test --test integration_tests
```

## Development Guidelines

### Calculation Precision
- **Always use `rust_decimal::Decimal`** for any sports science calculations (TSS, power, pace)
- Use `rust_decimal_macros::dec!` macro for decimal literals: `dec!(250.0)`
- Avoid `f64` in business logic; only use for intermediate mathematical operations (sqrt, etc.)

### TSS Calculation Implementation
- Formula: `TSS = (duration_hours × IF²) × 100` where `IF = NP/FTP`
- Normalized Power: Fourth root of rolling 30-second power averages raised to fourth power
- Always validate against sports science references when modifying calculations

### Multi-Sport Considerations
- Each sport has different primary metrics: cycling (power), running (pace), general (heart rate)
- Implement sport-specific calculation paths with intelligent fallbacks
- Maintain consistent TSS scaling across sports (100 TSS = 1 hour at threshold)

### Error Handling
- Use `thiserror` for domain-specific error types (`TssError`, `PmcError`, etc.)
- Validate athlete profile completeness before calculations (FTP, LTHR, threshold pace)
- Handle missing data gracefully with fallback calculation methods

### Performance Considerations
- Use `rayon` for parallel processing of large datasets
- Implement streaming imports for memory efficiency with large workout files
- Cache calculated metrics to avoid recomputation

## CLI Architecture

The CLI uses `clap` with a hierarchical command structure:
- Global options: `--athlete`, `--verbose`, `--format`
- Subcommands: `import`, `calculate`, `display`, `export`, `zones`, `config`
- Each subcommand has sport-specific and calculation-specific options

Commands are implemented in `src/main.rs` with extensive help text and validation.