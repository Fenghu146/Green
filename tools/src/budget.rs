//! The §9 performance budgets, as data the tool evaluates.

use crate::stats::{Summary, count_over, share_over};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stat {
    P50,
    P95,
    P99,
    Max,
    /// The share of samples above a limit, as a fraction.
    ShareOver(f64),
    /// How many samples are above a limit.
    CountOver(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bound {
    AtMost(f64),
    Below(f64),
}

#[derive(Debug, Clone, Copy)]
pub struct Cap {
    pub stat: Stat,
    pub bound: Bound,
}

#[derive(Debug, Clone, Copy)]
pub struct Budget {
    pub metric: &'static str,
    pub unit: &'static str,
    pub caps: &'static [Cap],
}

const fn at_most(stat: Stat, limit: f64) -> Cap {
    Cap {
        stat,
        bound: Bound::AtMost(limit),
    }
}

const fn below(stat: Stat, limit: f64) -> Cap {
    Cap {
        stat,
        bound: Bound::Below(limit),
    }
}

/// docs/design.md §9, one entry per metric the product can log.
pub const BUDGETS: &[Budget] = &[
    Budget {
        metric: "k2p",
        unit: "ms",
        caps: &[
            at_most(Stat::P50, 8.0),
            at_most(Stat::P95, 16.0),
            at_most(Stat::P99, 33.0),
            below(Stat::ShareOver(16.0), 0.01),
        ],
    },
    // A frame over 16.67 ms is a dropped frame at 60 Hz.
    Budget {
        metric: "scroll",
        unit: "ms",
        caps: &[below(Stat::ShareOver(1000.0 / 60.0), 0.01)],
    },
    // Folding or unfolding 50k characters: worst case, not a percentile.
    Budget {
        metric: "fold",
        unit: "ms",
        caps: &[at_most(Stat::Max, 16.0)],
    },
    // The shape calls during one fold; each sample is a count for one operation.
    Budget {
        metric: "fold.shape",
        unit: "calls",
        caps: &[at_most(Stat::Max, 0.0)],
    },
    Budget {
        metric: "open",
        unit: "ms",
        caps: &[at_most(Stat::P95, 400.0)],
    },
    Budget {
        metric: "cold-start",
        unit: "ms",
        caps: &[at_most(Stat::P95, 2000.0)],
    },
    Budget {
        metric: "hot-start",
        unit: "ms",
        caps: &[at_most(Stat::P95, 600.0)],
    },
    Budget {
        metric: "tab-switch",
        unit: "ms",
        caps: &[at_most(Stat::P95, 100.0)],
    },
    Budget {
        metric: "long-task",
        unit: "ms",
        caps: &[at_most(Stat::CountOver(50.0), 0.0)],
    },
    Budget {
        metric: "memory",
        unit: "MB",
        caps: &[at_most(Stat::Max, 200.0)],
    },
    Budget {
        metric: "incremental-build",
        unit: "ms",
        caps: &[at_most(Stat::P95, 60_000.0)],
    },
];

pub fn budget_for(metric: &str) -> Option<&'static Budget> {
    BUDGETS.iter().find(|budget| budget.metric == metric)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub label: String,
    pub value: f64,
    pub ok: bool,
}

/// One verdict per cap, with the measured value it was judged on.
pub fn evaluate(budget: &Budget, samples: &[f64]) -> Vec<Verdict> {
    let summary = Summary::of(samples).expect("a metric with samples");
    budget
        .caps
        .iter()
        .map(|cap| {
            let value = match cap.stat {
                Stat::P50 => summary.p50,
                Stat::P95 => summary.p95,
                Stat::P99 => summary.p99,
                Stat::Max => summary.max,
                Stat::ShareOver(over) => share_over(samples, over),
                Stat::CountOver(over) => count_over(samples, over) as f64,
            };
            let ok = match cap.bound {
                Bound::AtMost(limit) => value <= limit,
                Bound::Below(limit) => value < limit,
            };
            let words = match (cap.stat, cap.bound) {
                (Stat::ShareOver(over), Bound::Below(limit)) => {
                    format!("> {over:.2} {} share < {:.2}%", budget.unit, limit * 100.0)
                }
                (Stat::ShareOver(over), Bound::AtMost(limit)) => {
                    format!("> {over:.2} {} share <= {:.2}%", budget.unit, limit * 100.0)
                }
                (Stat::CountOver(over), Bound::AtMost(limit)) => {
                    format!("> {over:.0} {} count <= {limit:.0}", budget.unit)
                }
                (stat, Bound::AtMost(limit)) => {
                    format!("{} <= {limit:.2} {}", stat_name(stat), budget.unit)
                }
                (stat, Bound::Below(limit)) => {
                    format!("{} < {limit:.2} {}", stat_name(stat), budget.unit)
                }
            };
            let shown = match cap.stat {
                Stat::ShareOver(_) => format!("{:.2}%", value * 100.0),
                Stat::CountOver(_) => format!("{value:.0}"),
                _ => format!("{value:.2}"),
            };
            Verdict {
                label: format!("{words}: {shown}"),
                value,
                ok,
            }
        })
        .collect()
}

fn stat_name(stat: Stat) -> &'static str {
    match stat {
        Stat::P50 => "p50",
        Stat::P95 => "p95",
        Stat::P99 => "p99",
        Stat::Max => "max",
        Stat::ShareOver(_) => "share",
        Stat::CountOver(_) => "count",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k2p() -> &'static Budget {
        budget_for("k2p").expect("k2p has a budget")
    }

    #[test]
    fn a_k2p_run_inside_the_budget_passes() {
        let samples: Vec<f64> = (1..=100).map(|i| f64::from(i) / 10.0).collect();
        let verdicts = evaluate(k2p(), &samples);
        assert_eq!(verdicts.len(), 4);
        assert!(verdicts.iter().all(|verdict| verdict.ok), "{verdicts:?}");
        assert!(verdicts[3].label.contains("0.00%"), "{}", verdicts[3].label);
    }

    #[test]
    fn a_slow_tail_breaks_p95_not_p50() {
        let mut samples: Vec<f64> = (1..=100).map(|i| f64::from(i) / 10.0).collect();
        for sample in &mut samples[90..] {
            *sample = 40.0;
        }
        let verdicts = evaluate(k2p(), &samples);
        assert!(verdicts[0].ok, "p50 stays low: {verdicts:?}");
        assert!(!verdicts[1].ok, "p95 sees the tail: {verdicts:?}");
        assert!(!verdicts[2].ok, "p99 sees it too: {verdicts:?}");
        assert!(
            !verdicts[3].ok,
            "10% over 16 ms is over the 1% share: {verdicts:?}"
        );
    }

    #[test]
    fn the_share_cap_is_strict_at_one_percent() {
        let mut samples = vec![5.0; 200];
        samples[199] = 20.0;
        assert!(evaluate(k2p(), &samples)[3].ok, "0.5% is below 1%");
        samples[198] = 20.0;
        assert!(!evaluate(k2p(), &samples)[3].ok, "1% is not below 1%");
    }

    #[test]
    fn a_fold_that_shapes_anything_fails() {
        let shape = budget_for("fold.shape").expect("a budget");
        assert!(evaluate(shape, &[0.0, 0.0])[0].ok);
        assert!(!evaluate(shape, &[0.0, 3.0])[0].ok);
        let fold = budget_for("fold").expect("a budget");
        assert!(
            evaluate(fold, &[16.0])[0].ok,
            "16 ms is the limit, not over it"
        );
        assert!(!evaluate(fold, &[16.5])[0].ok);
    }

    #[test]
    fn any_long_task_fails() {
        let budget = budget_for("long-task").expect("a budget");
        assert!(
            evaluate(budget, &[49.0, 50.0])[0].ok,
            "50 ms is not over 50 ms"
        );
        assert!(!evaluate(budget, &[49.0, 51.0])[0].ok);
    }
}
