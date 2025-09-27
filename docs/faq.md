# Frequently Asked Questions (FAQ)

Common questions and answers about TrainRS training analysis software.

## General Questions

### What is TrainRS?

TrainRS is a command-line training analysis tool that calculates sports science metrics like Training Stress Score (TSS), Intensity Factor (IF), and Performance Management Chart (PMC) data. It's designed for athletes, coaches, and sports scientists who need accurate, evidence-based training load analysis.

### Why use TrainRS instead of other training platforms?

**Advantages of TrainRS:**
- **Precision**: Uses Rust's decimal arithmetic for exact calculations
- **Open Source**: Full transparency in calculation methods
- **Local Data**: Your training data stays on your computer
- **Customizable**: Configure zones, algorithms, and outputs
- **Command-Line**: Scriptable and automatable
- **Sports Science**: Based on peer-reviewed research

**Best for:**
- Sports scientists requiring precise calculations
- Coaches managing multiple athletes
- Athletes wanting full control over their data
- Developers needing training analysis APIs

### Is TrainRS suitable for beginners?

TrainRS is designed for users who want deeper insights into their training data. While beginners can use it, it requires some understanding of training science concepts like TSS, CTL, and ATL.

**For beginners, we recommend:**
1. Start with basic concepts (see [Training Load Guide](training-load.md))
2. Use simple commands first (`import`, `display`, `summary`)
3. Gradually explore advanced features
4. Read the [Sports Science Background](sports-science.md)

## Installation & Setup

### What are the system requirements?

**Minimum Requirements:**
- Rust 1.70 or later
- 1 GB RAM
- 100 MB disk space
- Command-line terminal access

**Supported Platforms:**
- Linux (Ubuntu 20.04+, CentOS 7+, etc.)
- macOS (10.14+)
- Windows (10+, with WSL recommended)

### How do I install TrainRS?

**From Source (Recommended):**
```bash
git clone https://github.com/jpequegn/trainrs.git
cd trainrs
cargo build --release
```

**Using Cargo:**
```bash
cargo install trainrs
```

See the full [Installation Guide](installation.md) for detailed instructions.

### Where does TrainRS store my data?

**Default Data Locations:**
- **Linux**: `~/.local/share/trainrs/`
- **macOS**: `~/Library/Application Support/trainrs/`
- **Windows**: `%APPDATA%\trainrs\`

**Configuration Files:**
- **Linux**: `~/.config/trainrs/`
- **macOS**: `~/Library/Preferences/trainrs/`
- **Windows**: `%APPDATA%\trainrs\config\`

You can change these locations using the `--data-dir` option or configuration settings.

## Data Import & Export

### What file formats does TrainRS support?

**Import Formats:**
- **CSV**: Comma-separated values with customizable columns
- **JSON**: Structured workout data
- **GPX**: GPS track files with time/power/heart rate
- **FIT**: Garmin/ANT+ files (planned feature)

**Export Formats:**
- **CSV**: For spreadsheet analysis
- **JSON**: For programmatic use
- **HTML**: For web-based reports
- **Plain Text**: For terminal display

### How do I import data from Strava/TrainingPeaks/Garmin?

**Current Options:**
1. **Export from platform**: Most platforms allow CSV/GPX export
2. **Use API connectors**: Third-party tools to download data
3. **Manual data entry**: For key workouts

**Example workflow:**
```bash
# Export from Strava as CSV
# Download activities.csv
trainrs import --file activities.csv --format csv

