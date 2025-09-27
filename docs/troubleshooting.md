# Troubleshooting Guide

Common issues and solutions for TrainRS training analysis software.

## Installation Issues

### Rust Installation Problems

**Issue**: Cannot install Rust or cargo command not found
```bash
error: command 'cargo' not found
```

**Solutions**:
1. Install Rust using rustup:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. Verify installation:
   ```bash
   cargo --version
   rustc --version
   ```

3. Update PATH if needed:
   ```bash
   echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
   source ~/.bashrc
   ```

### Build Compilation Errors

**Issue**: Compilation fails with dependency errors
```bash
error[E0432]: unresolved import
```

**Solutions**:
1. Update Rust to latest stable:
   ```bash
   rustup update stable
   ```

2. Clean and rebuild:
   ```bash
   cargo clean
   cargo build --release
   ```

3. Check system dependencies:
   ```bash
   # macOS
   xcode-select --install

   # Ubuntu/Debian
   sudo apt-get install build-essential pkg-config libssl-dev

   # CentOS/RHEL
   sudo yum groupinstall "Development Tools"
   sudo yum install openssl-devel
   ```

### Permission Issues

**Issue**: Permission denied when installing or running
```bash
permission denied: trainrs
```

**Solutions**:
1. Make binary executable:
   ```bash
   chmod +x target/release/trainrs
   ```

2. Install to user directory:
   ```bash
   cargo install --path . --root ~/.local
   export PATH="$HOME/.local/bin:$PATH"
   ```

3. Run with appropriate permissions:
   ```bash
   sudo ./target/release/trainrs config --list
   ```

---

## Data Import Issues

### File Format Problems

**Issue**: CSV import fails with parsing errors
```bash
Error: Failed to parse CSV row at line 15
```

**Solutions**:
1. Check CSV format requirements:
   ```csv
   date,duration_seconds,avg_power,max_power,normalized_power,tss,if
   2024-08-15,3600,180,320,195,85,0.78
   ```

2. Validate data types:
   - Date: YYYY-MM-DD format
   - Numbers: No currency symbols or units
   - Duration: Integer seconds

3. Use validation mode:
   ```bash
   trainrs import --file data.csv --validate-only
   ```

4. Check file encoding:
   ```bash
   file data.csv
   # Should show: ASCII text or UTF-8 Unicode text
   ```

### Missing Data Columns

**Issue**: Required columns not found in import file
```bash
Error: Required column 'date' not found
```

**Solutions**:
1. Check required columns:
   - `date` (YYYY-MM-DD)
   - `duration_seconds` (integer)
   - At least one of: `avg_power`, `avg_heart_rate`, `avg_pace`

2. Map column names:
   ```bash
   trainrs import --file data.csv --map-columns "workout_date:date,time:duration_seconds"
   ```

3. Use column order specification:
   ```bash
   trainrs import --file data.csv --columns date,duration,power,hr
   ```

### Large File Import

**Issue**: Import fails with large files or runs out of memory
```bash
Error: Out of memory during import
```

**Solutions**:
1. Use streaming import:
   ```bash
   trainrs import --file large_data.csv --streaming
   ```

2. Split large files:
   ```bash
   split -l 1000 large_data.csv chunk_
   for file in chunk_*; do
     trainrs import --file "$file"
   done
   ```

3. Increase system memory limits:
   ```bash
   ulimit -v 4194304  # 4GB virtual memory
   ```

---

## Configuration Issues

### Config File Problems

**Issue**: Configuration file not found or corrupted
```bash
Error: Could not read configuration file
```

**Solutions**:
1. Check config file location:
   ```bash
   trainrs config --list-paths
   ```

2. Reset to defaults:
   ```bash
   trainrs config --reset
   ```

3. Create new config manually:
   ```bash
   mkdir -p ~/.config/trainrs
   trainrs config --init
   ```

4. Validate config syntax:
   ```bash
   trainrs config --validate
   ```

### Athlete Profile Issues

