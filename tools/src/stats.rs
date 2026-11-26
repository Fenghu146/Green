//! Percentiles and over-budget shares for the §9 metrics.

#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

impl Summary {
    pub fn of(samples: &[f64]) -> Option<Summary> {
        if samples.is_empty() {
            return None;
        }
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("a sample is not a number"));
        let count = sorted.len();
        Some(Summary {
            count,
            min: sorted[0],
            max: sorted[count - 1],
            mean: sorted.iter().sum::<f64>() / count as f64,
            p50: percentile(&sorted, 50.0),
            p95: percentile(&sorted, 95.0),
            p99: percentile(&sorted, 99.0),
        })
    }
}

/// Nearest rank: the sample at index `ceil(p/100 * n) - 1`, no interpolation.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    assert!(!sorted.is_empty(), "no samples");
    assert!((0.0..=100.0).contains(&p), "p is a percentage");
    let n = sorted.len();
    let rank = ((p / 100.0) * n as f64).ceil() as usize;
    sorted[rank.clamp(1, n) - 1]
}

/// The share of samples strictly above `limit`, 0.0..=1.0.
pub fn share_over(samples: &[f64], limit: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    count_over(samples, limit) as f64 / samples.len() as f64
}

/// How many samples are strictly above `limit`.
pub fn count_over(samples: &[f64], limit: f64) -> usize {
    samples.iter().filter(|sample| **sample > limit).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hundreds() -> Vec<f64> {
        (1..=100).map(f64::from).collect()
    }

    #[test]
    fn nearest_rank_lands_on_whole_samples() {
        let sorted = hundreds();
        assert_eq!(percentile(&sorted, 50.0), 50.0);
        assert_eq!(percentile(&sorted, 95.0), 95.0);
        assert_eq!(percentile(&sorted, 99.0), 99.0);
        assert_eq!(percentile(&sorted, 100.0), 100.0);
        assert_eq!(percentile(&[7.0], 99.0), 7.0);
    }

    #[test]
    fn small_samples_round_up_not_down() {
        let sorted = [1.0, 2.0, 3.0];
        assert_eq!(percentile(&sorted, 50.0), 2.0);
        assert_eq!(percentile(&sorted, 10.0), 1.0);
        assert_eq!(percentile(&sorted, 95.0), 3.0);
    }

    #[test]
    fn share_over_is_strictly_greater() {
        let samples = [15.9, 16.0, 16.1, 33.0];
        assert_eq!(count_over(&samples, 16.0), 2);
        assert_eq!(share_over(&samples, 16.0), 0.5);
        assert_eq!(share_over(&[], 16.0), 0.0);
    }

    #[test]
    fn summary_of_unsorted_samples() {
        let summary = Summary::of(&[9.0, 1.0, 5.0]).expect("three samples");
        assert_eq!(summary.count, 3);
        assert_eq!((summary.min, summary.max), (1.0, 9.0));
        assert_eq!(summary.p50, 5.0);
        assert_eq!(summary.mean, 5.0);
        assert!(Summary::of(&[]).is_none());
    }
}
