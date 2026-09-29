//! Trace measurements.
//!
//! The same functions measure simulation and reference traces, so any bias
//! in a method (for example, onset detected a little after the true start of
//! motion) affects both sides equally. They work on irregularly sampled data
//! by interpolating horizontal displacement linearly between samples.
//!
//! Assumption: scenarios measured this way move in a straight line, so
//! straight-line displacement from the start equals distance travelled.

use crate::scenario::{Analysis, MetricSpec};
use crate::trace::Trace;

/// Step used when scanning interpolated data for threshold crossings.
const SCAN_STEP_S: f64 = 1.0 / 480.0;

/// Horizontal displacement from the first sample, over time.
#[derive(Debug, Clone)]
pub struct Displacement {
    points: Vec<(f64, f64)>,
}

impl Displacement {
    pub fn of(trace: &Trace) -> Option<Self> {
        let first = trace.samples.first()?;
        let points =
            trace.samples.iter().map(|s| (s.t, (s.pos[0] - first.pos[0]).hypot(s.pos[1] - first.pos[1]))).collect();
        Some(Self { points })
    }

    pub fn start(&self) -> f64 {
        self.points.first().map_or(0.0, |p| p.0)
    }

    pub fn end(&self) -> f64 {
        self.points.last().map_or(0.0, |p| p.0)
    }

    /// Displacement at `t` (clamped to the trace span).
    pub fn at(&self, t: f64) -> f64 {
        let pts = &self.points;
        match pts.partition_point(|p| p.0 <= t) {
            0 => pts.first().map_or(0.0, |p| p.1),
            i if i >= pts.len() => pts.last().map_or(0.0, |p| p.1),
            i => {
                let (t0, d0) = pts[i - 1];
                let (t1, d1) = pts[i];
                if t1 > t0 { d0 + (d1 - d0) * (t - t0) / (t1 - t0) } else { d1 }
            }
        }
    }

    /// Central-difference speed estimate at `t`.
    pub fn speed(&self, t: f64, half_window: f64) -> f64 {
        let (a, b) = ((t - half_window).max(self.start()), (t + half_window).min(self.end()));
        if b > a { (self.at(b) - self.at(a)) / (b - a) } else { 0.0 }
    }

    /// First time the displacement exceeds `threshold`, interpolated.
    pub fn onset(&self, threshold: f64) -> Option<f64> {
        let i = self.points.iter().position(|p| p.1 > threshold)?;
        if i == 0 {
            return Some(self.points[0].0);
        }
        let (t0, d0) = self.points[i - 1];
        let (t1, d1) = self.points[i];
        Some(t0 + (t1 - t0) * (threshold - d0) / (d1 - d0))
    }

    /// Least-squares slope of displacement over samples in `[a, b]`.
    pub fn slope(&self, a: f64, b: f64) -> Option<f64> {
        let pts: Vec<_> = self.points.iter().filter(|p| p.0 >= a && p.0 <= b).collect();
        if pts.len() < 3 {
            return None;
        }
        let n = pts.len() as f64;
        let mt = pts.iter().map(|p| p.0).sum::<f64>() / n;
        let md = pts.iter().map(|p| p.1).sum::<f64>() / n;
        let cov: f64 = pts.iter().map(|p| (p.0 - mt) * (p.1 - md)).sum();
        let var: f64 = pts.iter().map(|p| (p.0 - mt).powi(2)).sum();
        (var > 0.0).then(|| cov / var)
    }

    /// First `t` in `[from, end]` where `pred(speed(t))` holds.
    fn scan(&self, from: f64, half_window: f64, pred: impl Fn(f64) -> bool) -> Option<f64> {
        let mut t = from;
        while t <= self.end() {
            if pred(self.speed(t, half_window)) {
                return Some(t);
            }
            t += SCAN_STEP_S;
        }
        None
    }
}

/// Result of measuring one metric on one trace.
#[derive(Debug, Clone, PartialEq)]
pub enum Measured {
    Value(f64),
    /// The trace does not contain what the metric needs; the reason says why.
    Unavailable(String),
}

impl Measured {
    pub fn value(&self) -> Option<f64> {
        match self {
            Measured::Value(v) => Some(*v),
            Measured::Unavailable(_) => None,
        }
    }
}