**Issue**: Athlete profile not found or invalid
```bash
Error: Athlete 'john_doe' not found
```

**Solutions**:
1. List available athletes:
   ```bash
   trainrs athlete --list
   ```

2. Create missing athlete:
   ```bash
   trainrs athlete --create --name "John Doe" --id john_doe
   ```

3. Set default athlete:
   ```bash
   trainrs config --set athlete.default=john_doe
   ```

4. Use explicit athlete specification:
   ```bash
   trainrs calculate --athlete john_doe --days 30
   ```

### Threshold Value Problems

**Issue**: Invalid or missing threshold values
```bash
Warning: FTP not set, using default value
```

**Solutions**:
1. Set functional threshold power:
   ```bash
   trainrs zones --sport cycling --set-ftp 250
   ```

2. Set lactate threshold heart rate:
   ```bash
   trainrs zones --sport running --set-lthr 165
   ```

3. Auto-detect from data:
   ```bash
   trainrs zones --auto-detect --days 90
   ```

4. Verify threshold settings:
   ```bash
   trainrs zones --show
   ```

---

## Calculation Issues

### Missing Calculated Values

**Issue**: TSS or other metrics not calculated
```bash
Warning: Cannot calculate TSS - missing power data
```

**Solutions**:
1. Check required data availability:
   ```bash
   trainrs display --workouts --format json | grep -E "(avg_power|avg_heart_rate|avg_pace)"
   ```

2. Set appropriate thresholds:
   ```bash
   # For power-based TSS
   trainrs zones --set-ftp 250

   # For HR-based TSS
   trainrs zones --set-lthr 165

   # For pace-based TSS
   trainrs zones --set-threshold-pace 4:00
   ```

3. Force recalculation:
   ```bash
   trainrs calculate --recalculate --from 2024-08-01
   ```

4. Use estimation for missing values:
   ```bash
   trainrs calculate --estimate-missing --days 30
   ```

### Unrealistic Metric Values

**Issue**: Calculated metrics seem unrealistic
```bash
TSS: 2500 (seems too high)
IF: 2.5 (impossible value)
```

**Solutions**:
1. Verify threshold settings:
   ```bash
   trainrs zones --show
   # Check if FTP is too low
   ```

2. Check data quality:
   ```bash
   trainrs display --workouts --detailed
   # Look for power spikes or data errors
   ```

3. Validate equipment calibration:
   - Power meter zero offset
   - Heart rate monitor battery
   - GPS accuracy settings

4. Filter outliers:
   ```bash
   trainrs calculate --filter-outliers --days 30
   ```

### PMC Calculation Issues

**Issue**: PMC values don't match expectations
```bash
CTL: 45, ATL: 15, TSB: 30 (but feeling fatigued)
```

**Solutions**:
1. Check PMC parameters:
   ```bash
   trainrs pmc --show-config
   ```

2. Verify training history completeness:
   ```bash
   trainrs display --workouts --days 60 --format table
   ```

3. Adjust PMC constants if needed:
   ```bash
   trainrs pmc --ctl-days 45 --atl-days 5 --recalculate
   ```

4. Consider non-training stressors:
   - Work stress
   - Sleep quality
   - Nutrition changes
   - Life events

---

## Performance Issues

### Slow Import/Calculation

**Issue**: Operations take too long to complete
```bash
Import stuck at 50% for 10 minutes
```

**Solutions**:
1. Use verbose mode to identify bottlenecks:
   ```bash
   trainrs import --file data.csv --verbose
   ```

2. Enable streaming for large datasets:
   ```bash
   trainrs import --file data.csv --streaming --chunk-size 1000
   ```

3. Optimize database:
   ```bash
   trainrs maintenance --optimize-database
   ```

4. Check available disk space:
   ```bash
   df -h ~/.local/share/trainrs/
   ```

### Memory Usage Issues

**Issue**: High memory consumption during processing
```bash
trainrs process killed (out of memory)
```

