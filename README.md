# TrainRS - Training Load Analysis CLI

A powerful command-line tool for analyzing training data and calculating sports science metrics, built in Rust for performance and precision.

## Overview

TrainRS is designed for athletes, coaches, and sports scientists who need accurate training load analysis using established sports science methodologies. The tool processes workout data and calculates key performance indicators including Training Stress Score (TSS), Chronic Training Load (CTL), Acute Training Load (ATL), and Training Stress Balance (TSB).

## Sports Science Background

### Training Load Metrics

Training load quantification is fundamental to optimizing athletic performance while minimizing injury risk. TrainRS implements industry-standard metrics:

- **Training Stress Score (TSS)**: Quantifies the training stress of a workout based on intensity and duration
- **Intensity Factor (IF)**: Ratio of normalized power to functional threshold power
- **Normalized Power (NP)**: Power-based equivalent of steady-state effort for variable-intensity workouts
- **Chronic Training Load (CTL)**: 42-day exponentially weighted average representing fitness
- **Acute Training Load (ATL)**: 7-day exponentially weighted average representing fatigue
- **Training Stress Balance (TSB)**: Difference between CTL and ATL, indicating form/freshness

### Periodization Support

The tool supports various periodization models:
- Linear periodization analysis
- Block periodization tracking
- Polarized training distribution analysis
- Tapering and peaking optimization

## Features

- **Multi-format Import**: Support for CSV, JSON, and FIT files
- **Precise Calculations**: Uses `rust_decimal` for accurate power and duration calculations
- **Statistical Analysis**: Advanced metrics using `statrs` for trend analysis
- **Flexible Export**: Output to multiple formats (CSV, JSON, HTML, PDF)
- **Rich Visualization**: Terminal-based charts and tables with optional plotting
- **Configuration Management**: Customizable athlete profiles and settings

## Installation

### Prerequisites

- Rust 1.80+ (install from [rustup.rs](https://rustup.rs/))

### From Source

```bash
git clone https://github.com/yourusername/trainrs.git
cd trainrs
cargo build --release
```

The binary will be available at `target/release/trainrs`.

## Quick Start

### 1. Import Training Data

```bash
# Import from CSV
trainrs import --file workouts.csv --format csv

# Auto-detect format
trainrs import --file data.json
```

### 2. Calculate Metrics

```bash
# Calculate all metrics for the last 30 days
trainrs calculate --from 2024-08-01 --to 2024-08-31

# Specific athlete analysis
trainrs calculate --athlete john_doe
```

### 3. Analyze Trends

```bash
# 6-week analysis with predictions
trainrs analyze --period 42 --predict

# Quick current status
trainrs display --format summary
```

### 4. Export Results

```bash
# Export to CSV
trainrs export --output results.csv --format csv

# Generate HTML report
trainrs export --output report.html --format html
```

## Usage Examples

### Weekly Training Load Analysis

```bash
# Import week's data
trainrs import --file week_data.csv

# Analyze training distribution
trainrs analyze --period 7

# Display formatted results
trainrs display --format table --limit 7
```

### Monthly Performance Review

```bash
# Calculate monthly metrics
trainrs calculate --from 2024-08-01 --to 2024-08-31

# Export comprehensive report
trainrs export --output august_report.html --format html
```

### Configuration

```bash
# Set functional threshold power
trainrs config --set ftp=250

# Set athlete profile
trainrs config --set athlete.name="John Doe"

# View all settings
trainrs config --list
```

## Data Format

### Input CSV Format

```csv
date,duration_seconds,avg_power,max_power,normalized_power,tss,if
2024-08-15,3600,180,320,195,85,0.78
2024-08-16,5400,160,280,175,120,0.70
```

### Supported File Types

- **CSV**: Comma-separated values with headers
- **JSON**: Structured workout objects
- **FIT**: Garmin/ANT+ FIT files (planned)

## Architecture

TrainRS is built with performance and precision in mind:

- **`rust_decimal`**: Ensures precise calculations for power and duration metrics
- **`statrs`**: Provides robust statistical functions for trend analysis
- **`chrono`**: Handles complex date/time operations for training periodization
- **`clap`**: Modern CLI interface with comprehensive help and validation
- **`tabled`**: Beautiful terminal output formatting

## Development

### Building

```bash
# Development build
cargo build

# Optimized release build
cargo build --release

# With chart generation feature
cargo build --features charts
```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test module
cargo test calculations
```

### Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Roadmap

- [ ] FIT file import support
- [ ] Advanced visualization with `plotters`
- [ ] Machine learning predictions
- [ ] Web dashboard interface
- [ ] Real-time data streaming
- [ ] Integration with popular training platforms

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## References

- Coggan, A. R. (2003). Training and racing using a power meter
- Friel, J. (2012). The power meter handbook
- Seiler, S. (2010). What is best practice for training intensity and duration distribution in endurance athletes?
