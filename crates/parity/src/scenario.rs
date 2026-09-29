//! Parity scenario definitions (`tests/parity/scenarios/*.toml`).

use chernarus_sim::movement::{MoveIntent, Pace, Stance};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Stable id, also the reference-trace directory name.
    pub id: String,
    /// Parity-matrix system this scenario measures.
    pub system: String,
    pub description: String,
    /// How to perform the scenario in the reference game.
    pub capture_procedure: String,
    #[serde(default = "default_tick_rate")]
    pub tick_rate_hz: u32,
    pub duration_s: f64,
    pub initial: Initial,
    #[serde(rename = "input")]
    pub inputs: Vec<InputSegment>,
    pub analysis: Analysis,
    #[serde(rename = "metric")]
    pub metrics: Vec<MetricSpec>,
}

fn default_tick_rate() -> u32 {
    60
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Initial {
    #[serde(default)]
    pub position: [f64; 3],
    #[serde(default)]
    pub heading_deg: f64,
    pub stance: Stance,
}

/// Input held from `at_s` until the next segment starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSegment {
    pub at_s: f64,
    #[serde(default)]
    pub forward: i8,
    #[serde(default)]
    pub right: i8,
    #[serde(default = "default_pace")]
    pub pace: Pace,
    /// Inherits the previous segment's (or the initial) stance if absent.
    pub stance: Option<Stance>,
    /// Inherits the previous heading if absent.
    pub heading_deg: Option<f64>,
}

fn default_pace() -> Pace {
    Pace::Run
}

/// How traces are analysed. Applied identically to simulation and reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Analysis {
    /// Horizontal displacement that marks motion onset (aligns traces whose
    /// input timing is unknown, such as human captures).
    pub onset_threshold_m: f64,
    /// Window after onset in which the unit is expected to move at constant
    /// speed, seconds.
    pub steady_window_s: [f64; 2],
    /// Half-width of the central difference used to estimate speed, seconds.
    pub speed_half_window_s: f64,
}

/// A measurement and the largest acceptable sim-vs-reference difference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MetricSpec {
    /// Least-squares speed over the steady window, m/s.
    SteadySpeed { tolerance: f64 },
    /// Time from onset until speed first reaches `fraction` of steady speed, s.
    TimeToFraction { fraction: f64, tolerance: f64 },
    /// Distance covered after speed first falls below `fraction` of steady
    /// speed (following the steady window) until the end of the trace, m.
    StopDistance { fraction: f64, tolerance: f64 },
}

impl MetricSpec {
    pub fn name(&self) -> String {
        match self {
            MetricSpec::SteadySpeed { .. } => "steady_speed_mps".into(),
            MetricSpec::TimeToFraction { fraction, .. } => format!("time_to_{}pct_speed_s", (fraction * 100.0).round()),
            MetricSpec::StopDistance { fraction, .. } => {
                format!("stop_distance_below_{}pct_m", (fraction * 100.0).round())
            }
        }
    }

    pub fn tolerance(&self) -> f64 {
        match self {
            MetricSpec::SteadySpeed { tolerance }
            | MetricSpec::TimeToFraction { tolerance, .. }
            | MetricSpec::StopDistance { tolerance, .. } => *tolerance,
        }
    }
}

#[derive(Debug, Error)]
pub enum ScenarioError {
    #[error("cannot read {0}: {1}")]
    Io(String, std::io::Error),
    #[error("cannot parse {0}: {1}")]
    Parse(String, Box<toml::de::Error>),
    #[error("scenario {id}: {detail}")]
    Invalid { id: String, detail: String },
}

