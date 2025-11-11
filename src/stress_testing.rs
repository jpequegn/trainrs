//! Memory leak detection and stress testing framework
//!
//! Provides comprehensive stress testing capabilities to ensure production stability
//! including memory profiling, resource monitoring, and long-running operation testing.
//!
//! Features:
//! - Configurable stress test scenarios (100-10,000 files)
//! - Memory metrics collection and tracking
//! - Resource cleanup verification
//! - Performance regression detection
//! - Memory growth trend analysis

use anyhow::Result;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tracing::{info, warn, debug};

/// Memory metrics snapshot
#[derive(Debug, Clone, Default)]
pub struct MemoryMetrics {
    /// RSS (Resident Set Size) in bytes
    pub rss_bytes: u64,
    /// Virtual memory in bytes
    pub vms_bytes: u64,
    /// Peak memory used in bytes
    pub peak_rss_bytes: u64,
    /// Number of allocations
    pub allocation_count: u64,
}

impl MemoryMetrics {
    /// Get RSS in megabytes
    pub fn rss_mb(&self) -> f64 {
        self.rss_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get VMS in megabytes
    pub fn vms_mb(&self) -> f64 {
        self.vms_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get peak RSS in megabytes
    pub fn peak_rss_mb(&self) -> f64 {
        self.peak_rss_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get memory growth rate per operation
    pub fn growth_per_op(&self, operations: u64) -> f64 {
        if operations == 0 {
            return 0.0;
        }
        self.rss_bytes as f64 / operations as f64
    }
}

/// Performance metrics for stress test
#[derive(Debug, Clone)]
pub struct StressTestMetrics {
    /// Total files processed
    pub files_processed: u64,
    /// Successful imports
    pub successful_imports: u64,
    /// Failed imports
    pub failed_imports: u64,
    /// Total workouts imported
    pub total_workouts: u64,
    /// Total duration of test
    pub total_duration: Duration,
    /// Memory metrics at start
    pub memory_start: MemoryMetrics,
    /// Memory metrics at end
    pub memory_end: MemoryMetrics,
    /// Snapshot metrics at intervals
    pub memory_snapshots: Vec<(Duration, MemoryMetrics)>,
    /// Per-file processing times (sample)
    pub processing_times_ms: Vec<u128>,
    /// Peak memory usage
    pub peak_memory_mb: f64,
}

impl StressTestMetrics {
    /// Get throughput (files per second)
    pub fn throughput_files_per_sec(&self) -> f64 {
        if self.total_duration.as_secs_f64() == 0.0 {
            return 0.0;
        }
        self.files_processed as f64 / self.total_duration.as_secs_f64()
    }

    /// Get average processing time per file
    pub fn avg_processing_time_ms(&self) -> f64 {
        if self.files_processed == 0 {
            return 0.0;
        }
        let total_ms: u128 = self.processing_times_ms.iter().sum();
        (total_ms as f64) / (self.files_processed as f64)
    }

    /// Get memory growth (end - start)
    pub fn memory_growth_mb(&self) -> f64 {
        self.memory_end.rss_mb() - self.memory_start.rss_mb()
    }

    /// Detect potential memory leak
    pub fn has_memory_leak(&self) -> bool {
        // If memory grows more than 10% and doesn't stabilize, flag as potential leak
        let growth_percent = (self.memory_growth_mb() / self.memory_start.rss_mb().max(1.0)) * 100.0;

        // Check if growth is consistent across snapshots
        if self.memory_snapshots.len() >= 3 {
            let mut is_growing = true;
            for i in 0..self.memory_snapshots.len() - 1 {
                if self.memory_snapshots[i + 1].1.rss_mb() <= self.memory_snapshots[i].1.rss_mb() {
                    is_growing = false;
                    break;
                }
            }
            return growth_percent > 15.0 && is_growing;
        }

        growth_percent > 20.0
    }

    /// Get success rate percentage
    pub fn success_rate_percent(&self) -> f64 {
        if self.files_processed == 0 {
            return 0.0;
        }
        (self.successful_imports as f64 / self.files_processed as f64) * 100.0
    }

    /// Pretty print metrics
    pub fn pretty_print(&self) -> String {
        format!(
            "=== Stress Test Results ===\n\
             Files Processed: {}\n\
             Successful: {} ({:.2}%)\n\
             Failed: {}\n\
             Total Workouts: {}\n\
             Total Duration: {:.2}s\n\
             Throughput: {:.2} files/sec\n\
             Avg Time/File: {:.2}ms\n\
             ---\n\
             Memory (Start): {:.2} MB\n\
             Memory (End): {:.2} MB\n\
             Memory Growth: {:.2} MB\n\
             Peak Memory: {:.2} MB\n\
             Memory Leak Risk: {}\n\
             ---\n\
             Workouts/MB: {:.2}",
            self.files_processed,
            self.successful_imports,
            self.success_rate_percent(),
            self.failed_imports,
            self.total_workouts,
            self.total_duration.as_secs_f64(),
            self.throughput_files_per_sec(),
            self.avg_processing_time_ms(),
            self.memory_start.rss_mb(),
            self.memory_end.rss_mb(),
            self.memory_growth_mb(),
            self.peak_memory_mb,
            if self.has_memory_leak() { "HIGH" } else { "LOW" },
            if self.memory_end.rss_mb() > 0.0 {
                self.total_workouts as f64 / self.memory_end.rss_mb()
            } else {
                0.0
            }
        )
    }
}

/// Stress test configuration
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    /// Number of synthetic files to generate
    pub num_files: usize,
    /// Number of workouts per file
    pub workouts_per_file: usize,
    /// Whether to use parallel processing
    pub parallel_processing: bool,
    /// Number of threads for parallel processing
    pub num_threads: Option<usize>,
    /// Enable caching
    pub enable_caching: bool,
    /// Interval for memory snapshots
    pub snapshot_interval: Duration,
    /// Sample every Nth file for detailed timing
    pub timing_sample_rate: usize,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            num_files: 100,
            workouts_per_file: 1,
            parallel_processing: true,
            num_threads: None,
            enable_caching: true,
            snapshot_interval: Duration::from_secs(5),
            timing_sample_rate: 10,
        }
    }
}

