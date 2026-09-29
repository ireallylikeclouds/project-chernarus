//! The full measure → compare loop.
//!
//! The "reference" captures in these tests are SYNTHETIC: produced by the
//! simulation itself with known parameters and written in the RPT format of
//! the capture script. They test the harness and say nothing about the game.

use chernarus_core::Verification;
use chernarus_parity::compare::{Verdict, compare, load_reference, simulate};
use chernarus_parity::rpt;
use chernarus_parity::scenario::Scenario;
use chernarus_parity::trace::Trace;
use chernarus_sim::movement::{Direction, MovementParams, Pace, Stance};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn params() -> MovementParams {
    MovementParams::load(&repo().join("data/movement/infantry.toml")).expect("movement data")
}

fn scenario(name: &str) -> Scenario {
    Scenario::load(&repo().join("tests/parity/scenarios").join(name)).expect("scenario")
}

/// Render a trace as the capture script would log it: irregular frame
/// times, positions relative to the start, run id, BEGIN/END lines.
fn as_rpt_log(trace: &Trace, scenario: &str, run: &str, world_origin: [f64; 3]) -> String {
    let mut log = String::from("== synthetic RPT header\nnoise line\n");
    let _ = writeln!(log, "CHERNARUS_CAPTURE|1|{scenario}|{run}|BEGIN|[\"synthetic\"]|flatland|SynthUnit");
    let start = trace.samples[0].pos;
    let mut k: u32 = run.bytes().map(u32::from).sum();
    for (i, s) in trace.samples.iter().enumerate() {
        k = k.wrapping_mul(1_103_515_245).wrapping_add(12345);
        if !i.is_multiple_of(4) && !(k >> 16).is_multiple_of(3) {
            continue; // drop frames irregularly: ~15-60 ms between samples
        }
        // Positions pass through world coordinates and back, like getPosASL minus start.
        let world = [s.pos[0] + world_origin[0], s.pos[1] + world_origin[1], s.pos[2] + world_origin[2]];
        let rel = [world[0] - (start[0] + world_origin[0]), world[1] - (start[1] + world_origin[1]), 0.0];
        let _ = writeln!(
            log,
            " 0:00:{i:02} CHERNARUS_CAPTURE|1|{scenario}|{run}|SAMPLE|{:.4}|{:.4}|{:.4}|{:.4}|{:.2}|{:.3}|synthanim",
            s.t,
            rel[0],
            rel[1],
            rel[2],
            s.heading_deg,
            s.speed_kmh.unwrap_or(0.0)
        );
    }
    let _ = writeln!(log, "CHERNARUS_CAPTURE|1|{scenario}|{run}|END");
    log
}

#[test]
fn committed_scenarios_run_and_report_no_reference() {
    let dir = repo().join("tests/parity/scenarios");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .expect("scenario dir")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 3);
    let params = Arc::new(params());
    for path in paths {
        let s = Scenario::load(&path).expect("valid scenario");
        let (trace, used) = simulate(&s, params.clone());
        let reference = load_reference(&repo().join("tests/parity/reference"), &s).expect("reference dir readable");
        let report = compare(&s, &trace, used, &reference);
        assert!(
            report.metrics.iter().all(|m| m.sim.is_some()),
            "{}: simulation must be measurable: {:#?}",
            s.id,
            report.metrics
        );
        if reference.is_empty() {
            assert_eq!(report.verdict, Verdict::NoReference, "{}", s.id);
            assert_eq!(report.reference_status, Verification::Unknown);
        }
        // Placeholder parameters can never produce a better-than-UNKNOWN input status.
        assert_eq!(report.sim_inputs_status, Verification::Unknown);
        assert!(report.to_markdown().contains(&s.id));
    }
}

#[test]
fn simulation_matches_its_own_parameters() {
    let s = scenario("movement_stand_run_forward.toml");
    let p = params();
    let expected = p.speed(Stance::Stand, Pace::Run, Direction::Forward).value;
    let (trace, _) = simulate(&s, Arc::new(p));
    let report = compare(&s, &trace, Vec::new(), &[]);
    let steady = report.metrics[0].sim.expect("measured");
    assert!((steady - expected).abs() < 1e-9, "{steady} vs {expected}");
}

#[test]
fn synthetic_reference_passes_then_detects_a_difference() {
    let s = scenario("movement_stand_run_forward.toml");
    let origin = [4512.25, 10233.5, 12.0];

    // "Reference" produced with the shipped parameters, captured three times.
    let (ref_trace, _) = simulate(&s, Arc::new(params()));
    let log: String = ["a1", "b22", "c333"].iter().map(|run| as_rpt_log(&ref_trace, &s.id, run, origin)).collect();
    let import = rpt::import(&log);
    assert!(import.rejected.is_empty(), "{:?}", import.rejected);
    assert_eq!(import.runs.len(), 3);
    assert!(import.runs.iter().all(|r| r.complete));

    // Write and reload through the committed on-disk layout.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(&s.id)).expect("mkdir");
    for run in &import.runs {
        let name = format!("{}.csv", run.trace.meta("run").expect("run id"));
        run.trace.save(&dir.path().join(&s.id).join(name)).expect("save");
    }
    let reference = load_reference(dir.path(), &s).expect("load");
    assert_eq!(reference.len(), 3);

    // Same parameters: every metric within tolerance.
    let (sim, used) = simulate(&s, Arc::new(params()));
    let report = compare(&s, &sim, used, &reference);
    assert_eq!(report.verdict, Verdict::Pass, "{}", report.to_markdown());
    assert_eq!(report.reference_status, Verification::Verified);
    assert_eq!(report.reference_runs, 3);

    // One reference run only: reference behaviour is PARTIALLY VERIFIED.
    let report_one = compare(&s, &sim, Vec::new(), &reference[..1]);
    assert_eq!(report_one.reference_status, Verification::PartiallyVerified);

    // Simulation 0.3 m/s too fast: steady speed fails with the right sign and size.
    let mut fast = params();
    fast.speed_mps.stand.run.forward.value += 0.3;
    let (sim_fast, used) = simulate(&s, Arc::new(fast));
    let report = compare(&s, &sim_fast, used, &reference);
    assert_eq!(report.verdict, Verdict::Fail);
    let diff = report.metrics[0].difference.expect("compared");
    assert!((diff - 0.3).abs() < 0.01, "difference {diff}");
}

#[test]
fn reference_for_another_scenario_is_rejected() {
    let s = scenario("movement_stand_run_forward.toml");
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(&s.id)).expect("mkdir");
    let mut t = Trace::default();
    t.meta.insert("scenario".into(), "movement.other".into());
    t.save(&dir.path().join(&s.id).join("x.csv")).expect("save");
    assert!(load_reference(dir.path(), &s).is_err());
}
