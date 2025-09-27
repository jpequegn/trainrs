# CLI Command Reference

Complete reference for all TrainRS command-line interface commands with examples and detailed explanations.

## Global Options

Available with all commands:

```bash
-c, --config <FILE>      # Sets a custom config file
-a, --athlete <ATHLETE>  # Specify athlete profile name or ID
--data-dir <DIR>         # Custom data directory path
-v, --verbose...         # Increase verbosity of output
-q, --quiet              # Suppress non-essential output
--format <FORMAT>        # Output format (table, json, csv) [default: table]
-h, --help               # Print help
-V, --version            # Print version
```

## Commands Overview

| Command | Purpose | Key Options |
|---------|---------|-------------|
| `import` | Import workout data | `--file`, `--format`, `--validate` |
| `calculate` | Calculate training metrics | `--from`, `--to`, `--athlete` |
| `analyze` | Analyze training patterns | `--period`, `--predict` |
| `export` | Export data and reports | `--output`, `--format` |
| `display` | Show metrics in terminal | `--format`, `--limit` |
| `zones` | Manage training zones | `--sport`, `--method` |
| `summary` | Generate summaries | `--period`, `--detail` |
| `pmc` | Performance Management Chart | `--period`, `--chart` |
| `power` | Power analysis | `--curve`, `--analysis` |
| `running` | Running analysis | `--pace`, `--elevation` |
| `multi-sport` | Multi-sport analysis | `--sports`, `--combined` |
| `training-plan` | Training planning | `--duration`, `--goal` |
| `config` | Application settings | `--set`, `--get`, `--list` |
| `athlete` | Athlete profiles | `--create`, `--update` |

---

## `import` - Import Workout Data

Import training data from various file formats.

### Basic Usage

```bash
# Import CSV file (auto-detect format)
trainrs import --file workouts.csv

# Import with explicit format
trainrs import --file data.json --format json

# Import with validation only (don't save)
trainrs import --file data.csv --validate-only
```

### Supported Formats

#### CSV Import
```bash
# Standard CSV with headers
trainrs import --file workouts.csv --format csv

# Custom delimiter
trainrs import --file workouts.txt --format csv --delimiter ";"

# Skip header row
trainrs import --file data.csv --skip-header
```

Expected CSV format:
```csv
date,duration_seconds,avg_power,max_power,normalized_power,tss,if
2024-08-15,3600,180,320,195,85,0.78
2024-08-16,5400,160,280,175,120,0.70
```

#### JSON Import
```bash
# Import JSON workout array
trainrs import --file workouts.json --format json

# Import single workout object
trainrs import --file single_workout.json --format json
```

#### GPX Import
```bash
# Import GPS track data
trainrs import --file ride.gpx --format gpx

# Import with sport specification
trainrs import --file run.gpx --format gpx --sport running
```

### Advanced Options

```bash
# Import to specific athlete profile
trainrs import --file data.csv --athlete john_doe

# Batch import directory
trainrs import --directory ./workout_files/ --recursive

# Import with data validation
trainrs import --file data.csv --strict-validation

# Preview import without saving
trainrs import --file data.csv --dry-run
```

---

## `calculate` - Calculate Training Metrics

Calculate TSS, IF, NP, and other training metrics for specified periods.

### Basic Usage

```bash
# Calculate for last 30 days
trainrs calculate --days 30

# Calculate for specific date range
trainrs calculate --from 2024-08-01 --to 2024-08-31

# Calculate for specific athlete
trainrs calculate --athlete john_doe --days 14
```

### Calculation Types

```bash
# Calculate all metrics
trainrs calculate --all

# Calculate specific metrics only
trainrs calculate --metrics tss,if,np

# Calculate with sport-specific adaptations
trainrs calculate --sport cycling --days 7

# Recalculate existing data
trainrs calculate --recalculate --from 2024-08-01
```

### Output Options

```bash
# Save results to file
trainrs calculate --days 30 --output calculations.csv

# Display with custom format
trainrs calculate --days 7 --format json

# Show detailed breakdown
trainrs calculate --days 14 --detailed

# Include raw data points
trainrs calculate --days 7 --include-raw
```

---

## `analyze` - Analyze Training Patterns