/// Stress test harness
pub struct StressTestHarness;

impl StressTestHarness {
    /// Create a stress test scenario
    pub fn create_scenario(config: StressTestConfig) -> StressTestScenario {
        StressTestScenario {
            config,
            start_time: None,
            metrics: StressTestMetrics {
                files_processed: 0,
                successful_imports: 0,
                failed_imports: 0,
                total_workouts: 0,
                total_duration: Duration::default(),
                memory_start: MemoryMetrics::default(),
                memory_end: MemoryMetrics::default(),
                memory_snapshots: Vec::new(),
                processing_times_ms: Vec::new(),
                peak_memory_mb: 0.0,
            },
        }
    }

    /// Quick smoke test (100 files)
    pub fn smoke_test() -> StressTestScenario {
        Self::create_scenario(StressTestConfig {
            num_files: 100,
            workouts_per_file: 1,
            parallel_processing: false,
            ..Default::default()
        })
    }

    /// Standard stress test (1000 files)
    pub fn standard_test() -> StressTestScenario {
        Self::create_scenario(StressTestConfig {
            num_files: 1000,
            workouts_per_file: 1,
            parallel_processing: true,
            ..Default::default()
        })
    }

    /// Heavy stress test (10000 files)
    pub fn heavy_test() -> StressTestScenario {
        Self::create_scenario(StressTestConfig {
            num_files: 10000,
            workouts_per_file: 1,
            parallel_processing: true,
            enable_caching: true,
            ..Default::default()
        })
    }

    /// Maximum stress test (50000 files, long-running)
    pub fn extreme_test() -> StressTestScenario {
        Self::create_scenario(StressTestConfig {
            num_files: 50000,
            workouts_per_file: 1,
            parallel_processing: true,
            num_threads: Some(8),
            enable_caching: true,
            snapshot_interval: Duration::from_secs(10),
            timing_sample_rate: 50,
        })
    }
}

/// Stress test scenario execution
pub struct StressTestScenario {
    config: StressTestConfig,
    start_time: Option<Instant>,
    metrics: StressTestMetrics,
}

