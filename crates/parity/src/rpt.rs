//! Import of reference captures from an ARMA 2 RPT log.
//!
//! `tools/capture/chernarus_capture.sqf` writes lines through `diag_log`:
//!
//! ```text
//! CHERNARUS_CAPTURE|1|<scenario>|<run>|BEGIN|<productVersion>|<worldName>|<typeOf player>
//! CHERNARUS_CAPTURE|1|<scenario>|<run>|SAMPLE|t|dx|dy|dz|dir|speed_kmh|animationState
//! CHERNARUS_CAPTURE|1|<scenario>|<run>|END
//! ```
//!
//! `dx dy dz` are metres relative to the capture's start position: SQF
//! formats numbers with about six significant digits, which would cost
//! ~0.1 m at Chernarus coordinates but keeps millimetres for offsets.
//! Anything before the marker (timestamps, quotes) is ignored, so the exact
//! RPT line decoration does not matter. Status of the whole capture path:
//! UNTESTED against the reference game.

use crate::trace::{Sample, Trace};
use std::collections::BTreeMap;

pub const MARKER: &str = "CHERNARUS_CAPTURE|";
pub const PROTOCOL: &str = "1";

/// One capture run found in a log.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedRun {
    pub trace: Trace,
    /// An END line was seen.
    pub complete: bool,
}

/// Lines that looked like capture output but could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct Import {
    pub runs: Vec<ImportedRun>,
    pub rejected: Vec<Rejected>,
}

/// Parse all capture runs in an RPT log. Runs come out ordered by
/// (scenario, run id).
pub fn import(log: &str) -> Import {
    let mut runs: BTreeMap<(String, String), ImportedRun> = BTreeMap::new();
    let mut rejected = Vec::new();
    for (i, raw) in log.lines().enumerate() {
        let Some(at) = raw.find(MARKER) else { continue };
        let line = raw[at + MARKER.len()..].trim_end().trim_end_matches('"');
        let line_no = i + 1;
        let mut reject = |reason: &str| rejected.push(Rejected { line: line_no, reason: reason.to_owned() });
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() < 4 {
            reject("too few fields");
            continue;
        }
        if fields[0] != PROTOCOL {
            reject("unsupported capture protocol version");
            continue;
        }
        let key = (fields[1].to_owned(), fields[2].to_owned());
        let run = runs.entry(key.clone()).or_insert_with(|| {
            let mut trace = Trace::default();
            trace.meta.insert("scenario".into(), key.0.clone());
            trace.meta.insert("run".into(), key.1.clone());
            trace.meta.insert("source".into(), "reference".into());
            ImportedRun { trace, complete: false }
        });
        match fields[3] {
            "BEGIN" => {
                for (name, idx) in [("game_version", 4), ("world", 5), ("unit_class", 6)] {
                    if let Some(v) = fields.get(idx) {
                        run.trace.meta.insert(name.into(), (*v).to_owned());
                    }
                }
            }
            "SAMPLE" => {
                if fields.len() < 11 {
                    reject("SAMPLE needs t, dx, dy, dz, dir, speed");
                    continue;
                }
                let nums: Result<Vec<f64>, _> = fields[4..10].iter().map(|f| f.trim().parse::<f64>()).collect();
                let Ok(n) = nums else {
                    reject("SAMPLE has a non-numeric field");
                    continue;
                };
                if n.iter().any(|v| !v.is_finite()) {
                    reject("SAMPLE has a non-finite value");
                    continue;
                }
                if run.trace.samples.last().is_some_and(|s| n[0] < s.t) {
                    reject("SAMPLE time goes backwards");
                    continue;
                }
                run.trace.samples.push(Sample {
                    t: n[0],
                    pos: [n[1], n[2], n[3]],
                    heading_deg: n[4],
                    speed_kmh: Some(n[5]),
                    anim: fields.get(10).map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()),
                });
            }
            "END" => run.complete = true,
            _ => reject("unknown record type"),
        }
    }
    Import { runs: runs.into_values().collect(), rejected }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = r#"=====================================================================
== C:\Games\ArmA2OA.exe
Exe timestamp: 2013/01/01
unrelated engine noise
CHERNARUS_CAPTURE|1|movement.stand_run_forward|42|BEGIN|["ArmA 2 OA","ArmA2OA",163,112555]|chernarus|Survivor2_DZ
CHERNARUS_CAPTURE|1|movement.stand_run_forward|42|SAMPLE|0|0|0|0|0|0|amovpercmstpsraswrfldnon
"CHERNARUS_CAPTURE|1|movement.stand_run_forward|42|SAMPLE|0.0166|0|0.001|0|0.5|0.2|amovpercmrunsraswrfldf"
 1:02:03 CHERNARUS_CAPTURE|1|movement.stand_run_forward|42|SAMPLE|1.2e+000|0|2.5|0|0.5|15.3|amovpercmrunsraswrfldf
CHERNARUS_CAPTURE|1|movement.stand_run_forward|42|SAMPLE|1.1|0|2.4|0|0.5|15.3|out-of-order
CHERNARUS_CAPTURE|1|movement.stand_run_forward|42|SAMPLE|x|0|0|0|0|0|bad
CHERNARUS_CAPTURE|2|movement.stand_run_forward|42|SAMPLE|0|0|0|0|0|0|future
CHERNARUS_CAPTURE|1|movement.stand_run_forward|42|END
CHERNARUS_CAPTURE|1|movement.stand_run_forward|7|SAMPLE|0|0|0|0|0|0|
"#;

    #[test]
    fn windows_line_endings_are_handled() {
        // RPT logs come from a Windows program, also when it runs under Wine/Proton.
        let crlf = LOG.replace('\n', "\r\n");
        let (lf, crlf) = (import(LOG), import(&crlf));
        assert_eq!(lf.runs, crlf.runs);
        assert_eq!(lf.rejected, crlf.rejected);
        assert_eq!(crlf.runs[0].trace.samples[1].anim.as_deref(), Some("amovpercmrunsraswrfldf"));
    }

    #[test]
    fn imports_runs_and_rejects_bad_lines() {
        let import = import(LOG);
        assert_eq!(import.runs.len(), 2);
        let run = &import.runs[0];
        assert_eq!(run.trace.meta("run"), Some("42"));
        assert!(run.complete);
        assert_eq!(run.trace.meta("world"), Some("chernarus"));
        assert_eq!(run.trace.meta("unit_class"), Some("Survivor2_DZ"));
        assert_eq!(run.trace.samples.len(), 3);
        assert_eq!(run.trace.samples[2].t, 1.2);
        assert_eq!(run.trace.samples[2].pos, [0.0, 2.5, 0.0]);
        assert_eq!(run.trace.samples[1].anim.as_deref(), Some("amovpercmrunsraswrfldf"));

        let incomplete = &import.runs[1];
        assert_eq!(incomplete.trace.meta("run"), Some("7"));
        assert!(!incomplete.complete);
        assert_eq!(incomplete.trace.samples[0].anim, None);

        let reasons: Vec<_> = import.rejected.iter().map(|r| r.reason.as_str()).collect();
        assert_eq!(
            reasons,
            ["SAMPLE time goes backwards", "SAMPLE has a non-numeric field", "unsupported capture protocol version"]
        );
    }
}