impl Scenario {
    pub fn load(path: &Path) -> Result<Self, ScenarioError> {
        let name = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|e| ScenarioError::Io(name.clone(), e))?;
        Self::parse(&text).map_err(|e| match e {
            ScenarioError::Parse(_, err) => ScenarioError::Parse(name, err),
            other => other,
        })
    }

    pub fn parse(text: &str) -> Result<Self, ScenarioError> {
        let scenario: Scenario = toml::from_str(text).map_err(|e| ScenarioError::Parse(String::new(), Box::new(e)))?;
        scenario.validate()?;
        Ok(scenario)
    }

    fn validate(&self) -> Result<(), ScenarioError> {
        let fail = |detail: &str| Err(ScenarioError::Invalid { id: self.id.clone(), detail: detail.to_owned() });
        if self.id.is_empty() || !self.id.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
            return fail("id must be non-empty and use only [A-Za-z0-9._-]");
        }
        // Every comparison below also rejects NaN, which compares false.
        let positive = |v: f64| v.is_finite() && v > 0.0;
        let non_negative = |v: f64| v.is_finite() && v >= 0.0;
        if self.tick_rate_hz == 0 || !positive(self.duration_s) {
            return fail("tick_rate_hz and duration_s must be positive");
        }
        let finite_inputs = self.inputs.iter().all(|s| s.at_s.is_finite() && s.heading_deg.is_none_or(f64::is_finite));
        if !finite_inputs
            || !self.initial.heading_deg.is_finite()
            || self.initial.position.iter().any(|v| !v.is_finite())
        {
            return fail("positions, headings and times must be finite");
        }
        if self.inputs.first().map(|s| s.at_s) != Some(0.0) {
            return fail("the first input segment must start at 0");
        }
        if self.inputs.windows(2).any(|w| w[1].at_s <= w[0].at_s) {
            return fail("input segments must be in strictly increasing time order");
        }
        let [a, b] = self.analysis.steady_window_s;
        let window_ok = non_negative(a) && b.is_finite() && a < b;
        if !window_ok || !positive(self.analysis.speed_half_window_s) || !positive(self.analysis.onset_threshold_m) {
            return fail("analysis parameters are out of range");
        }
        for m in &self.metrics {
            if !non_negative(m.tolerance()) {
                return fail("metric tolerances must be non-negative");
            }
            if let MetricSpec::TimeToFraction { fraction, .. } | MetricSpec::StopDistance { fraction, .. } = m
                && (*fraction <= 0.0 || *fraction >= 1.0 || fraction.is_nan())
            {
                return fail("metric fractions must be in (0, 1)");
            }
        }
        Ok(())
    }

    /// Intent at time `t`, resolving inherited stance and heading.
    pub fn intent_at(&self, t: f64) -> MoveIntent {
        let mut stance = self.initial.stance;
        let mut heading = self.initial.heading_deg;
        let mut current = None;
        for seg in self.inputs.iter().take_while(|s| s.at_s <= t + 1e-9) {
            stance = seg.stance.unwrap_or(stance);
            heading = seg.heading_deg.unwrap_or(heading);
            current = Some(seg);
        }
        match current {
            Some(seg) => {
                MoveIntent { forward: seg.forward, right: seg.right, pace: seg.pace, stance, heading_deg: heading }
            }
            None => MoveIntent::idle(stance, heading),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"
id = "movement.test"
system = "movement"
description = "d"
capture_procedure = "p"
duration_s = 4.0
[initial]
stance = "stand"
heading_deg = 90.0
[[input]]
at_s = 0.0
forward = 1
[[input]]
at_s = 2.0
stance = "crouch"
heading_deg = 180.0
pace = "walk"
[[input]]
at_s = 3.0
[analysis]
onset_threshold_m = 0.02
steady_window_s = [1.0, 2.0]
speed_half_window_s = 0.05
[[metric]]
kind = "steady_speed"
tolerance = 0.1
"#;

    #[test]
    fn intents_inherit_stance_and_heading() {
        let s = Scenario::parse(BASE).expect("valid");
        let i = s.intent_at(0.5);
        assert_eq!((i.forward, i.pace, i.stance, i.heading_deg), (1, Pace::Run, Stance::Stand, 90.0));
        let i = s.intent_at(2.0);
        assert_eq!((i.forward, i.pace, i.stance, i.heading_deg), (0, Pace::Walk, Stance::Crouch, 180.0));
        let i = s.intent_at(3.5);
        assert_eq!((i.forward, i.pace, i.stance, i.heading_deg), (0, Pace::Run, Stance::Crouch, 180.0));
    }

    #[test]
    fn invalid_scenarios_are_rejected() {
        for (from, to) in [
            ("at_s = 0.0\nforward = 1", "at_s = 0.5\nforward = 1"),
            ("at_s = 3.0", "at_s = 1.0"),
            ("steady_window_s = [1.0, 2.0]", "steady_window_s = [2.0, 1.0]"),
            ("tolerance = 0.1", "tolerance = -0.1"),
            ("id = \"movement.test\"", "id = \"bad id\""),
            ("kind = \"steady_speed\"", "kind = \"steady_speed\"\ntypo = 1"),
        ] {
            let text = BASE.replacen(from, to, 1);
            assert!(Scenario::parse(&text).is_err(), "accepted: {to}");
        }
    }
}