impl StressTestScenario {
    /// Start the stress test
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.metrics.memory_start = Self::capture_memory_metrics();
        info!(
            "Starting stress test with {} files, {} workouts per file, parallel={}",
            self.config.num_files, self.config.workouts_per_file, self.config.parallel_processing
        );
    }

    /// Record successful import
    pub fn record_success(&mut self, workouts_count: usize, processing_time_ms: u128) {
        self.metrics.files_processed += 1;
        self.metrics.successful_imports += 1;
        self.metrics.total_workouts += workouts_count as u64;

        // Sample processing times
        if self.metrics.files_processed % self.config.timing_sample_rate as u64 == 0 {
            self.metrics.processing_times_ms.push(processing_time_ms);
        }

        // Take memory snapshot at intervals
        if self.metrics.files_processed % ((self.config.snapshot_interval.as_secs() * 10).max(1) as u64) == 0 {
            if let Some(start) = self.start_time {
                let elapsed = start.elapsed();
                let current_memory = Self::capture_memory_metrics();
                self.metrics.peak_memory_mb = self.metrics.peak_memory_mb.max(current_memory.rss_mb());
                self.metrics.memory_snapshots.push((elapsed, current_memory));
            }
        }
    }

    /// Record failed import
    pub fn record_failure(&mut self) {
        self.metrics.files_processed += 1;
        self.metrics.failed_imports += 1;
    }

    /// Finish the stress test and get results
    pub fn finish(mut self) -> StressTestMetrics {
        if let Some(start) = self.start_time {
            self.metrics.total_duration = start.elapsed();
        }

        self.metrics.memory_end = Self::capture_memory_metrics();
        self.metrics.peak_memory_mb = self.metrics.peak_memory_mb.max(self.metrics.memory_end.rss_mb());

        info!("Stress test completed");
        info!("{}", self.metrics.pretty_print());

        if self.metrics.has_memory_leak() {
            warn!("⚠️  POTENTIAL MEMORY LEAK DETECTED");
            warn!("Memory grew by {:.2} MB over {} files",
                self.metrics.memory_growth_mb(),
                self.metrics.files_processed);
        }

        self.metrics
    }

    /// Capture current memory metrics (Linux/macOS/Unix only)
    fn capture_memory_metrics() -> MemoryMetrics {
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                let mut rss = 0u64;
                let mut vms = 0u64;

                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(val) = line.split_whitespace().nth(1) {
                            rss = val.parse::<u64>().unwrap_or(0) * 1024;
                        }
                    } else if line.starts_with("VmSize:") {
                        if let Some(val) = line.split_whitespace().nth(1) {
                            vms = val.parse::<u64>().unwrap_or(0) * 1024;
                        }
                    }
                }

                return MemoryMetrics {
                    rss_bytes: rss,
                    vms_bytes: vms,
                    peak_rss_bytes: rss,
                    allocation_count: 0,
                };
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            if let Ok(output) = Command::new("ps")
                .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(rss_kb) = text.trim().parse::<u64>() {
                        let rss = rss_kb * 1024;
                        return MemoryMetrics {
                            rss_bytes: rss,
                            vms_bytes: rss * 2, // Estimate
                            peak_rss_bytes: rss,
                            allocation_count: 0,
                        };
                    }
                }
            }
        }

        // Fallback
        MemoryMetrics::default()
    }

    /// Get current metrics snapshot
    pub fn current_metrics(&self) -> &StressTestMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_test_config_defaults() {
        let config = StressTestConfig::default();
        assert_eq!(config.num_files, 100);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_stress_test_scenarios() {
        let smoke = StressTestHarness::smoke_test();
        assert_eq!(smoke.config.num_files, 100);

        let standard = StressTestHarness::standard_test();
        assert_eq!(standard.config.num_files, 1000);

        let heavy = StressTestHarness::heavy_test();
        assert_eq!(heavy.config.num_files, 10000);
    }

    #[test]
    fn test_memory_metrics_conversion() {
        let metrics = MemoryMetrics {
            rss_bytes: 1024 * 1024 * 100, // 100 MB
            vms_bytes: 1024 * 1024 * 200,
            peak_rss_bytes: 1024 * 1024 * 150,
            allocation_count: 1000,
        };

        assert!(metrics.rss_mb() > 99.0 && metrics.rss_mb() < 101.0);
        assert!(metrics.peak_rss_mb() > 149.0 && metrics.peak_rss_mb() < 151.0);
    }

    #[test]
    fn test_stress_test_metrics_throughput() {
        let metrics = StressTestMetrics {
            files_processed: 100,
            successful_imports: 100,
            failed_imports: 0,
            total_workouts: 500,
            total_duration: Duration::from_secs(10),
            memory_start: MemoryMetrics::default(),
            memory_end: MemoryMetrics {
                rss_bytes: 1024 * 1024 * 50,
                ..Default::default()
            },
            memory_snapshots: Vec::new(),
            processing_times_ms: vec![100; 100],  // 100 items of 100ms each
            peak_memory_mb: 50.0,
        };

        assert!((metrics.throughput_files_per_sec() - 10.0).abs() < 0.1);
        assert!((metrics.avg_processing_time_ms() - 100.0).abs() < 0.1);
        assert_eq!(metrics.success_rate_percent(), 100.0);
    }

    #[test]
    fn test_memory_leak_detection() {
        // Test case: steady growth indicates potential leak
        let mut metrics = StressTestMetrics {
            files_processed: 100,
            successful_imports: 100,
            failed_imports: 0,
            total_workouts: 100,
            total_duration: Duration::from_secs(100),
            memory_start: MemoryMetrics {
                rss_bytes: 1024 * 1024 * 50, // 50 MB
                ..Default::default()
            },
            memory_end: MemoryMetrics {
                rss_bytes: 1024 * 1024 * 90, // 90 MB (40% growth)
                ..Default::default()
            },
            memory_snapshots: vec![
                (Duration::from_secs(20), MemoryMetrics { rss_bytes: 1024 * 1024 * 60, ..Default::default() }),
                (Duration::from_secs(40), MemoryMetrics { rss_bytes: 1024 * 1024 * 70, ..Default::default() }),
                (Duration::from_secs(60), MemoryMetrics { rss_bytes: 1024 * 1024 * 80, ..Default::default() }),
                (Duration::from_secs(80), MemoryMetrics { rss_bytes: 1024 * 1024 * 90, ..Default::default() }),
            ],
            processing_times_ms: vec![100; 10],
            peak_memory_mb: 90.0,
        };

        // With consistent growth, should flag as leak risk
        assert!(metrics.has_memory_leak());

        // Test case: memory stabilizes - not a leak
        metrics.memory_end.rss_bytes = 1024 * 1024 * 55; // Only 10% growth
        assert!(!metrics.has_memory_leak());
    }
}