/// Measure every metric of a scenario on a trace.
pub fn measure(trace: &Trace, analysis: &Analysis, metrics: &[MetricSpec]) -> Vec<Measured> {
    let unavailable = |why: &str| metrics.iter().map(|_| Measured::Unavailable(why.to_owned())).collect();
    let Some(d) = Displacement::of(trace) else {
        return unavailable("empty trace");
    };
    let Some(onset) = d.onset(analysis.onset_threshold_m) else {
        return unavailable("no motion onset in trace");
    };
    let [wa, wb] = analysis.steady_window_s;
    let steady = d.slope(onset + wa, onset + wb);
    let h = analysis.speed_half_window_s;
    metrics
        .iter()
        .map(|m| {
            let Some(v) = steady.filter(|v| *v > 0.0) else {
                return Measured::Unavailable("fewer than 3 samples, or no motion, in the steady window".into());
            };
            match m {
                MetricSpec::SteadySpeed { .. } => Measured::Value(v),
                MetricSpec::TimeToFraction { fraction, .. } => d
                    .scan(onset, h, |s| s >= fraction * v)
                    .map(|t| Measured::Value(t - onset))
                    .unwrap_or_else(|| Measured::Unavailable("speed never reached the fraction".into())),
                MetricSpec::StopDistance { fraction, .. } => d
                    .scan(onset + wb, h, |s| s < fraction * v)
                    .map(|t| Measured::Value(d.at(d.end()) - d.at(t)))
                    .unwrap_or_else(|| Measured::Unavailable("speed never dropped below the fraction".into())),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::Sample;

    /// Analytic motion: still until `t0`, constant acceleration `a` up to
    /// speed `v`, cruise until `t_stop`, constant deceleration `b` to rest.
    fn analytic(t: f64, t0: f64, a: f64, v: f64, t_stop: f64, b: f64) -> f64 {
        let ta = v / a;
        let dist_run = |t: f64| {
            let t = (t - t0).max(0.0);
            if t < ta { 0.5 * a * t * t } else { 0.5 * a * ta * ta + v * (t - ta) }
        };
        if t <= t_stop {
            dist_run(t)
        } else {
            let tb = (t - t_stop).min(v / b);
            dist_run(t_stop) + v * tb - 0.5 * b * tb * tb
        }
    }

    fn trace(times: impl Iterator<Item = f64>) -> Trace {
        Trace {
            meta: Default::default(),
            samples: times
                .map(|t| Sample {
                    t,
                    pos: [0.0, analytic(t, 1.0, 8.0, 4.0, 7.0, 16.0), 0.0],
                    heading_deg: 0.0,
                    speed_kmh: None,
                    anim: None,
                })
                .collect(),
        }
    }

    fn analysis() -> Analysis {
        Analysis { onset_threshold_m: 0.02, steady_window_s: [2.0, 5.0], speed_half_window_s: 0.02 }
    }

    fn metrics() -> Vec<MetricSpec> {
        vec![
            MetricSpec::SteadySpeed { tolerance: 0.0 },
            MetricSpec::TimeToFraction { fraction: 0.9, tolerance: 0.0 },
            MetricSpec::StopDistance { fraction: 0.9, tolerance: 0.0 },
        ]
    }

    fn check(tr: &Trace, tol: [f64; 3]) {
        let m: Vec<f64> = measure(tr, &analysis(), &metrics()).iter().map(|m| m.value().expect("measurable")).collect();
        // Onset is detected when displacement passes 0.02 m: t = sqrt(2*0.02/8) ≈ 0.0707 s after
        // true start, so time-to-90% (true 0.45 s) reads ≈ 0.379 s. Same bias on both sides.
        let onset_bias = (2.0f64 * 0.02 / 8.0).sqrt();
        let expected = [4.0, 0.45 - onset_bias, {
            // Speed falls below 3.6 m/s 0.025 s after release; remaining distance 3.6²/(2·16).
            3.6 * 3.6 / 32.0
        }];
        for i in 0..3 {
            assert!((m[i] - expected[i]).abs() <= tol[i], "metric {i}: {} vs {}", m[i], expected[i]);
        }
    }

    #[test]
    fn regular_sampling_recovers_analytic_values() {
        check(&trace((0..=600).map(|i| f64::from(i) / 60.0)), [1e-9, 0.02, 0.02]);
    }

    #[test]
    fn irregular_sampling_is_handled() {
        // Frame times jitter between ~10 ms and ~40 ms, like a real capture.
        let mut t = 0.0;
        let mut times = Vec::new();
        let mut k = 0u32;
        while t <= 10.0 {
            times.push(t);
            k = k.wrapping_mul(1_103_515_245).wrapping_add(12345);
            t += 0.010 + f64::from((k >> 16) % 30) / 1000.0;
        }
        check(&trace(times.into_iter()), [1e-9, 0.05, 0.05]);
    }

    #[test]
    fn unmeasurable_traces_say_why() {
        let still = Trace {
            meta: Default::default(),
            samples: (0..10)
                .map(|i| Sample {
                    t: f64::from(i),
                    pos: [5.0, 5.0, 0.0],
                    heading_deg: 0.0,
                    speed_kmh: None,
                    anim: None,
                })
                .collect(),
        };
        let m = measure(&still, &analysis(), &metrics());
        assert!(m.iter().all(|m| matches!(m, Measured::Unavailable(r) if r.contains("onset"))));
        assert!(measure(&Trace::default(), &analysis(), &metrics()).iter().all(|m| m.value().is_none()));
    }
}