**Solutions**:
1. Reduce batch size:
   ```bash
   trainrs calculate --batch-size 100 --days 365
   ```

2. Use streaming calculations:
   ```bash
   trainrs analyze --streaming --days 365
   ```

3. Process in smaller chunks:
   ```bash
   trainrs calculate --from 2024-01-01 --to 2024-03-31
   trainrs calculate --from 2024-04-01 --to 2024-06-30
   ```

4. Increase virtual memory:
   ```bash
   # Temporary increase
   ulimit -v 8388608  # 8GB
   ```

---

## Data Quality Issues

### Power Data Problems

**Issue**: Unrealistic power values or spikes
```bash
Max Power: 2500W (unrealistic for most athletes)
```

**Solutions**:
1. Check power meter calibration:
   ```bash
   # Perform zero offset before rides
   # Check manufacturer's calibration procedure
   ```

2. Filter power spikes:
   ```bash
   trainrs import --file data.csv --filter-spikes --max-power 1500
   ```

3. Validate power data:
   ```bash
   trainrs display --workouts --detailed | grep -E "(max_power|avg_power)"
   ```

4. Use power smoothing:
   ```bash
   trainrs calculate --smooth-power --window 30 --days 30
   ```

### Heart Rate Data Issues

**Issue**: Erratic or missing heart rate data
```bash
Heart rate: 0, 250, 0, 95, 240 (erratic pattern)
```

**Solutions**:
1. Check heart rate monitor:
   - Battery level
   - Strap moisture
   - Skin contact
   - Interference sources

2. Filter HR data:
   ```bash
   trainrs import --file data.csv --filter-hr --min-hr 40 --max-hr 220
   ```

3. Use HR estimation:
   ```bash
   trainrs calculate --estimate-hr-from-power --days 30
   ```

4. Validate HR zones:
   ```bash
   trainrs zones --sport running --method lthr --set-lthr 165
   ```

### GPS/Pace Data Issues

**Issue**: Incorrect distance or pace calculations
```bash
Distance: 0.5 km for 1-hour ride (clearly wrong)
```

**Solutions**:
1. Check GPS accuracy settings:
   - Enable all satellite systems (GPS, GLONASS, Galileo)
   - Use 1-second recording
   - Avoid poor satellite conditions

2. Correct distance manually:
   ```bash
   trainrs workout --id workout_123 --set-distance 45.2
   ```

3. Use course-based distance:
   ```bash
   trainrs import --file data.gpx --use-course-distance
   ```

4. Filter GPS outliers:
   ```bash
   trainrs import --file data.gpx --filter-gps --max-speed 80
   ```

---

## Export/Report Issues

### Export Format Problems

**Issue**: Export fails or produces invalid files
```bash
Error: Cannot write to output file
```

**Solutions**:
1. Check file permissions:
   ```bash
   ls -la output.csv
   chmod 644 output.csv
   ```

2. Verify output directory exists:
   ```bash
   mkdir -p /path/to/output/directory
   trainrs export --output /path/to/output/report.csv
   ```

3. Use different format:
   ```bash
   trainrs export --format json --output data.json
   ```

4. Test with smaller dataset:
   ```bash
   trainrs export --days 7 --output test.csv
   ```

### Report Generation Issues

**Issue**: HTML/PDF reports not generating correctly
```bash
Error: Template rendering failed
```

**Solutions**:
1. Check template availability:
   ```bash
   trainrs export --list-templates
   ```

2. Use basic template:
   ```bash
   trainrs export --template basic --output report.html
   ```

3. Install additional dependencies:
   ```bash
   # For PDF generation
   sudo apt-get install wkhtmltopdf  # Ubuntu
   brew install wkhtmltopdf         # macOS
   ```

4. Generate step by step:
   ```bash
   trainrs export --data workouts --output workouts.json
   trainrs export --template custom.html --data workouts.json
   ```

---

## Command-Line Interface Issues

### Command Not Found

**Issue**: trainrs command not recognized
```bash
bash: trainrs: command not found
```

