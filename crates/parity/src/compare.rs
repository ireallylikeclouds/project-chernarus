//! Run a scenario in the simulation and compare it with reference captures.

use crate::metrics::{Measured, measure};
use crate::scenario::Scenario;
use crate::trace::{Sample, Trace};
use chernarus_core::Verification;
use chernarus_core::param::ParamUse;
use chernarus_core::time::TickRate;
use chernarus_sim::Simulation;
use chernarus_sim::movement::{MovementParams, PlayerMotion};
use serde::Serialize;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

/// Reference runs needed before a scenario's reference behaviour counts as
/// VERIFIED (fewer gives PARTIALLY VERIFIED). A project convention.
pub const RUNS_FOR_VERIFIED: usize = 3;

/// Run the scenario and record the player's trace at every tick.
pub fn simulate(scenario: &Scenario, params: Arc<MovementParams>) -> (Trace, Vec<ParamUse>) {
    let rate = TickRate::new(scenario.tick_rate_hz).unwrap_or_default();
    let mut sim = Simulation::new(rate, params);
    let i = &scenario.initial;
    let id = sim.spawn_player(PlayerMotion::at(i.position, i.heading_deg, i.stance));
    let mut trace = Trace::default();
    trace.meta.insert("scenario".into(), scenario.id.clone());
    trace.meta.insert("source".into(), "simulation".into());
    trace.meta.insert("tick_rate_hz".into(), rate.hz().to_string());
    let ticks = rate.ticks_for(scenario.duration_s);
    let mut record = |sim: &Simulation| {
        if let Some(p) = sim.player(id) {
            trace.samples.push(Sample {
                t: sim.time(),
                pos: p.position,
                heading_deg: p.heading_deg,
                speed_kmh: Some(p.speed() * 3.6),
                anim: None,
            });
        }
    };
    record(&sim);
    for _ in 0..ticks {
        let intent = scenario.intent_at(sim.time());
        sim.step(&[intent]);
        record(&sim);
    }
    let uses = sim.ledger().uses().cloned().collect();
    (trace, uses)
}

