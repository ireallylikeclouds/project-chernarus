//! Asset discovery over a reference installation.
//!
//! Pipeline (see `docs/assets/discovery-pipeline.md`):
//! 1. walk the installation (sorted; symlinks followed, because Linux setups
//!    commonly symlink mod folders into the game directory; loops are
//!    reported as scan issues);
//! 2. identify each file by signature and extension;
//! 3. open PBOs and identify each entry, mounting it at `<prefix>\<name>`;
//! 4. extract dependency references from configs, text and (heuristically)
//!    binary formats not parsed yet;
//! 5. resolve references against the mounted virtual file system.
//!
//! The scanner only reads. Everything it records is metadata, and every
//! record is classed REFERENCE / not redistributable.

use crate::catalog::{
    AssetRecord, Catalog, Edge, EdgeMethod, Location, PboDetails, PrefixSource, SCHEMA_VERSION, ScanIssue,
};
use crate::provenance::{ProvenanceClass, relative_slash_path, sha256_bytes, sha256_file};
use crate::vfs;
use chernarus_formats::identify::{SNIFF_LEN, identify};
use chernarus_formats::pbo::PboHeader;
use chernarus_formats::{Format, Identification, rap, strings};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use walkdir::WalkDir;

/// How many leading bytes a binary string scan may discard when looking for
/// a resolvable path (printable bytes before a string get glued onto it).
const MAX_BINARY_TRIM: usize = 32;

/// Extensions the engine tries for extension-less paths in configs, by the
/// config key that holds them. ESTIMATED; used only when the resulting path
/// exists in the scanned installation.
const IMPLICIT_EXTENSIONS: &[&str] = &["p3d", "wss", "ogg", "wav", "paa", "sqf"];

/// What the scanner does beyond identification.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// SHA-256 of every file and entry.
    pub hash: bool,
    /// Extract and resolve dependency references.
    pub dependencies: bool,
    /// Verify each PBO's trailing SHA-1 (reads every archive completely).
    pub verify_pbo_checksums: bool,
    /// Entries larger than this are identified but not hashed or scanned
    /// for dependencies.
    pub max_content_bytes: u64,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self { hash: true, dependencies: true, verify_pbo_checksums: true, max_content_bytes: 256 * 1024 * 1024 }
    }
}

/// An unresolved reference found while reading content.
struct PendingRef {
    from: String,
    from_dir: String,
    raw: String,
    method: EdgeMethod,
    context: Option<String>,
}

#[derive(Default)]
struct Scan {
    assets: Vec<AssetRecord>,
    pending: Vec<PendingRef>,
    issues: Vec<ScanIssue>,
}

impl Scan {
    fn issue(&mut self, asset: &str, message: impl Into<String>) {
        self.issues.push(ScanIssue { asset: asset.to_owned(), message: message.into() });
    }
}

/// Scan an installation directory.
pub fn scan(root: &Path, options: &ScanOptions) -> std::io::Result<Catalog> {
    if !root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} is not a directory", root.display()),
        ));
    }
    let mut scan = Scan::default();
    // Files reached through a symlink are recorded under the path they were
    // reached by, which is also the path the game sees.
    for entry in WalkDir::new(root).follow_links(true).sort_by_file_name() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                let at = e.path().map_or_else(|| root.display().to_string(), |p| relative_slash_path(root, p));
                scan.issue(&at, format!("walk error: {e}"));
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = relative_slash_path(root, entry.path());
        if let Err(e) = scan_file(entry.path(), &rel, options, &mut scan) {
            scan.issue(&rel, format!("read error: {e}"));
        }
    }
    let edges =
        if options.dependencies { resolve(&scan.assets, std::mem::take(&mut scan.pending)) } else { Vec::new() };
    let mut assets = scan.assets;
    assets.sort_by(|a, b| a.id.cmp(&b.id));
    scan.issues.sort_by(|a, b| (&a.asset, &a.message).cmp(&(&b.asset, &b.message)));
    Ok(Catalog {
        schema: SCHEMA_VERSION,
        tool: format!("chernarus-assets {}", env!("CARGO_PKG_VERSION")),
        root: root.display().to_string(),
        assets,
        edges,
        issues: scan.issues,
    })
}