# Or export as GPX files
trainrs import --directory gpx_files/ --format gpx
```

**Planned Features:**
- Direct API integration with major platforms
- Automatic synchronization
- Real-time data streaming

### Can I export data to other platforms?

Yes! TrainRS can export data in formats compatible with major training platforms:

**TrainingPeaks Format:**
```bash
trainrs export --format trainingpeaks --output tp_data.csv
```

**Strava Bulk Upload:**
```bash
trainrs export --format strava --output strava_data.json
```

**WKO5 Compatible:**
```bash
trainrs export --format wko5 --output wko_data.csv
```

## Training Metrics

### How accurate are the TSS calculations?

TrainRS implements the original TSS algorithm as published by Dr. Andy Coggan with several accuracy improvements:

**Accuracy Features:**
- **Decimal arithmetic**: Prevents floating-point errors
- **Validated algorithms**: Match published formulas exactly
- **Quality checks**: Data validation and outlier detection
- **Sport-specific**: Adaptations for cycling, running, swimming

**Validation Methods:**
- Compared against reference implementations
- Tested with known workout data
- Validated by sports scientists
- Extensive unit testing

### Why are my TSS values different from other platforms?

Common reasons for TSS differences:

1. **Threshold Settings**: Different FTP/LTHR values
2. **Data Quality**: Power spikes, dropouts, or smoothing
3. **Algorithm Variants**: Some platforms use modified formulas
4. **Rounding Differences**: TrainRS uses higher precision
5. **Sport Adaptations**: Different scaling factors

**To check:**
```bash
# Verify your thresholds
trainrs zones --show

# Check workout data quality
trainrs display --workouts --detailed

# Compare raw vs processed data
trainrs export --include-raw --format json
```

### What's the difference between average power and normalized power?

**Average Power**: Simple arithmetic mean of all power values
**Normalized Power (NP)**: Physiologically-adjusted power accounting for:
- Variability in intensity
- Non-linear stress response
- 30-second physiological lag time

**When they differ:**
- **NP > Average**: Variable intensity (intervals, group rides)
- **NP ≈ Average**: Steady-state efforts (time trials)
- **High difference**: Poor pacing or very variable efforts

### How do I interpret PMC values?

**PMC Components:**
- **CTL (Fitness)**: 42-day exponentially weighted average
- **ATL (Fatigue)**: 7-day exponentially weighted average
- **TSB (Form)**: CTL - ATL, indicates readiness

**Typical Values:**
```
Recreational athlete: CTL 40-60
Competitive amateur: CTL 60-80
Elite athlete: CTL 80-120+
```

**TSB Interpretation:**
- **+20**: Very fresh, peak performance possible
- **+10**: Fresh, ready for hard training
- **0**: Balanced, normal training
- **-10**: Slightly fatigued, reduce intensity
- **-20**: Fatigued, easy training only
- **-30**: Very fatigued, rest needed
```

See the [Training Load Guide](training-load.md) for detailed interpretation.

## Multi-Sport Training

### How does TrainRS handle different sports?

TrainRS applies sport-specific scaling factors and calculations:

**Sport Scaling Factors:**
- **Cycling**: 1.0 (baseline)
- **Running**: 1.3-1.5 (higher impact stress)
- **Swimming**: 0.8-1.0 (lower overall stress)

**Sport-Specific Metrics:**
- **Cycling**: Power-based TSS, IF, NP
- **Running**: Pace-based TSS, GAP, elevation adjustments
- **Swimming**: Time-based TSS, stroke rate analysis

### Can I combine training loads from different sports?

Yes! TrainRS provides several approaches:

**Combined Load Analysis:**
```bash
trainrs multi-sport --combined-load --days 30
```

**Sport-Specific Breakdown:**
```bash
trainrs multi-sport --sport-breakdown --days 30
```

**Weighted Combination:**
```bash
trainrs multi-sport --weights cycling:1.0,running:1.3,swimming:0.9
```

### How do I set up zones for multiple sports?

Configure zones for each sport separately:

```bash
# Cycling power zones
trainrs zones --sport cycling --set-ftp 250

# Running pace zones
trainrs zones --sport running --set-threshold-pace 4:00

# Swimming zones (time-based)
trainrs zones --sport swimming --set-css-pace 1:20
```

## Configuration & Customization

### How do I manage multiple athlete profiles?

**Create Athletes:**
```bash
trainrs athlete --create --name "John Doe" --id john_doe
trainrs athlete --create --name "Jane Smith" --id jane_smith
```

