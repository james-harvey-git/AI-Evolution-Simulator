use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

impl Default for MetricSummary {
    fn default() -> Self {
        Self {
            count: 0,
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            p50: 0.0,
            p90: 0.0,
            p95: 0.0,
            p99: 0.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetricAggregator {
    samples: Vec<f64>,
    sum: f64,
    min: f64,
    max: f64,
}

impl MetricAggregator {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            sum: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    pub fn push(&mut self, value: f64) {
        self.sum += value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.samples.push(value);
    }

    pub fn pct_leq(&self, threshold: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let hits = self.samples.iter().filter(|v| **v <= threshold).count();
        (hits as f64 / self.samples.len() as f64) * 100.0
    }

    pub fn summary(&self) -> MetricSummary {
        if self.samples.is_empty() {
            return MetricSummary::default();
        }

        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));

        MetricSummary {
            count: self.samples.len(),
            min: self.min,
            max: self.max,
            mean: self.sum / self.samples.len() as f64,
            p50: percentile_nearest_rank(&sorted, 0.50),
            p90: percentile_nearest_rank(&sorted, 0.90),
            p95: percentile_nearest_rank(&sorted, 0.95),
            p99: percentile_nearest_rank(&sorted, 0.99),
        }
    }
}

fn percentile_nearest_rank(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let p = p.clamp(0.0, 1.0);
    let rank = ((p * sorted.len() as f64).ceil() as usize).saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_summary_is_reasonable() {
        let mut agg = MetricAggregator::new();
        for i in 1..=100 {
            agg.push(i as f64);
        }
        let s = agg.summary();
        assert_eq!(s.count, 100);
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 100.0);
        assert!((s.mean - 50.5).abs() < 1e-6);
        assert_eq!(s.p50, 50.0);
        assert_eq!(s.p90, 90.0);
        assert_eq!(s.p95, 95.0);
        assert_eq!(s.p99, 99.0);
    }

    #[test]
    fn pct_leq_handles_empty_and_populated() {
        let mut agg = MetricAggregator::new();
        assert_eq!(agg.pct_leq(10.0), 0.0);
        agg.push(5.0);
        agg.push(15.0);
        agg.push(10.0);
        assert!((agg.pct_leq(10.0) - 66.666666).abs() < 0.01);
    }
}
