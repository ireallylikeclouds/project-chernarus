//! `refscan` — inspect a reference installation and enforce the asset boundary.
//!
//! Everything this tool reads from a reference installation stays local: it
//! prints to the terminal or writes catalogs to a path you choose (use the
//! git-ignored `reference-data/`). See docs/assets/discovery-pipeline.md.

use anyhow::{Context, Result, bail};
use chernarus_assets::catalog::Catalog;
use chernarus_assets::discovery::{ScanOptions, scan};
use chernarus_assets::provenance::{ProvenanceManifest, check_assets_dir};
use chernarus_assets::{fixture, guard, query};
use chernarus_formats::identify::{SNIFF_LEN, identify};
use chernarus_formats::pbo::PboHeader;
use chernarus_formats::{Format, rap};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "refscan", version, about = "Reference-installation discovery and provenance enforcement")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan an installation and write a catalog (metadata only).
    Scan {
        /// Installation root (ARMA 2 OA folder with mod folders).
        #[arg(env = "CHERNARUS_REFERENCE_DIR")]
        install: PathBuf,
        /// Catalog output path (keep it under the git-ignored reference-data/).
        #[arg(long, default_value = "reference-data/catalog.json")]
        out: PathBuf,
        /// Skip SHA-256 hashing.
        #[arg(long)]
        no_hash: bool,
        /// Skip dependency extraction.
        #[arg(long)]
        no_deps: bool,
        /// Skip PBO checksum verification.
        #[arg(long)]
        no_verify: bool,
        /// Largest entry (MiB) to hash or scan for dependencies.
        #[arg(long, default_value_t = 256)]
        max_content_mib: u64,
    },
    /// Summarise a catalog.
    Summary { catalog: PathBuf },
    /// Describe one asset: format, origin, dependencies, dependents, project equivalent.
    Query {
        catalog: PathBuf,
        /// Asset id or virtual path (e.g. `dz\weapons\data\x.p3d`).
        asset: String,
        /// Provenance manifest used for project equivalents.
        #[arg(long, default_value = "assets/provenance.toml")]
        manifest: PathBuf,
    },
    /// Identify files by signature and extension.
    Identify { files: Vec<PathBuf> },
    /// List the entries of a PBO.
    Pbo {
        file: PathBuf,
        /// Verify the trailing SHA-1.
        #[arg(long)]
        verify: bool,
    },
    /// Print a rapified config as text (a .bin file, or an entry inside a PBO).
    Config {
        file: PathBuf,
        /// Entry name inside the PBO (e.g. `config.bin`).
        #[arg(long)]
        entry: Option<String>,
    },
    /// Check this repository: no proprietary formats, clean provenance manifest.
    Provenance {
        /// Repository root.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Write the PROJECT-CREATED synthetic installation (for demos and tests).
    SynthInstall { dir: PathBuf },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Command::Scan { install, out, no_hash, no_deps, no_verify, max_content_mib } => {
            let options = ScanOptions {
                hash: !no_hash,
                dependencies: !no_deps,
                verify_pbo_checksums: !no_verify,
                max_content_bytes: max_content_mib * 1024 * 1024,
            };
            let catalog = scan(&install, &options).with_context(|| format!("scanning {}", install.display()))?;
            if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            catalog.save(&out).with_context(|| format!("writing {}", out.display()))?;
            print_summary(&catalog);
            println!("\ncatalog written to {} (local reference metadata — do not commit)", out.display());
        }
        Command::Summary { catalog } => print_summary(&Catalog::load(&catalog)?),
        Command::Query { catalog, asset, manifest } => {
            let catalog = Catalog::load(&catalog)?;
            let manifest = if manifest.exists() { Some(ProvenanceManifest::load(&manifest)?) } else { None };
            let index = catalog.index();
            let hits = query::find(&index, &asset);
            if hits.is_empty() {
                bail!("no asset matches {asset}");
            }
            if hits.len() > 1 {
                println!("{} assets provide this path (engine choice depends on load order — UNKNOWN):\n", hits.len());
            }
            for hit in hits {
                println!("{}", query::describe(&catalog, &index, hit, manifest.as_ref()));
            }
        }
        Command::Identify { files } => {
            for f in files {
                let mut head = Vec::new();
                File::open(&f)?.take(SNIFF_LEN as u64).read_to_end(&mut head)?;
                let id = identify(&f.to_string_lossy(), &head);
                let conflict = if id.conflict { "  CONFLICT (signature vs extension)" } else { "" };
                println!("{}: {:?} by {:?}{conflict}", f.display(), id.format, id.basis);
            }
        }
        Command::Pbo { file, verify } => {
            let len = std::fs::metadata(&file)?.len();
            let header = PboHeader::read(BufReader::new(File::open(&file)?), len)?;
            let mut reader = BufReader::new(File::open(&file)?);
            for (k, v) in &header.extensions {
                println!("{k} = {v}");
            }
            println!("{} entries", header.entries.len());
            for e in &header.entries {
                let head = header.read_prefix(&mut reader, e, SNIFF_LEN).unwrap_or_default();
                let format = identify(&e.name, &head).format;
                let (packing, format) = (format!("{:?}", e.packing), format!("{format:?}"));
                println!("{:>12}  {packing:<13} {format:<14} {}", e.content_size(), e.name);
            }
            if verify {
                println!("checksum: {:?}", header.verify_checksum(&mut reader)?);
            }
        }
        Command::Config { file, entry } => {
            let data = match entry {
                None => std::fs::read(&file)?,
                Some(name) => read_pbo_entry(&file, &name)?,
            };
            print!("{}", rap::to_text(&rap::read(&data)?));
        }
        Command::Provenance { repo } => return provenance(&repo),
        Command::SynthInstall { dir } => {
            fixture::write_synthetic_installation(&dir)?;
            println!("synthetic installation written to {} (PROJECT-CREATED, invented content)", dir.display());
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn read_pbo_entry(file: &Path, name: &str) -> Result<Vec<u8>> {
    let len = std::fs::metadata(file)?.len();
    let header = PboHeader::read(BufReader::new(File::open(file)?), len)?;
    let entry = header
        .entries
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case(name))
        .with_context(|| format!("no entry {name} in {}", file.display()))?;
    Ok(header.read_entry(&mut BufReader::new(File::open(file)?), entry)?.data)
}

fn print_summary(catalog: &Catalog) {
    let pbos = catalog.assets.iter().filter(|a| a.format == Format::Pbo).count();
    let entries = catalog.assets.iter().filter(|a| a.virtual_path.is_some()).count();
    let resolved = catalog.edges.iter().filter(|e| e.resolved()).count();
    let ambiguous = catalog.edges.iter().filter(|e| e.providers.len() > 1).count();
    let conflicts = catalog.assets.iter().filter(|a| a.format_conflict).count();
    println!("root: {}", catalog.root);
    println!(
        "assets: {} ({} files, {pbos} PBOs, {entries} PBO entries)",
        catalog.assets.len(),
        catalog.assets.len() - entries
    );
    println!("formats:");
    for (format, count) in catalog.format_counts() {
        println!("  {count:>8}  {format:?}");
    }
    println!(
        "dependency edges: {} ({resolved} resolved, {} unresolved, {ambiguous} with several providers)",
        catalog.edges.len(),
        catalog.edges.len() - resolved
    );
    println!("signature/extension conflicts: {conflicts}");
    println!("scan issues: {}", catalog.issues.len());
    for issue in catalog.issues.iter().take(20) {
        println!("  {}: {}", issue.asset, issue.message);
    }
    if catalog.issues.len() > 20 {
        println!("  ... {} more", catalog.issues.len() - 20);
    }
}

fn provenance(repo: &Path) -> Result<ExitCode> {
    let findings = guard::scan_repository(repo)?;
    let issues = check_assets_dir(&repo.join(guard::ASSETS_DIR))?;
    for f in &findings {
        println!("FORBIDDEN  {} — {:?} ({:?})", f.path, f.format, f.reason);
    }
    for i in &issues {
        println!("PROVENANCE {i}");
    }
    if findings.is_empty() && issues.is_empty() {
        println!("provenance check passed: no proprietary formats, assets/ manifest clean");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("provenance check FAILED ({} forbidden file(s), {} manifest issue(s))", findings.len(), issues.len());
        Ok(ExitCode::FAILURE)
    }
}