**Switch Between Athletes:**
```bash
# Use specific athlete for command
trainrs calculate --athlete john_doe --days 30

# Set default athlete
trainrs config --set athlete.default=john_doe

# List all athletes
trainrs athlete --list
```

### Can I customize the calculation algorithms?

TrainRS allows several customizations:

**PMC Time Constants:**
```bash
# Custom CTL period (default: 42 days)
trainrs pmc --ctl-days 45

# Custom ATL period (default: 7 days)
trainrs pmc --atl-days 5
```

**Zone Calculation Methods:**
```bash
# Heart rate zones from LTHR vs Max HR
trainrs zones --method lthr
trainrs zones --method max-hr
```

**Smoothing and Filtering:**
```bash
# Power data smoothing
trainrs calculate --smooth-power --window 30

# Outlier filtering
trainrs import --filter-outliers --max-power 1500
```

### How do I automate TrainRS with scripts?

TrainRS is designed for automation:

**Bash Script Example:**
```bash
#!/bin/bash
# Daily training analysis script

# Import today's data
trainrs import --file today_workout.csv

# Calculate metrics
trainrs calculate --days 1

# Generate summary
trainrs summary --type daily --output daily_summary.txt

# Check if rest day needed
TSB=$(trainrs display --format json | jq '.tsb')
if (( $(echo "$TSB < -20" | bc -l) )); then
    echo "Rest day recommended (TSB: $TSB)"
fi
```

**Python Integration:**
```python
import subprocess
import json

# Run TrainRS and parse output
result = subprocess.run(['trainrs', 'display', '--format', 'json'],
                       capture_output=True, text=True)
data = json.loads(result.stdout)

# Analyze data
ctl = data['ctl']
atl = data['atl']
tsb = data['tsb']

print(f"Fitness: {ctl}, Fatigue: {atl}, Form: {tsb}")
```

## Performance & Optimization

### TrainRS is running slowly. How can I improve performance?

**Common Optimizations:**

1. **Use streaming for large datasets:**
   ```bash
   trainrs import --file large_data.csv --streaming
   ```

2. **Reduce batch sizes:**
   ```bash
   trainrs calculate --batch-size 100 --days 365
   ```

3. **Enable database optimization:**
   ```bash
   trainrs maintenance --optimize-database
   ```

4. **Use specific date ranges:**
   ```bash
   trainrs analyze --from 2024-08-01 --to 2024-08-31
   ```

### How much disk space does TrainRS use?

**Typical Usage:**
- **Database**: 1-10 MB per year of training data
- **Logs**: 1-5 MB (rotated automatically)
- **Cache**: 10-50 MB (cleared automatically)
- **Exports**: Varies by format and data range

**For heavy users (10+ hours/week):**
- **5 years of data**: ~50 MB
- **With time series**: ~500 MB
- **With all exports**: ~1 GB

**Cleanup Commands:**
```bash
# Clear old logs
trainrs maintenance --cleanup-logs

# Clear cache
trainrs maintenance --clear-cache

# Optimize database
trainrs maintenance --optimize-database
```

## Troubleshooting

### My power data looks wrong. What should I check?

**Common Power Data Issues:**

1. **Calibration**: Zero offset before each ride
2. **Spikes**: Check for electromagnetic interference
3. **Dropouts**: Verify battery and connectivity
4. **Scaling**: Some devices report kW instead of W

**Diagnostic Commands:**
```bash
# Check for outliers
trainrs display --workouts --detailed | grep max_power

# View power distribution
trainrs power --distribution --days 30

# Filter obvious errors
trainrs import --file data.csv --filter-spikes --max-power 1500
```

### TrainRS says my FTP is too low/high. How do I set it correctly?

**FTP Testing Methods:**

1. **20-minute test**: FTP = 95% of 20-min average power
2. **60-minute test**: FTP = average power for 1 hour
3. **Ramp test**: Use platform-specific protocols
4. **Analysis-based**: TrainRS can estimate from training data