**Solutions**:
1. Use full path:
   ```bash
   ./target/release/trainrs --help
   ```

2. Add to PATH:
   ```bash
   export PATH="$PWD/target/release:$PATH"
   echo 'export PATH="$PWD/target/release:$PATH"' >> ~/.bashrc
   ```

3. Install globally:
   ```bash
   cargo install --path . --root ~/.local
   export PATH="$HOME/.local/bin:$PATH"
   ```

4. Create symlink:
   ```bash
   ln -s $PWD/target/release/trainrs ~/.local/bin/trainrs
   ```

### Argument Parsing Errors

**Issue**: Command line arguments not recognized
```bash
error: Found argument '--unknown-flag' which wasn't expected
```

**Solutions**:
1. Check command syntax:
   ```bash
   trainrs --help
   trainrs import --help
   ```

2. Use correct flag format:
   ```bash
   # Correct
   trainrs import --file data.csv --format csv

   # Incorrect
   trainrs import -file data.csv -format csv
   ```

3. Quote arguments with spaces:
   ```bash
   trainrs athlete --create --name "John Doe"
   ```

4. Use explicit value assignment:
   ```bash
   trainrs config --set athlete.name="John Doe"
   ```

---

## Database Issues

### Database Corruption

**Issue**: Database file corrupted or inaccessible
```bash
Error: Database corruption detected
```

**Solutions**:
1. Backup existing data:
   ```bash
   cp ~/.local/share/trainrs/database.db database.db.backup
   ```

2. Run database repair:
   ```bash
   trainrs maintenance --repair-database
   ```

3. Rebuild from exports:
   ```bash
   trainrs export --all-data --output backup.json
   trainrs maintenance --rebuild-database
   trainrs import --file backup.json
   ```

4. Start fresh if needed:
   ```bash
   trainrs maintenance --reset-database
   # Re-import all training data
   ```

### Database Lock Issues

**Issue**: Database locked by another process
```bash
Error: Database is locked
```

**Solutions**:
1. Check for running instances:
   ```bash
   ps aux | grep trainrs
   killall trainrs
   ```

2. Remove lock file:
   ```bash
   rm ~/.local/share/trainrs/database.db-wal
   rm ~/.local/share/trainrs/database.db-shm
   ```

3. Wait and retry:
   ```bash
   sleep 5
   trainrs import --file data.csv
   ```

---

## Getting Additional Help

### Debug Information

**Collect debug information:**
```bash
# System information
uname -a
cargo --version
rustc --version

# TrainRS version and config
trainrs --version
trainrs config --list

# Recent log entries
cat ~/.local/share/trainrs/logs/trainrs.log | tail -50
```

### Reporting Issues

**When reporting bugs, include:**

1. **System Information**:
   - Operating system and version
   - Rust/Cargo version
   - TrainRS version

2. **Error Details**:
   - Complete error message
   - Command that caused the error
   - Steps to reproduce

3. **Data Context**:
   - File format being imported
   - Data size and timeframe
   - Athlete configuration

4. **Logs**:
   - Relevant log entries
   - Debug output (use `--verbose`)

### Support Channels

- **GitHub Issues**: [GitHub Repository](https://github.com/jpequegn/trainrs/issues)
- **GitHub Discussions**: General questions and help
- **Documentation**: This guide and other docs
- **Community Forums**: Sports science and training communities

### Emergency Recovery

**If TrainRS is completely broken:**

1. **Backup data**:
   ```bash
   tar -czf trainrs_backup.tar.gz ~/.local/share/trainrs/
   ```

2. **Clean reinstall**:
   ```bash
   cargo clean
   cargo build --release
   ```

3. **Reset configuration**:
   ```bash
   rm -rf ~/.config/trainrs/
   trainrs config --init
   ```

4. **Restore from backup**:
   ```bash
   trainrs import --file backup_data.csv
   ```

---

*For additional support, see the [FAQ](faq.md) or contact the development team through [GitHub Issues](https://github.com/jpequegn/trainrs/issues).*