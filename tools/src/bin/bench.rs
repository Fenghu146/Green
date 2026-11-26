//! Reads the product's raw measurement log and prints §9 statistics.

use std::collections::BTreeMap;
use std::process::ExitCode;

use green_tools::budget::{self, BUDGETS};
use green_tools::json::{self, number_field, string_field};
use green_tools::stats::Summary;

const USAGE: &str = "\
Usage: bench [--json PATH] <stats|check> FILE...

  FILE            one JSON object per line: {\"metric\":\"k2p\",\"ms\":7.4}
  stats           count, min, p50, p95, p99, max, mean per metric
  check           the same, judged against docs/design.md §9; exit 1 on a breach
  --json PATH     also write the numbers as JSON";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut json_path = None;
    if let Some(at) = args.iter().position(|arg| arg == "--json") {
        if at + 1 >= args.len() {
            eprintln!("bench: --json needs a path\n\n{USAGE}");
            return ExitCode::from(2);
        }
        json_path = Some(args.remove(at + 1));
        args.remove(at);
    }
    let Some(command) = args.first().cloned() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if command == "--help" || command == "-h" {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let check = match command.as_str() {
        "stats" => false,
        "check" => true,
        other => {
            eprintln!("bench: no command {other:?}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let files = &args[1..];
    if files.is_empty() {
        eprintln!("bench: {command} needs at least one file\n\n{USAGE}");
        return ExitCode::from(2);
    }

    let samples = match read(files) {
        Ok(samples) => samples,
        Err(problem) => {
            eprintln!("bench: {problem}");
            return ExitCode::from(2);
        }
    };
    if samples.is_empty() {
        eprintln!("bench: no samples in {files:?}");
        return ExitCode::from(2);
    }

    print_table(&samples);
    let mut failed = false;
    if check {
        println!();
        failed = print_verdicts(&samples);
    }
    if let Some(path) = json_path
        && let Err(problem) = std::fs::write(&path, to_json(&samples, check))
    {
        eprintln!("bench: {path}: {problem}");
        return ExitCode::from(2);
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

type Samples = BTreeMap<String, Vec<f64>>;

fn read(files: &[String]) -> Result<Samples, String> {
    let mut samples = Samples::new();
    let mut unknown = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(file).map_err(|error| format!("{file}: {error}"))?;
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let where_ = format!("{file}:{}", index + 1);
            let fields = json::object(line).map_err(|error| format!("{where_}: {error}"))?;
            let metric = string_field(&fields, "metric")
                .ok_or_else(|| format!("{where_}: no string \"metric\""))?;
            let ms =
                number_field(&fields, "ms").ok_or_else(|| format!("{where_}: no number \"ms\""))?;
            if ms < 0.0 {
                return Err(format!("{where_}: a negative \"ms\""));
            }
            if budget::budget_for(metric).is_none() && !unknown.contains(&metric.to_string()) {
                unknown.push(metric.to_string());
            }
            samples.entry(metric.to_string()).or_default().push(ms);
        }
    }
    if !unknown.is_empty() {
        let known = BUDGETS
            .iter()
            .map(|budget| budget.metric)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "no §9 budget for {}; known metrics: {known}",
            unknown.join(", ")
        ));
    }
    Ok(samples)
}

fn print_table(samples: &Samples) {
    println!(
        "{:<16} {:>6} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "metric", "n", "min", "p50", "p95", "p99", "max", "mean"
    );
    for (metric, values) in samples {
        let summary = Summary::of(values).expect("a metric has samples");
        println!(
            "{:<16} {:>6} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2}",
            metric,
            summary.count,
            summary.min,
            summary.p50,
            summary.p95,
            summary.p99,
            summary.max,
            summary.mean
        );
    }
}

/// Prints one line per cap; true if any cap is missed.
fn print_verdicts(samples: &Samples) -> bool {
    let mut failed = false;
    for budget in BUDGETS {
        let Some(values) = samples.get(budget.metric) else {
            println!("{:<16} not measured", budget.metric);
            continue;
        };
        for verdict in budget::evaluate(budget, values) {
            failed |= !verdict.ok;
            let mark = if verdict.ok { "PASS" } else { "FAIL" };
            println!("{:<16} {:<44} {mark}", budget.metric, verdict.label);
        }
    }
    failed
}

fn to_json(samples: &Samples, check: bool) -> String {
    let mut out = String::from("{\"metrics\":[");
    for (index, (metric, values)) in samples.iter().enumerate() {
        let summary = Summary::of(values).expect("a metric has samples");
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "\n  {{\"metric\":\"{}\",\"unit\":\"{}\",\"count\":{},\"min\":{:.4},\"p50\":{:.4},\"p95\":{:.4},\"p99\":{:.4},\"max\":{:.4},\"mean\":{:.4}",
            metric,
            budget::budget_for(metric).expect("known metric").unit,
            summary.count,
            summary.min,
            summary.p50,
            summary.p95,
            summary.p99,
            summary.max,
            summary.mean
        ));
        if check {
            let budget = budget::budget_for(metric).expect("known metric");
            let verdicts = budget::evaluate(budget, values);
            out.push_str(",\"caps\":[");
            for (at, verdict) in verdicts.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                out.push_str(&format!(
                    "{{\"rule\":\"{}\",\"value\":{:.4},\"ok\":{}}}",
                    verdict.label.replace('"', "'"),
                    verdict.value,
                    verdict.ok
                ));
            }
            out.push(']');
        }
        out.push('}');
    }
    out.push_str("\n]}\n");
    out
}
