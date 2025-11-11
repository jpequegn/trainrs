# Benchmark Workflow Setup Guide

## Overview

This document provides comprehensive instructions for setting up and maintaining GitHub Actions benchmark workflows for Rust projects using Criterion. It documents lessons learned from implementing the TrainRS benchmark workflow.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Common Issues & Solutions](#common-issues--solutions)
3. [Workflow Architecture](#workflow-architecture)
4. [Best Practices](#best-practices)
5. [Future Enhancements](#future-enhancements)
6. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Minimal Working Benchmark Workflow

```yaml
name: Performance Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  benchmark:
    name: Run Performance Benchmarks
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write  # Required for PR comments

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true

      - name: Cache cargo dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run benchmarks
        run: cargo bench --bench performance_benchmarks -- --output-format bencher | tee benchmark_results.txt

      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: benchmark-results
          path: |
            target/criterion/
            benchmark_results.txt

      - name: Comment PR with benchmark results
        if: github.event_name == 'pull_request'
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            const results = fs.readFileSync('benchmark_results.txt', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: '## 📊 Performance Benchmark Results\n\n```\n' + results + '\n```'
            });
```

---

## Common Issues & Solutions

### Issue 1: Benchmark API Mismatches

**Problem:** Benchmark code doesn't compile due to API changes.

**Symptoms:**
```bash
error[E0425]: cannot find function `calculate_power_zones` in module `zones`
error[E0061]: this method takes 3 arguments but 2 arguments were supplied
error[E0609]: no field `avg_hr` on type `WorkoutSummary`
```

**Root Cause:** Benchmarks written against old API that has changed.

**Solution:**
1. Always compile benchmarks before committing
2. Update benchmark code to match current API
3. Use proper imports (e.g., `use rust_decimal::prelude::FromPrimitive`)
4. Check method signatures match current implementation
5. Verify field names match current structs

**Prevention:**
```bash
# Always test before committing
cargo bench --bench performance_benchmarks --no-run
cargo bench --bench performance_benchmarks -- --test
```

### Issue 2: GitHub Actions Version Deprecation

**Problem:** Deprecated GitHub Actions versions cause warnings.

**Symptoms:**
```
Node.js 16 actions are deprecated
actions/upload-artifact@v3 is deprecated
```

**Solution:**
Update to latest versions:
- `actions/cache@v3` → `actions/cache@v4`
- `actions/upload-artifact@v3` → `actions/upload-artifact@v4`
- `actions/github-script@v6` → `actions/github-script@v7`

**Prevention:**
- Review GitHub Actions deprecation notices regularly
- Check [GitHub Actions marketplace](https://github.com/marketplace?type=actions) for latest versions
- Update versions in bulk when multiple actions are deprecated

### Issue 3: github-action-benchmark gh-pages Dependency

**Problem:** The `github-action-benchmark` action requires gh-pages branch.

**Symptoms:**
```
fatal: couldn't find remote ref gh-pages
exit code 128
```

**Root Cause:** The action uses gh-pages as the default data store for historical benchmarks. It attempts git operations even with `auto-push: false` and without `github-token`.

**Why Previous Solutions Failed:**
1. ❌ `auto-push: false` - Action still fetches from gh-pages
2. ❌ Removing `github-token` - Action still tries git operations
3. ❌ `save-data-file: true` - Action still needs gh-pages for comparison

**Solution A: Remove the Action (Simplest)**
```yaml
# Just run benchmarks and upload results
- name: Run benchmarks
  run: cargo bench --bench performance_benchmarks -- --output-format bencher | tee benchmark_results.txt

- name: Upload benchmark results
  uses: actions/upload-artifact@v4
  with:
    name: benchmark-results
    path: benchmark_results.txt
```

**Solution B: Create gh-pages Branch (Full Features)**
```bash
# One-time setup
git checkout --orphan gh-pages
git rm -rf .
echo "# Benchmark Data" > README.md
git add README.md
git commit -m "Initialize gh-pages for benchmark data"
git push origin gh-pages
```

Then use the full benchmark action:
```yaml
- name: Store benchmark result
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: benchmark_results.txt
    github-token: ${{ secrets.GITHUB_TOKEN }}
    auto-push: true
    alert-threshold: '105%'
    comment-on-alert: true
```

**Trade-offs:**

| Approach | Pros | Cons |
|----------|------|------|
| **Solution A: No Action** | ✅ No gh-pages needed<br>✅ Simple setup<br>✅ No git errors | ❌ No automated regression alerts<br>❌ No trend charts<br>❌ Manual comparison |
| **Solution B: With gh-pages** | ✅ Automated regression detection<br>✅ Performance charts<br>✅ Historical tracking | ❌ Requires gh-pages setup<br>❌ More complex<br>❌ Git push operations |

### Issue 4: Benchmark Output File Path

**Problem:** github-action-benchmark can't find benchmark results.

**Symptoms:**
```
Invalid value for 'output-file-path' input
```

**Root Cause:** Using glob patterns like `target/criterion/*/new/estimates.json` which the action doesn't support.

**Solution:**
Point to the actual output file from your benchmark command:
```yaml
- name: Run benchmarks
  run: cargo bench -- --output-format bencher | tee benchmark_results.txt

- name: Store benchmark result
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: benchmark_results.txt  # Actual file, not glob
```

**Criterion Output Options:**
- `--output-format bencher` - Compatible with `tool: 'cargo'`
- Criterion JSON - Use `tool: 'criterion'` (but still has gh-pages dependency)

### Issue 5: PR Comment Permission Error

**Problem:** Workflow can't comment on PRs.

**Symptoms:**
```
RequestError [HttpError]: Resource not accessible by integration
```

**Root Cause:** Default `GITHUB_TOKEN` has restricted permissions following principle of least privilege.

**Solution:**
Add explicit permissions to the job:
```yaml
jobs:
  benchmark:
    permissions:
      contents: read          # Required to checkout code
      pull-requests: write    # Required to comment on PRs
```

**Security Note:** These are minimal permissions. The workflow can only read code and comment on PRs, nothing else.

---

## Workflow Architecture

### Recommended Architecture (Without gh-pages)

```
┌─────────────────────────────────────────────────┐
│          Trigger (Push/PR)                      │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│     Setup Environment                            │
│  • Checkout code                                 │
│  • Install Rust toolchain                        │
│  • Cache dependencies                            │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│     Run Benchmarks                               │
│  • cargo bench --output-format bencher           │
│  • Save to benchmark_results.txt                 │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│     Store Results                                │
│  • Upload as artifacts (90-day retention)        │
│  • Criterion reports in target/criterion/        │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│     PR Communication                             │
│  • Comment with benchmark results (PRs only)     │
│  • Manual regression review                      │
└─────────────────────────────────────────────────┘
```

### With gh-pages Architecture

```
┌─────────────────────────────────────────────────┐
│          Trigger (Push/PR)                      │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│     Setup Environment                            │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│     Run Benchmarks                               │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│     github-action-benchmark                      │
│  • Fetch history from gh-pages                   │
│  • Compare with baseline                         │
│  • Detect regressions (>5% threshold)            │
│  • Generate performance charts                   │
│  • Push updated data to gh-pages                 │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│     Alerts & Communication                       │
│  • Automated regression alerts                   │
│  • PR comments with comparison                   │
│  • Performance trend charts on gh-pages          │
└─────────────────────────────────────────────────┘
```

---

## Best Practices

### 1. Benchmark Organization

**File Structure:**
```
benches/
├── performance_benchmarks.rs    # Main benchmark suite
├── fixtures/                    # Test data
│   ├── sample_workout.fit
│   └── README.md
└── helpers.rs                   # Shared benchmark utilities
```

**Benchmark Groups:**
```rust
criterion_group!(
    benches,
    bench_tss_calculation,
    bench_pmc_calculation,
    bench_power_analysis,
    // ... group related benchmarks
);
```

### 2. Cargo.toml Configuration

```toml
[[bench]]
name = "performance_benchmarks"
harness = false

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

### 3. GitHub Actions Best Practices

**Caching Strategy:**
```yaml
- name: Cache dependencies
  uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-bench-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-bench-
      ${{ runner.os }}-cargo-
```

**Artifact Retention:**
```yaml
- name: Upload benchmark results
  uses: actions/upload-artifact@v4
  if: always()  # Upload even if benchmarks fail
  with:
    name: benchmark-results-${{ github.sha }}
    path: |
      target/criterion/
      benchmark_results.txt
    retention-days: 90  # Adjust based on needs
```

**Conditional Execution:**
```yaml
- name: Comment on PR
  if: github.event_name == 'pull_request'  # Only for PRs
  # ...

- name: Store baseline
  if: github.event_name != 'pull_request'  # Only for main branch
  # ...
```

### 4. Security Best Practices

**Principle of Least Privilege:**
```yaml
permissions:
  contents: read          # Only what's needed
  pull-requests: write    # Only what's needed
  # Don't add: actions: write, packages: write, etc.
```

**Token Safety:**
```yaml
# ❌ Don't expose tokens in logs
- run: echo ${{ secrets.GITHUB_TOKEN }}

# ✅ Use built-in mechanisms
- uses: actions/github-script@v7
  # Token automatically available
```

### 5. Performance Optimization

**Parallel Caching:**
```yaml
# Separate cache keys for different artifacts
- uses: actions/cache@v4
  with:
    path: ~/.cargo/registry
    key: registry-${{ hashFiles('**/Cargo.lock') }}

- uses: actions/cache@v4
  with:
    path: target
    key: target-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}
```

**Benchmark Subset Testing:**
```bash
# Run only specific benchmarks during development
cargo bench --bench performance_benchmarks -- tss_calculation

# Run all benchmarks in CI
cargo bench --bench performance_benchmarks
```

---

## Future Enhancements

### 1. Custom Regression Detection Script

If you don't want gh-pages but need automated regression detection:

```yaml
- name: Download previous benchmark
  uses: dawidd6/action-download-artifact@v2
  with:
    workflow: benchmarks.yml
    name: benchmark-results
    path: baseline/

- name: Compare benchmarks
  run: |
    python scripts/compare_benchmarks.py \
      --baseline baseline/benchmark_results.txt \
      --current benchmark_results.txt \
      --threshold 1.05 \
      --fail-on-regression
```

**Sample comparison script:**
```python
# scripts/compare_benchmarks.py
import sys
import re

def parse_bencher_output(filepath):
    """Parse cargo bench bencher format output."""
    benchmarks = {}
    with open(filepath) as f:
        for line in f:
            # Format: test bench_name ... bench: 1,234 ns/iter (+/- 56)
            match = re.match(r'test (\w+)\s+.*bench:\s+([\d,]+) ns/iter', line)
            if match:
                name = match.group(1)
                time_ns = int(match.group(2).replace(',', ''))
                benchmarks[name] = time_ns
    return benchmarks

def compare_benchmarks(baseline, current, threshold):
    """Compare benchmark results and detect regressions."""
    regressions = []

    for name, current_time in current.items():
        if name not in baseline:
            continue

        baseline_time = baseline[name]
        ratio = current_time / baseline_time

        if ratio > threshold:
            regression_pct = (ratio - 1) * 100
            regressions.append({
                'name': name,
                'baseline': baseline_time,
                'current': current_time,
                'regression': regression_pct
            })

    return regressions

# ... implement full comparison logic
```

### 2. Performance Trend Dashboard

Create a simple dashboard using GitHub Pages without gh-pages branch:

```yaml
- name: Generate performance report
  run: |
    python scripts/generate_report.py \
      --results benchmark_results.txt \
      --output docs/benchmarks.html

- name: Deploy to GitHub Pages
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./docs
    destination_dir: benchmarks/${{ github.sha }}
```

### 3. Integration with Monitoring Services

**Datadog Integration:**
```yaml
- name: Send metrics to Datadog
  run: |
    python scripts/send_to_datadog.py \
      --file benchmark_results.txt \
      --api-key ${{ secrets.DATADOG_API_KEY }}
```

**Prometheus Integration:**
```yaml
- name: Push to Prometheus Pushgateway
  run: |
    python scripts/push_to_prometheus.py \
      --file benchmark_results.txt \
      --gateway ${{ secrets.PROMETHEUS_GATEWAY }}
```

### 4. Benchmark Matrix Testing

Test across multiple configurations:

```yaml
strategy:
  matrix:
    rust: [stable, beta, nightly]
    os: [ubuntu-latest, macos-latest, windows-latest]

steps:
  - name: Run benchmarks
    run: cargo bench --bench performance_benchmarks

  - name: Upload results
    uses: actions/upload-artifact@v4
    with:
      name: benchmark-${{ matrix.os }}-${{ matrix.rust }}
      path: benchmark_results.txt
```

---

## Troubleshooting

### Benchmark Compilation Errors

**Check:**
1. Benchmark code is up to date with current API
2. All dependencies are available
3. Features flags are correct

**Debug:**
```bash
cargo bench --bench performance_benchmarks --no-run --verbose
```

### Workflow Permission Errors

**Common errors:**
- `Resource not accessible by integration` → Missing `pull-requests: write`
- `Permission denied` → Check repository settings → Actions → General → Workflow permissions

**Fix:**
```yaml
permissions:
  contents: read
  pull-requests: write
  # Add others as needed
```

### Cache Not Working

**Common issues:**
- Cache key doesn't match
- Cache size exceeds 10GB limit
- Wrong paths specified

**Debug:**
```yaml
- name: Debug cache
  run: |
    echo "Cache key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}"
    ls -la ~/.cargo/registry || echo "No registry cache"
    ls -la target || echo "No target cache"
```

### Benchmarks Take Too Long

**Optimization strategies:**
1. Reduce sample size in development:
   ```rust
   criterion_group! {
       name = benches;
       config = Criterion::default().sample_size(10);  // Faster iteration
       targets = bench_tss_calculation
   }
   ```

2. Use faster benchmark runner in CI:
   ```yaml
   - name: Run benchmarks
     run: cargo bench --bench performance_benchmarks -- --quick
   ```

3. Cache more aggressively:
   ```yaml
   - uses: actions/cache@v4
     with:
       path: target/criterion
       key: criterion-${{ hashFiles('benches/**/*.rs') }}
   ```

### Artifacts Not Uploading

**Check:**
1. File paths exist
2. `if: always()` condition is set
3. Artifact name is unique

**Debug:**
```yaml
- name: List files before upload
  if: always()
  run: |
    ls -R target/criterion/
    ls -la benchmark_results.txt
```

---

## Summary Checklist

When setting up benchmark workflows:

- [ ] Benchmark code compiles and runs locally
- [ ] GitHub Actions versions are up to date (v4+)
- [ ] Workflow has correct permissions
- [ ] Caching is configured properly
- [ ] Artifacts upload with reasonable retention
- [ ] PR comments work (if needed)
- [ ] Decide: gh-pages vs simple artifact storage
- [ ] Document expected benchmark performance
- [ ] Set up alerting/monitoring (optional)
- [ ] Test workflow on PR before merging

---

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [github-action-benchmark](https://github.com/benchmark-action/github-action-benchmark)
- [Cargo Bench Documentation](https://doc.rust-lang.org/cargo/commands/cargo-bench.html)

---

**Last Updated:** 2025-10-10
**Maintainer:** TrainRS Team
**Related Issues:** #74, #122, #123