fn reference_record(
    id: String,
    virtual_path: Option<String>,
    location: Location,
    size: u64,
    ident: Identification,
) -> AssetRecord {
    AssetRecord {
        id,
        virtual_path,
        location,
        size,
        sha256: None,
        format: ident.format,
        identified_by: ident.basis,
        format_conflict: ident.conflict,
        provenance: ProvenanceClass::Reference,
        redistributable: false,
        pbo: None,
        lzss_checksum: None,
    }
}

fn scan_file(path: &Path, rel: &str, options: &ScanOptions, scan: &mut Scan) -> std::io::Result<()> {
    let size = std::fs::metadata(path)?.len();
    let mut head = Vec::with_capacity(SNIFF_LEN);
    File::open(path)?.take(SNIFF_LEN as u64).read_to_end(&mut head)?;
    let ident = identify(rel, &head);
    let mut record = reference_record(rel.to_owned(), None, Location::File { path: rel.to_owned() }, size, ident);
    if options.hash {
        record.sha256 = Some(sha256_file(path)?);
    }

    if ident.format == Format::Pbo {
        scan_pbo(path, rel, size, options, scan, &mut record)?;
    } else if options.dependencies && size <= options.max_content_bytes && wants_dependency_scan(ident.format) {
        let data = std::fs::read(path)?;
        extract_references(rel, "", ident.format, &data, scan);
    }
    scan.assets.push(record);
    Ok(())
}

fn scan_pbo(
    path: &Path,
    rel: &str,
    size: u64,
    options: &ScanOptions,
    scan: &mut Scan,
    record: &mut AssetRecord,
) -> std::io::Result<()> {
    let header = match PboHeader::read(BufReader::new(File::open(path)?), size) {
        Ok(h) => h,
        Err(e) => {
            scan.issue(rel, format!("PBO header not readable: {e}"));
            return Ok(());
        }
    };
    let (prefix, prefix_source) = match header.prefix() {
        Some(p) => (p.to_owned(), PrefixSource::Header),
        None => {
            let stem = Path::new(rel).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            (stem, PrefixSource::FileName)
        }
    };
    let mut file = BufReader::new(File::open(path)?);
    let checksum = if options.verify_pbo_checksums {
        match header.verify_checksum(&mut file) {
            Ok(c) => Some(c),
            Err(e) => {
                scan.issue(rel, format!("PBO checksum not verifiable: {e}"));
                None
            }
        }
    } else {
        None
    };
    record.pbo = Some(PboDetails {
        prefix: Some(prefix.clone()),
        prefix_source,
        extensions: header.extensions.clone(),
        entry_count: header.entries.len(),
        checksum,
    });

    for entry in &header.entries {
        let id = format!("{rel}::{}", entry.name);
        let virtual_path = vfs::mounted(&prefix, &entry.name);
        let location = Location::PboEntry { pbo: rel.to_owned(), entry: entry.name.clone(), packing: entry.packing };
        let head = match header.read_prefix(&mut file, entry, SNIFF_LEN) {
            Ok(h) => h,
            Err(e) => {
                scan.issue(&id, format!("entry not readable: {e}"));
                let ident = identify(&entry.name, &[]);
                scan.assets.push(reference_record(id, Some(virtual_path), location, entry.content_size(), ident));
                continue;
            }
        };
        let ident = identify(&entry.name, &head);
        let mut child = reference_record(id.clone(), Some(virtual_path.clone()), location, entry.content_size(), ident);
        let want_deps = options.dependencies && wants_dependency_scan(ident.format);
        if (options.hash || want_deps) && entry.content_size() <= options.max_content_bytes {
            match header.read_entry(&mut file, entry) {
                Ok(content) => {
                    child.lzss_checksum = content.lzss_checksum;
                    if options.hash {
                        child.sha256 = Some(sha256_bytes(&content.data));
                    }
                    if want_deps {
                        extract_references(&id, vfs::parent(&virtual_path), ident.format, &content.data, scan);
                    }
                }
                Err(e) => scan.issue(&id, format!("entry content not readable: {e}")),
            }
        }
        scan.assets.push(child);
    }
    Ok(())
}

fn wants_dependency_scan(format: Format) -> bool {
    format.is_text()
        || matches!(
            format,
            Format::RapConfig | Format::P3dMlod | Format::P3dOdol | Format::WrpOprw | Format::WrpEditable
        )
}