**Setting FTP:**
```bash
# Manual setting
trainrs zones --sport cycling --set-ftp 250

# Auto-detection from data
trainrs zones --auto-detect --days 90

# Gradual adjustment
trainrs zones --adjust-ftp +5  # Increase by 5W
```

### How do I know if my training zones are correct?

**Zone Validation Methods:**

1. **Physiological testing**: Lab testing for accurate thresholds
2. **Field testing**: Time trials at different durations
3. **Training analysis**: Review zone distribution over time
4. **Perceived exertion**: Match zones to RPE scales

**Check Zone Distribution:**
```bash
# Analyze time in zones
trainrs analyze --distribution zones --days 30

# Compare with recommended distributions
trainrs analyze --distribution --target polarized
```

## Privacy & Security

### Is my training data secure?

**TrainRS Security Features:**
- **Local storage**: Data never leaves your computer
- **No cloud sync**: No automatic uploads
- **Open source**: Code is auditable
- **Encryption**: Optional database encryption

**Best Practices:**
- Regular backups to secure locations
- Use file system encryption
- Restrict file permissions
- Keep TrainRS updated

### Can I share my TrainRS data safely?

**Safe Sharing Methods:**

1. **Export aggregated data only:**
   ```bash
   trainrs export --data summary --no-personal-info
   ```

2. **Anonymize exported data:**
   ```bash
   trainrs export --anonymize --output anonymous_data.csv
   ```

3. **Share specific date ranges:**
   ```bash
   trainrs export --from 2024-08-01 --to 2024-08-31
   ```

**Avoid sharing:**
- Raw GPS tracks (location privacy)
- Personal information (athlete profiles)
- Complete training database

## Development & Contributing

### How can I contribute to TrainRS development?

**Ways to Contribute:**

1. **Report bugs**: Use GitHub issues
2. **Suggest features**: Feature requests welcome
3. **Improve documentation**: Help other users
4. **Submit code**: Pull requests for fixes/features
5. **Test beta versions**: Early feedback is valuable

See the [Contributing Guide](contributing.md) for detailed information.

### Can I extend TrainRS with plugins?

**Current Extension Options:**
- Custom import/export formats
- Additional calculation methods
- Third-party integrations
- Custom report templates

**Planned Features:**
- Plugin architecture
- Custom metric calculations
- External API integrations
- Scripting interface

### Is there a GUI version planned?

**Current Status:**
- TrainRS is primarily command-line focused
- Web-based dashboard is planned
- Desktop GUI is under consideration

**Alternatives:**
- Use JSON export with visualization tools
- Integration with existing platforms
- Third-party dashboard development

## Support & Community

### Where can I get help?

**Support Channels:**
1. **Documentation**: Start with this guide and others in `/docs/`
2. **GitHub Issues**: Bug reports and feature requests
3. **GitHub Discussions**: Questions and community help
4. **Sports Science Forums**: Training analysis discussions

### How can I stay updated on TrainRS development?

**Stay Connected:**
- **GitHub**: Watch the repository for updates
- **Releases**: Subscribe to release notifications
- **Discussions**: Join community conversations
- **Blog**: Development updates and tutorials (planned)

### Can I use TrainRS for commercial purposes?

TrainRS is open source under the MIT License, which allows:
- Commercial use
- Modification and distribution
- Private use
- Patent use protection

**Requirements:**
- Include the original license
- Include copyright notice

See the LICENSE file for complete terms.

---

## Still Have Questions?

If your question isn't answered here:

1. **Check the documentation**: [Complete guide index](README.md)
2. **Search GitHub issues**: Your question might already be answered
3. **Ask the community**: Use GitHub Discussions
4. **Report bugs**: Use GitHub Issues for problems
5. **Contact maintainers**: For sensitive or complex issues

---

*This FAQ is regularly updated. Suggest additions through [GitHub Issues](https://github.com/jpequegn/trainrs/issues) or [GitHub Discussions](https://github.com/jpequegn/trainrs/discussions).*