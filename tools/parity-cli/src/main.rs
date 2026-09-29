//! `parity` — run parity scenarios against reference captures.

use anyhow::{Context, Result, bail};
use chernarus_core::Verification;
use chernarus_parity::compare::{Verdict, compare, load_reference, simulate};
use chernarus_parity::rpt;
use chernarus_parity::scenario::Scenario;
use chernarus_sim::movement::MovementParams;
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "parity", version, about = "Parity scenarios: simulate, measure, compare with reference captures")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run scenarios and compare with reference traces.
    Run {
        /// Scenario files (default: every .toml in --scenarios-dir).
        scenarios: Vec<PathBuf>,
        #[arg(long, default_value = "tests/parity/scenarios")]
        scenarios_dir: PathBuf,
        #[arg(long, default_value = "tests/parity/reference")]
        reference_dir: PathBuf,
        #[arg(long, default_value = "data/movement/infantry.toml")]
        params: PathBuf,
        /// Write JSON reports and simulation traces here.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Exit non-zero if any scenario with reference data fails.
        #[arg(long)]
        strict: bool,
    },
    /// Import capture runs from an ARMA 2 RPT log into reference traces.
    ImportRpt {
        rpt: PathBuf,
        #[arg(long, default_value = "tests/parity/reference")]
        out: PathBuf,
        /// Replace existing trace files.
        #[arg(long)]
        force: bool,
        /// Metadata stamped on every imported trace, e.g. `--meta platform=proton-9 --meta fps=60`.
        /// Record at least `platform` (windows or proton/wine version) and `fps`.
        #[arg(long = "meta", value_name = "KEY=VALUE", value_parser = parse_meta)]
        meta: Vec<(String, String)>,
    },
    /// List movement parameters and their knowledge status.
    Params {
        #[arg(long, default_value = "data/movement/infantry.toml")]
        params: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) if is_broken_pipe(&e) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn parse_meta(arg: &str) -> Result<(String, String), String> {
    let (k, v) = arg.split_once('=').ok_or_else(|| format!("expected KEY=VALUE, got `{arg}`"))?;
    Ok((k.trim().to_owned(), v.to_owned()))
}

fn scenario_files(explicit: Vec<PathBuf>, dir: &Path) -> Result<Vec<PathBuf>> {
    if !explicit.is_empty() {
        return Ok(explicit);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    Ok(files)
}

/// `parity … | head` closes stdout early; that is normal use, not an error.
fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.chain().any(|c| c.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe))
}

fn run(cli: Cli) -> Result<ExitCode> {
    let stdout = std::io::stdout();
    let out = &mut stdout.lock();
    match cli.command {
        Command::Run { scenarios, scenarios_dir, reference_dir, params, out: out_dir, strict } => {
            let params = Arc::new(MovementParams::load(&params)?);
            let mut failed = 0;
            if let Some(dir) = &out_dir {
                std::fs::create_dir_all(dir)?;
            }
            writeln!(out, "# Parity run\n")?;
            for path in scenario_files(scenarios, &scenarios_dir)? {
                let scenario = Scenario::load(&path)?;
                let reference = load_reference(&reference_dir, &scenario).map_err(anyhow::Error::msg)?;
                let (trace, used) = simulate(&scenario, params.clone());
                let report = compare(&scenario, &trace, used, &reference);
                writeln!(out, "{}", report.to_markdown())?;
                if report.verdict == Verdict::Fail || report.verdict == Verdict::SimUnmeasurable {
                    failed += 1;
                }
                if let Some(dir) = &out_dir {
                    std::fs::write(
                        dir.join(format!("{}.report.json", scenario.id)),
                        serde_json::to_vec_pretty(&report)?,
                    )?;
                    trace.save(&dir.join(format!("{}.sim.csv", scenario.id)))?;
                }
            }
            if strict && failed > 0 {
                bail!("{failed} scenario(s) failed");
            }
        }
        Command::ImportRpt { rpt: path, out: out_dir, force, meta } => {
            let text = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
            let import = rpt::import(&text);
            for r in &import.rejected {
                eprintln!("line {}: rejected: {}", r.line, r.reason);
            }
            if import.runs.is_empty() {
                bail!("no CHERNARUS_CAPTURE runs found in {}", path.display());
            }
            let mut runs = import.runs;
            for run in &mut runs {
                for (k, v) in &meta {
                    run.trace.add_user_meta(k, v).map_err(anyhow::Error::msg)?;
                }
            }
            for run in &runs {
                let scenario = run.trace.meta("scenario").unwrap_or("unknown");
                let run_id = run.trace.meta("run").unwrap_or("unknown");
                if !run.complete {
                    eprintln!("{scenario} run {run_id}: no END line (capture interrupted?) — skipped");
                    continue;
                }
                let dir = out_dir.join(scenario);
                std::fs::create_dir_all(&dir)?;
                let file = dir.join(format!("{run_id}.csv"));
                if file.exists() && !force {
                    bail!("{} exists (use --force to replace)", file.display());
                }
                run.trace.save(&file)?;
                writeln!(out, "{scenario} run {run_id}: {} samples -> {}", run.trace.samples.len(), file.display())?;
            }
        }
        Command::Params { params } => {
            let p = MovementParams::load(&params)?;
            let all = p.all();
            let mut counts: BTreeMap<Verification, usize> = BTreeMap::new();
            for (key, param) in &all {
                *counts.entry(param.status).or_default() += 1;
                writeln!(out, "{key:<44} {:>7.3}  {:<18} {}", param.value, param.status.label(), param.source)?;
            }
            writeln!(out)?;
            for (status, n) in counts {
                writeln!(out, "{:<18} {n}", status.label())?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