fn extract_references(from: &str, from_dir: &str, format: Format, data: &[u8], scan: &mut Scan) {
    let push = |raw: &str, method: EdgeMethod, context: Option<String>, scan: &mut Scan| {
        scan.pending.push(PendingRef {
            from: from.to_owned(),
            from_dir: from_dir.to_owned(),
            raw: raw.to_owned(),
            method,
            context,
        });
    };
    match format {
        Format::RapConfig => match rap::read(data) {
            Ok(config) => {
                let mut found = Vec::new();
                config.root.visit_strings(&mut |path, value| {
                    let method = if strings::has_asset_extension(value) {
                        Some(EdgeMethod::ConfigParse)
                    } else if looks_like_extensionless_path(value) {
                        Some(EdgeMethod::ConfigImplicitExtension)
                    } else {
                        None
                    };
                    if let Some(m) = method {
                        found.push((value.to_owned(), m, path.join("/")));
                    }
                });
                for (value, method, context) in found {
                    push(&value, method, Some(context), scan);
                }
            }
            Err(e) => scan.issue(from, format!("rapified config not parseable: {e}")),
        },
        f if f.is_text() => {
            for p in strings::asset_paths(data) {
                push(&p, EdgeMethod::TextScan, None, scan);
            }
        }
        _ => {
            for p in strings::asset_paths(data) {
                push(&p, EdgeMethod::BinaryStringScan, None, scan);
            }
        }
    }
}

/// `\a\b\c` style strings with no extension and no spaces.
fn looks_like_extensionless_path(s: &str) -> bool {
    let last = s.rsplit('\\').next().unwrap_or(s);
    s.contains('\\')
        && !last.is_empty()
        && !last.contains('.')
        && !s.starts_with('#')
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'\\' | b'-'))
}

fn resolve(assets: &[AssetRecord], pending: Vec<PendingRef>) -> Vec<Edge> {
    let mut providers: HashMap<&str, Vec<&str>> = HashMap::new();
    for a in assets {
        if let Some(vp) = &a.virtual_path {
            providers.entry(vp.as_str()).or_default().push(a.id.as_str());
        }
    }
    let lookup =
        |candidate: &str| providers.get(candidate).map(|v| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());

    let mut edges = Vec::new();
    for p in pending {
        let absolute = vfs::normalize(&p.raw);
        let relative = (!p.from_dir.is_empty() && !p.raw.starts_with('\\')).then(|| vfs::join(&p.from_dir, &p.raw));
        let mut candidates = vec![absolute.clone()];
        candidates.extend(relative);
        if p.method == EdgeMethod::ConfigImplicitExtension {
            candidates = candidates
                .iter()
                .flat_map(|c| IMPLICIT_EXTENSIONS.iter().map(move |ext| format!("{c}.{ext}")))
                .collect();
        }
        if p.method == EdgeMethod::BinaryStringScan {
            // Longest suffix first, so the least-trimmed resolvable path wins.
            for k in 1..=MAX_BINARY_TRIM.min(p.raw.len().saturating_sub(5)) {
                let Some(suffix) = p.raw.get(k..) else { continue };
                candidates.push(vfs::normalize(suffix));
                if !p.from_dir.is_empty() {
                    candidates.push(vfs::join(&p.from_dir, suffix));
                }
            }
        }
        let found = candidates.iter().find_map(|c| lookup(c).map(|ids| (c.clone(), ids)));
        match found {
            Some((target, ids)) => {
                edges.push(Edge { from: p.from, target, providers: ids, method: p.method, context: p.context })
            }
            // An implicit-extension guess that matches nothing is not a reference.
            None if p.method == EdgeMethod::ConfigImplicitExtension => {}
            None => edges.push(Edge {
                from: p.from,
                target: absolute,
                providers: Vec::new(),
                method: p.method,
                context: p.context,
            }),
        }
    }
    edges.sort_by(|a, b| (&a.from, &a.target, a.method, &a.context).cmp(&(&b.from, &b.target, b.method, &b.context)));
    edges.dedup();
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensionless_path_heuristic() {
        assert!(looks_like_extensionless_path("\\dz\\sounds\\zombie_growl1"));
        assert!(!looks_like_extensionless_path("single_word"));
        assert!(!looks_like_extensionless_path("\\has space\\x"));
        assert!(!looks_like_extensionless_path("\\a\\b.paa"));
        assert!(!looks_like_extensionless_path("#(argb,8,8,3)color(1,1,1,1)"));
    }
}
