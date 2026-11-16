//! Performance metrics instrumentation, latency tracking, and budget enforcement
//! for the Antares IDE AI agent panel and editor interactions.

use serde::{Deserialize, Serialize};

/// Maximum number of latency samples retained in ring buffers.
const MAX_SAMPLES: usize = 1_000;

/// Violations reported when performance exceeds specified budgets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetViolation {
    TypingLatencyExceeded {
        measured_ms: u64,
        budget_ms: u64,
    },
    ScrollFrameExceeded {
        measured_ms: u64,
        budget_ms: u64,
    },
    CancellationLatencyExceeded {
        measured_ms: u64,
        budget_ms: u64,
    },
    EventQueueOverflow {
        measured_depth: usize,
        max_depth: usize,
    },
}

/// Latency statistics calculated from metric samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatencyStats {
    pub count: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl LatencyStats {
    pub fn compute(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self {
                count: 0,
                min_ms: 0.0,
                max_ms: 0.0,
                mean_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
            };
        }

        let count = samples.len();
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min_ms = sorted[0];
        let max_ms = sorted[count - 1];
        let sum: f64 = sorted.iter().sum();
        let mean_ms = sum / count as f64;

        let p95_idx = ((count as f64 * 0.95).ceil() as usize).saturating_sub(1);
        let p99_idx = ((count as f64 * 0.99).ceil() as usize).saturating_sub(1);
        let p95_ms = sorted[p95_idx.min(count - 1)];
        let p99_ms = sorted[p99_idx.min(count - 1)];

        Self {
            count,
            min_ms,
            max_ms,
            mean_ms,
            p95_ms,
            p99_ms,
        }
    }
}

/// Budget configuration for responsiveness and throughput.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceBudgets {
    /// Target typing latency in milliseconds (default: 16.6ms / 60 FPS).
    pub max_typing_latency_ms: f64,
    /// Target scroll frame duration in milliseconds (default: 16.6ms).
    pub max_scroll_frame_ms: f64,
    /// Target ACP prompt cancellation latency in milliseconds (default: 500ms).
    pub max_cancellation_latency_ms: f64,
    /// Maximum allowed event queue depth before backpressure triggers (default: 1,000).
    pub max_event_queue_depth: usize,
}

impl Default for PerformanceBudgets {
    fn default() -> Self {
        Self {
            max_typing_latency_ms: 16.6,
            max_scroll_frame_ms: 16.6,
            max_cancellation_latency_ms: 500.0,
            max_event_queue_depth: 1_000,
        }
    }
}

/// Real-time performance metrics collector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    typing_latencies: Vec<f64>,
    scroll_frame_times: Vec<f64>,
    context_build_latencies: Vec<f64>,
    cancellation_latencies: Vec<f64>,
    pub current_queue_depth: usize,
    pub peak_queue_depth: usize,
    pub budgets: PerformanceBudgets,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new(PerformanceBudgets::default())
    }
}

impl PerformanceMetrics {
    pub fn new(budgets: PerformanceBudgets) -> Self {
        Self {
            typing_latencies: Vec::new(),
            scroll_frame_times: Vec::new(),
            context_build_latencies: Vec::new(),
            cancellation_latencies: Vec::new(),
            current_queue_depth: 0,
            peak_queue_depth: 0,
            budgets,
        }
    }

    fn push_sample(buffer: &mut Vec<f64>, value: f64) {
        if buffer.len() >= MAX_SAMPLES {
            buffer.remove(0);
        }
        buffer.push(value);
    }

    /// Record composer typing latency in milliseconds.
    pub fn record_typing_latency(&mut self, ms: f64) {
        Self::push_sample(&mut self.typing_latencies, ms);
    }

    /// Record transcript scroll frame duration in milliseconds.
    pub fn record_scroll_frame(&mut self, ms: f64) {
        Self::push_sample(&mut self.scroll_frame_times, ms);
    }

    /// Record context assembly and secret redaction latency in milliseconds.
    pub fn record_context_build(&mut self, ms: f64) {
        Self::push_sample(&mut self.context_build_latencies, ms);
    }

    /// Record ACP prompt cancellation latency in milliseconds.
    pub fn record_cancellation_latency(&mut self, ms: f64) {
        Self::push_sample(&mut self.cancellation_latencies, ms);
    }

    /// Update current event queue depth.
    pub fn set_queue_depth(&mut self, depth: usize) {
        self.current_queue_depth = depth;
        if depth > self.peak_queue_depth {
            self.peak_queue_depth = depth;
        }
    }

    pub fn typing_stats(&self) -> LatencyStats {
        LatencyStats::compute(&self.typing_latencies)
    }

    pub fn scroll_stats(&self) -> LatencyStats {
        LatencyStats::compute(&self.scroll_frame_times)
    }

    pub fn context_build_stats(&self) -> LatencyStats {
        LatencyStats::compute(&self.context_build_latencies)
    }

    pub fn cancellation_stats(&self) -> LatencyStats {
        LatencyStats::compute(&self.cancellation_latencies)
    }

    /// Check recorded metrics against configured performance budgets.
    pub fn check_budgets(&self) -> Vec<BudgetViolation> {
        let mut violations = Vec::new();

        let typing_p95 = self.typing_stats().p95_ms;
        if typing_p95 > self.budgets.max_typing_latency_ms {
            violations.push(BudgetViolation::TypingLatencyExceeded {
                measured_ms: typing_p95.round() as u64,
                budget_ms: self.budgets.max_typing_latency_ms.round() as u64,
            });
        }

        let scroll_p95 = self.scroll_stats().p95_ms;
        if scroll_p95 > self.budgets.max_scroll_frame_ms {
            violations.push(BudgetViolation::ScrollFrameExceeded {
                measured_ms: scroll_p95.round() as u64,
                budget_ms: self.budgets.max_scroll_frame_ms.round() as u64,
            });
        }

        let cancel_max = self.cancellation_stats().max_ms;
        if cancel_max > self.budgets.max_cancellation_latency_ms {
            violations.push(BudgetViolation::CancellationLatencyExceeded {
                measured_ms: cancel_max.round() as u64,
                budget_ms: self.budgets.max_cancellation_latency_ms.round() as u64,
            });
        }

        if self.peak_queue_depth > self.budgets.max_event_queue_depth {
            violations.push(BudgetViolation::EventQueueOverflow {
                measured_depth: self.peak_queue_depth,
                max_depth: self.budgets.max_event_queue_depth,
            });
        }

        violations
    }
}