Perform advanced analysis of training patterns, trends, and distributions.

### Trend Analysis

```bash
# Analyze fitness trends (42-day CTL)
trainrs analyze --trend fitness --days 90

# Analyze fatigue patterns (7-day ATL)
trainrs analyze --trend fatigue --days 30

# Combined fitness and fatigue analysis
trainrs analyze --trend both --days 60
```

### Distribution Analysis

```bash
# Training intensity distribution
trainrs analyze --distribution intensity --days 30

# Training zone distribution by time
trainrs analyze --distribution zones --metric time

# Training zone distribution by TSS
trainrs analyze --distribution zones --metric tss
```

### Predictive Analysis

```bash
# Predict performance based on trends
trainrs analyze --predict --horizon 14

# Predict with confidence intervals
trainrs analyze --predict --confidence 95

# What-if scenario analysis
trainrs analyze --scenario --target-ctl 80
```

### Pattern Recognition

```bash
# Detect training patterns
trainrs analyze --patterns --days 90

# Identify periodization blocks
trainrs analyze --blocks --days 180

# Recovery pattern analysis
trainrs analyze --recovery --days 60
```

---

## `export` - Export Data and Reports

Export training data and analysis reports in various formats.

### Basic Export

```bash
# Export to CSV
trainrs export --output results.csv --format csv

# Export to JSON
trainrs export --data workouts --output data.json

# Export specific date range
trainrs export --from 2024-08-01 --to 2024-08-31 --output august.csv
```

### Report Generation

```bash
# Generate HTML report
trainrs export --report summary --output report.html

# Generate PDF report (requires additional dependencies)
trainrs export --report detailed --output report.pdf

# Generate training log
trainrs export --report log --days 30 --output training_log.html
```

### Data Selection

```bash
# Export specific athlete data
trainrs export --athlete john_doe --output john_data.csv

# Export specific sports
trainrs export --sports cycling,running --output multi_sport.csv

# Export calculated metrics only
trainrs export --data metrics --output metrics.json

# Export raw workout data
trainrs export --data raw --include-time-series --output raw_data.json
```

### Format Options

```bash
# TrainingPeaks compatible format
trainrs export --format trainingpeaks --output tp_data.csv

# Strava compatible format
trainrs export --format strava --output strava_data.json

# Custom format template
trainrs export --template custom.json --output custom_data.csv
```

---

## `display` - Display Metrics in Terminal

Show training metrics and summaries in formatted terminal output.

### Summary Views

```bash
# Current training status
trainrs display --status

# Weekly summary
trainrs display --summary --days 7

# Monthly overview
trainrs display --summary --days 30 --detailed
```

### Metric Display

```bash
# PMC chart in terminal
trainrs display --pmc --days 60

# Training zone distribution
trainrs display --zones --days 30

# Recent workouts table
trainrs display --workouts --limit 10

# Power curve display
trainrs display --power-curve --days 90
```

### Formatting Options

```bash
# Compact table format
trainrs display --format compact --limit 20

# JSON output for scripting
trainrs display --format json --summary

# CSV format for spreadsheets
trainrs display --format csv --workouts --limit 50

# Rich terminal formatting
trainrs display --rich --summary --days 14
```

---

## `zones` - Manage Training Zones

Configure and manage training zones and threshold values.

### Zone Configuration

```bash
# Set power zones (cycling)
trainrs zones --sport cycling --set-ftp 250

# Set heart rate zones
trainrs zones --sport running --set-lthr 165 --set-max-hr 190

# Set pace zones (running)
trainrs zones --sport running --set-threshold-pace 4:00
```

### Zone Display

```bash
# Show current zones
trainrs zones --show

# Show zones for specific sport
trainrs zones --sport cycling --show

# Export zones to file
trainrs zones --export zones.json
```

### Zone Calculation Methods

```bash
# Use percentage of max for HR zones
trainrs zones --sport running --method max-hr

# Use LTHR method for HR zones
trainrs zones --sport running --method lthr

# Auto-detect zones from data
trainrs zones --auto-detect --days 90
```

---

## `summary` - Generate Training Summaries

Generate comprehensive training summaries and reports.

### Period Summaries