/// Load every reference trace for a scenario from `<dir>/<scenario id>/*.csv`.
pub fn load_reference(dir: &Path, scenario: &Scenario) -> Result<Vec<Trace>, String> {
    let sdir = dir.join(&scenario.id);
    if !sdir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<_> = std::fs::read_dir(&sdir)
        .map_err(|e| format!("{}: {e}", sdir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "csv"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|p| {
            let t = Trace::load(p).map_err(|e| format!("{}: {e}", p.display()))?;
            match t.meta("scenario") {
                Some(s) if s == scenario.id => Ok(t),
                other => Err(format!("{}: trace is for scenario {other:?}, not {}", p.display(), scenario.id)),
            }
        })
        .collect()
}

/// Outcome of comparing one metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    Fail,
    /// No usable reference measurement: nothing to compare against.
    NoReference,
    /// The simulation trace could not be measured.
    SimUnmeasurable,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricReport {
    pub name: String,
    pub sim: Option<f64>,
    pub sim_note: Option<String>,
    pub reference_mean: Option<f64>,
    pub reference_std: Option<f64>,
    pub reference_runs: usize,
    pub difference: Option<f64>,
    pub tolerance: f64,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioReport {
    pub scenario: String,
    pub system: String,
    pub description: String,
    pub metrics: Vec<MetricReport>,
    /// Overall comparison outcome.
    pub verdict: Verdict,
    /// How well the reference behaviour of this scenario is established.
    pub reference_status: Verification,
    /// Weakest knowledge status among the simulation parameters used.
    pub sim_inputs_status: Verification,
    pub params_used: Vec<ParamUse>,
    pub reference_runs: usize,
}

fn mean_std(values: &[f64]) -> Option<(f64, f64)> {
    if values.is_empty() {
        return None;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = if values.len() > 1 { values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0) } else { 0.0 };
    Some((mean, var.sqrt()))
}

/// Compare a simulation trace with reference traces.
pub fn compare(scenario: &Scenario, sim: &Trace, params_used: Vec<ParamUse>, reference: &[Trace]) -> ScenarioReport {
    let sim_m = measure(sim, &scenario.analysis, &scenario.metrics);
    let ref_m: Vec<Vec<Measured>> =
        reference.iter().map(|t| measure(t, &scenario.analysis, &scenario.metrics)).collect();

    let metrics: Vec<MetricReport> = scenario
        .metrics
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            let refs: Vec<f64> = ref_m.iter().filter_map(|m| m[i].value()).collect();
            let stats = mean_std(&refs);
            let sim_value = sim_m[i].value();
            let difference = sim_value.zip(stats).map(|(s, (m, _))| s - m);
            let verdict = match (sim_value, difference) {
                (None, _) => Verdict::SimUnmeasurable,
                (Some(_), None) => Verdict::NoReference,
                (Some(_), Some(d)) if d.abs() <= spec.tolerance() => Verdict::Pass,
                _ => Verdict::Fail,
            };
            MetricReport {
                name: spec.name(),
                sim: sim_value,
                sim_note: match &sim_m[i] {
                    Measured::Unavailable(why) => Some(why.clone()),
                    Measured::Value(_) => None,
                },
                reference_mean: stats.map(|s| s.0),
                reference_std: stats.map(|s| s.1),
                reference_runs: refs.len(),
                difference,
                tolerance: spec.tolerance(),
                verdict,
            }
        })
        .collect();

    let verdict = if metrics.iter().any(|m| m.verdict == Verdict::SimUnmeasurable) {
        Verdict::SimUnmeasurable
    } else if metrics.iter().any(|m| m.verdict == Verdict::NoReference) {
        Verdict::NoReference
    } else if metrics.iter().any(|m| m.verdict == Verdict::Fail) {
        Verdict::Fail
    } else {
        Verdict::Pass
    };
    let runs = metrics.iter().map(|m| m.reference_runs).min().unwrap_or(0);
    let consistent = metrics.iter().all(|m| m.reference_std.is_some_and(|s| s <= m.tolerance));
    let reference_status = match runs {
        0 => Verification::Unknown,
        n if n >= RUNS_FOR_VERIFIED && consistent => Verification::Verified,
        _ => Verification::PartiallyVerified,
    };
    ScenarioReport {
        scenario: scenario.id.clone(),
        system: scenario.system.clone(),
        description: scenario.description.clone(),
        metrics,
        verdict,
        reference_status,
        sim_inputs_status: Verification::weakest(params_used.iter().map(|p| p.status)),
        params_used,
        reference_runs: runs,
    }
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map_or_else(|| "—".to_owned(), |v| format!("{v:.3}"))
}

impl ScenarioReport {
    /// Markdown summary for humans and for `docs/parity/`.
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "### `{}` ({})\n", self.scenario, self.system);
        let _ = writeln!(s, "{}\n", self.description);
        let _ = writeln!(
            s,
            "- Verdict: **{:?}**\n- Reference behaviour: **{}** ({} run(s))\n- Simulation inputs: **{}** (weakest of {} parameter(s) used)\n",
            self.verdict,
            self.reference_status,
            self.reference_runs,
            self.sim_inputs_status,
            self.params_used.len()
        );
        let _ = writeln!(s, "| Metric | Rust | Reference (mean ± sd, n) | Difference | Tolerance | Verdict |");
        let _ = writeln!(s, "|---|---|---|---|---|---|");
        for m in &self.metrics {
            let reference = match (m.reference_mean, m.reference_std) {
                (Some(mean), Some(sd)) => format!("{mean:.3} ± {sd:.3} (n={})", m.reference_runs),
                _ => "none".to_owned(),
            };
            let sim = match (&m.sim, &m.sim_note) {
                (Some(v), _) => format!("{v:.3}"),
                (None, Some(note)) => format!("— ({note})"),
                (None, None) => "—".to_owned(),
            };
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} | {} | {:?} |",
                m.name,
                sim,
                reference,
                fmt_opt(m.difference),
                m.tolerance,
                m.verdict
            );
        }
        let _ = writeln!(s, "\nParameters used:\n");
        for p in &self.params_used {
            let _ = writeln!(s, "- `{}` — {} ({})", p.name, p.status, p.source);
        }
        s
    }
}
