//! Position/heading traces and their CSV form.
//!
//! CSV layout: `# key: value` metadata lines, a header line, then one sample
//! per line: `t,x,y,z,heading_deg,speed_kmh,anim` (the last two may be empty).
//! Positions follow the reference convention (x east, y north, z up, metres);
//! reference captures store positions relative to the capture start.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use thiserror::Error;

const HEADER: &str = "t,x,y,z,heading_deg,speed_kmh,anim";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// Seconds since the start of the trace.
    pub t: f64,
    pub pos: [f64; 3],
    pub heading_deg: f64,
    /// `speed player` in the reference (km/h); derived in the simulation.
    pub speed_kmh: Option<f64>,
    /// `animationState player` in the reference.
    pub anim: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Trace {
    /// Metadata: `scenario`, `source` (`simulation` or `reference`), `run`,
    /// `game_version`, `world`, `unit_class`, ...
    pub meta: BTreeMap<String, String>,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Error)]
pub enum TraceError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("line {line}: {detail}")]
    Parse { line: usize, detail: String },
}

impl Trace {
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.meta.get(key).map(String::as_str)
    }

    pub fn to_csv(&self) -> String {
        let mut out = String::new();
        for (k, v) in &self.meta {
            let _ = writeln!(out, "# {k}: {}", v.replace('\n', " "));
        }
        out.push_str(HEADER);
        out.push('\n');
        for s in &self.samples {
            let speed = s.speed_kmh.map(|v| v.to_string()).unwrap_or_default();
            let anim = s.anim.as_deref().unwrap_or("").replace([',', '\n'], " ");
            let _ = writeln!(out, "{},{},{},{},{},{speed},{anim}", s.t, s.pos[0], s.pos[1], s.pos[2], s.heading_deg);
        }
        out
    }

    pub fn from_csv(text: &str) -> Result<Self, TraceError> {
        let mut trace = Trace::default();
        let mut seen_header = false;
        for (i, line) in text.lines().enumerate() {
            let line_no = i + 1;
            let line = line.trim_end_matches('\r');
            if let Some(meta) = line.strip_prefix('#') {
                if let Some((k, v)) = meta.split_once(':') {
                    trace.meta.insert(k.trim().to_owned(), v.trim().to_owned());
                }
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            if !seen_header {
                if line.trim() != HEADER {
                    return Err(TraceError::Parse { line: line_no, detail: format!("expected header `{HEADER}`") });
                }
                seen_header = true;
                continue;
            }
            let fields: Vec<&str> = line.splitn(7, ',').collect();
            if fields.len() < 5 {
                return Err(TraceError::Parse { line: line_no, detail: "expected at least 5 fields".into() });
            }
            let num = |idx: usize| -> Result<f64, TraceError> {
                fields[idx]
                    .trim()
                    .parse::<f64>()
                    .map_err(|e| TraceError::Parse { line: line_no, detail: format!("field {}: {e}", idx + 1) })
            };
            let speed_kmh = match fields.get(5).map(|s| s.trim()) {
                None | Some("") => None,
                Some(_) => Some(num(5)?),
            };
            let anim = fields.get(6).map(|s| s.trim()).filter(|s| !s.is_empty()).map(str::to_owned);
            trace.samples.push(Sample {
                t: num(0)?,
                pos: [num(1)?, num(2)?, num(3)?],
                heading_deg: num(4)?,
                speed_kmh,
                anim,
            });
        }
        if !seen_header {
            return Err(TraceError::Parse { line: 0, detail: "no header line".into() });
        }
        trace.validate()?;
        Ok(trace)
    }

    /// Samples must have finite values and non-decreasing time.
    pub fn validate(&self) -> Result<(), TraceError> {
        let mut last = f64::NEG_INFINITY;
        for (i, s) in self.samples.iter().enumerate() {
            let finite = s.t.is_finite() && s.pos.iter().all(|v| v.is_finite()) && s.heading_deg.is_finite();
            if !finite || s.t < last {
                return Err(TraceError::Parse {
                    line: i + 1,
                    detail: "sample times must be finite and non-decreasing".into(),
                });
            }
            last = s.t;
        }
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, TraceError> {
        Self::from_csv(&std::fs::read_to_string(path)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), TraceError> {
        Ok(std::fs::write(path, self.to_csv())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_round_trip() {
        let mut t = Trace::default();
        t.meta.insert("scenario".into(), "movement.x".into());
        t.samples.push(Sample { t: 0.0, pos: [0.0, 0.0, 0.0], heading_deg: 0.0, speed_kmh: None, anim: None });
        t.samples.push(Sample {
            t: 0.016_666_666_666_666_666,
            pos: [0.1, -2.5, 1e-7],
            heading_deg: 359.5,
            speed_kmh: Some(15.25),
            anim: Some("amovpercmrunsnonwnondf".into()),
        });
        assert_eq!(Trace::from_csv(&t.to_csv()).expect("parses"), t);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Trace::from_csv("").is_err());
        assert!(Trace::from_csv("wrong,header\n").is_err());
        assert!(Trace::from_csv(&format!("{HEADER}\n1,0,0\n")).is_err());
        assert!(Trace::from_csv(&format!("{HEADER}\n1,0,0,0,0,,\n0.5,0,0,0,0,,\n")).is_err());
        assert!(Trace::from_csv(&format!("{HEADER}\nnan,0,0,0,0,,\n")).is_err());
    }
}