```bash
# Weekly summary
trainrs summary --week

# Monthly summary
trainrs summary --month 8 --year 2024

# Custom period
trainrs summary --from 2024-08-01 --to 2024-08-31
```

### Summary Types

```bash
# Training load summary
trainrs summary --type load --days 30

# Performance summary
trainrs summary --type performance --days 90

# Zone distribution summary
trainrs summary --type zones --days 30

# Complete training summary
trainrs summary --type complete --days 60
```

### Output Formats

```bash
# Detailed text summary
trainrs summary --detailed --days 30

# Export summary to file
trainrs summary --days 30 --output summary.txt

# Generate markdown report
trainrs summary --format markdown --output report.md
```

---

## `pmc` - Performance Management Chart

Analyze and display Performance Management Chart (PMC) data.

### Basic PMC Analysis

```bash
# Show PMC for last 90 days
trainrs pmc --days 90

# PMC with trend analysis
trainrs pmc --days 120 --trends

# Export PMC data
trainrs pmc --days 90 --export pmc_data.csv
```

### PMC Configuration

```bash
# Custom CTL time constant
trainrs pmc --ctl-days 45 --days 90

# Custom ATL time constant
trainrs pmc --atl-days 5 --days 90

# Both custom constants
trainrs pmc --ctl-days 45 --atl-days 5 --days 90
```

### PMC Visualization

```bash
# Terminal chart
trainrs pmc --chart --days 60

# Export chart image (requires plotting features)
trainrs pmc --chart --output pmc_chart.png --days 90

# Interactive chart in browser
trainrs pmc --interactive --days 90
```

---

## `power` - Power Analysis

Advanced power analysis for cycling training.

### Power Curve Analysis

```bash
# Generate power curve
trainrs power --curve --days 90

# Export power curve data
trainrs power --curve --export curve.csv --days 180

# Compare power curves
trainrs power --curve --compare --periods 30,90,180
```

### Critical Power Analysis

```bash
# Calculate critical power
trainrs power --critical-power --days 90

# CP model with different durations
trainrs power --critical-power --durations 300,600,1200

# Export CP model data
trainrs power --critical-power --export cp_model.csv
```

### Power Distribution

```bash
# Power distribution analysis
trainrs power --distribution --days 30

# Zone-based power analysis
trainrs power --zones --days 30

# Quadrant analysis
trainrs power --quadrants --days 30
```

---

## `running` - Running Analysis

Specialized analysis for running training data.

### Pace Analysis

```bash
# Pace distribution analysis
trainrs running --pace-distribution --days 30

# Critical speed analysis
trainrs running --critical-speed --days 90

# Pace zone analysis
trainrs running --pace-zones --days 30
```

### Elevation Analysis

```bash
# Elevation gain analysis
trainrs running --elevation --days 30

# Grade-adjusted pace (GAP)
trainrs running --gap --days 30

# Hill repeat analysis
trainrs running --hills --days 30
```

### Performance Predictions

```bash
# Race time predictions
trainrs running --predict --distance 10k

# VDOT calculation and analysis
trainrs running --vdot --days 90

# Training pace recommendations
trainrs running --training-paces
```

---

## `multi-sport` - Multi-Sport Analysis

Analysis for athletes training multiple sports.

### Combined Load Analysis

```bash
# Combined training load
trainrs multi-sport --combined-load --days 30

# Sport-specific load breakdown
trainrs multi-sport --sport-breakdown --days 30

# Load distribution by sport
trainrs multi-sport --distribution --days 30
```

### Triathlon-Specific Analysis

```bash
# Triathlon training analysis
trainrs multi-sport --triathlon --days 90

# Brick workout analysis
trainrs multi-sport --brick-analysis --days 30

# Transition analysis
trainrs multi-sport --transitions --days 30
```

---

## `training-plan` - Training Plan Generation

Generate and analyze training plans.

### Plan Generation

```bash
# Generate base training plan
trainrs training-plan --generate --weeks 12 --goal endurance

# Generate race-specific plan
trainrs training-plan --generate --race-date 2024-09-15 --distance marathon

# Generate custom plan
trainrs training-plan --generate --template custom.json
```

### Plan Analysis

```bash
# Analyze current plan adherence
trainrs training-plan --analyze --plan-file plan.json

# Plan vs actual comparison
trainrs training-plan --compare --days 30

# Progress tracking
trainrs training-plan --progress --plan-file plan.json
```

---

## `config` - Application Configuration

Manage application settings and preferences.

### Basic Configuration

```bash
# View all settings
trainrs config --list

# Set configuration value
trainrs config --set athlete.default_ftp=250

# Get specific setting
trainrs config --get data.directory

# Reset to defaults
trainrs config --reset
```

### Athlete Configuration

```bash
# Set default athlete
trainrs config --set athlete.default=john_doe

# Set global FTP
trainrs config --set athlete.ftp=250

# Set units preference
trainrs config --set display.units=metric
```

### Data Configuration

```bash
# Set data directory
trainrs config --set data.directory=/path/to/data

# Set backup directory
trainrs config --set data.backup_dir=/path/to/backups

# Configure auto-backup
trainrs config --set data.auto_backup=true
```

---

## `athlete` - Athlete Profile Management

Manage athlete profiles and settings.

### Profile Management

```bash
# Create new athlete profile
trainrs athlete --create --name "John Doe" --id john_doe

# Update athlete profile
trainrs athlete --update john_doe --ftp 260 --weight 70

# List all athletes
trainrs athlete --list

# Show athlete details
trainrs athlete --show john_doe
```

### Threshold Management

```bash
# Set power thresholds
trainrs athlete --athlete john_doe --set-ftp 250

# Set heart rate thresholds
trainrs athlete --athlete john_doe --set-lthr 165 --set-max-hr 190

# Set running thresholds
trainrs athlete --athlete john_doe --set-threshold-pace 4:00

# Threshold history
trainrs athlete --athlete john_doe --threshold-history
```

---

## Advanced Examples

### Comprehensive Analysis Workflow

```bash
# 1. Import recent data
trainrs import --file recent_workouts.csv

# 2. Calculate all metrics
trainrs calculate --days 30 --all

# 3. Analyze trends and patterns
trainrs analyze --trend both --days 90
trainrs analyze --distribution zones --days 30

# 4. Generate comprehensive report
trainrs export --report detailed --days 30 --output monthly_report.html

# 5. Display current status
trainrs display --status
trainrs display --pmc --days 60
```

### Race Preparation Analysis

```bash
# 1. Set race date and goal
trainrs training-plan --race-date 2024-09-15 --goal "sub-3-marathon"

# 2. Analyze current fitness
trainrs pmc --days 120 --trends
trainrs running --predict --distance marathon

# 3. Monitor training load
trainrs summary --type load --days 14
trainrs analyze --trend fitness --days 60

# 4. Generate race preparation report
trainrs export --report race-prep --output race_analysis.html
```

### Multi-Athlete Coaching Setup

```bash
# 1. Create athlete profiles
trainrs athlete --create --name "Athlete 1" --id athlete_1
trainrs athlete --create --name "Athlete 2" --id athlete_2

# 2. Import data for each athlete
trainrs import --file athlete1_data.csv --athlete athlete_1
trainrs import --file athlete2_data.csv --athlete athlete_2

# 3. Generate comparison reports
trainrs export --athlete athlete_1 --output athlete1_report.html
trainrs export --athlete athlete_2 --output athlete2_report.html

# 4. Team summary
trainrs summary --all-athletes --days 30
```

---

## Error Handling & Troubleshooting

### Common Error Patterns

```bash
# File not found
trainrs import --file missing.csv
# Error: File 'missing.csv' not found

# Invalid date format
trainrs calculate --from "invalid-date"
# Error: Invalid date format. Use YYYY-MM-DD

# Missing required parameters
trainrs zones --set-ftp
# Error: FTP value required

# Insufficient data
trainrs analyze --days 365
# Warning: Insufficient data for 365-day analysis
```

### Debugging Options

```bash
# Verbose output for debugging
trainrs import --file data.csv --verbose

# Very verbose (debug level)
trainrs calculate --days 30 -vv

# Dry run to test without changes
trainrs import --file data.csv --dry-run

# Validate configuration
trainrs config --validate
```

---

*For more examples and advanced usage, see the [Training Load Guide](training-load.md) and [Troubleshooting](troubleshooting.md).*