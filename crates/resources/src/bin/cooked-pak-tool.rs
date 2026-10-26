//! `cooked-pak-tool`: builds, composes and checks the launcher presync data
//! offline (LP-04). Subcommands take `--flag value` pairs:
//!
//! - `build --data-dir <dir>` loads the server's cooked data and prints the
//!   served index JSON.
//! - `compose --data-dir <dir> --source <dir> --out <dir>` composes each
//!   category the presync would write from `<source>/<pak>` and the served
//!   body, and writes `<out>/<pak>`. It prints `<id> <pak> <action> <reason>`
//!   per category.
//! - `check --data-dir <dir> --source <dir>` prints one row per category on how
//!   the source PAK's records line up with the served ones.
//! - `layout --pak <file>` lists a PAK's zip entries in order.
//!
//! Exit codes: 0 on success, 1 when a category fails (`compose`) or the source
//! base doesn't line up (`check`), 2 on a usage or I/O error.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs::File;
use std::path::Path;
use std::process::ExitCode;

use cimmeria_cooked_pak::catalog::{decode_body, CategoryBody, Index, IndexEntry, Limits};
use cimmeria_cooked_pak::compose::{compose, ComposeError};
use cimmeria_cooked_pak::decide::{decide, Action};
use cimmeria_cooked_pak::pak::{read_pak, read_version, write_pak};
use cimmeria_resources::base::resources::ResourceCache;
use cimmeria_resources::base::served_catalog::check::{check_category, CheckRow};
use cimmeria_resources::base::served_catalog::ServedCatalog;

const USAGE: &str = "usage: cooked-pak-tool <build|compose|check|layout> [--flag value ...]
  build   --data-dir <dir>
  compose --data-dir <dir> --source <dir> --out <dir>
  check   --data-dir <dir> --source <dir>
  layout  --pak <file>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("cooked-pak-tool: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let (command, rest) = args.split_first().ok_or_else(|| USAGE.to_string())?;
    match command.as_str() {
        "build" => build(&Flags::parse(rest, &["data-dir"])?),
        "compose" => compose_command(&Flags::parse(rest, &["data-dir", "source", "out"])?),
        "check" => check_command(&Flags::parse(rest, &["data-dir", "source"])?),
        "layout" => layout(&Flags::parse(rest, &["pak"])?),
        other => Err(format!("unknown subcommand {other}\n{USAGE}")),
    }
}

/// The `--flag value` pairs of one subcommand.
struct Flags(BTreeMap<String, String>);

impl Flags {
    /// Parse `args` as pairs, accepting only the `allowed` flag names.
    fn parse(args: &[String], allowed: &[&str]) -> Result<Self, String> {
        let mut map = BTreeMap::new();
        let mut iter = args.iter();
        while let Some(flag) = iter.next() {
            let name = flag
                .strip_prefix("--")
                .ok_or_else(|| format!("expected a --flag, got {flag}"))?;
            if !allowed.contains(&name) {
                return Err(format!("unknown flag --{name}"));
            }
            let value = iter
                .next()
                .ok_or_else(|| format!("--{name} needs a value"))?;
            if map.insert(name.to_string(), value.clone()).is_some() {
                return Err(format!("--{name} given twice"));
            }
        }
        Ok(Self(map))
    }

    fn required(&self, name: &str) -> Result<&str, String> {
        self.0
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| format!("missing --{name}"))
    }
}

/// Load the server's cooked data from `data_dir` and build its served catalog.
fn load(data_dir: &str) -> Result<(ResourceCache, ServedCatalog), String> {
    let cache = ResourceCache::load_all(data_dir)?;
    let catalog = ServedCatalog::build(&cache, data_dir)?;
    Ok((cache, catalog))
}

/// Every served category in index order, with its decoded body.
fn served_bodies(catalog: &ServedCatalog) -> Result<Vec<(IndexEntry, CategoryBody)>, String> {
    let index: Index =
        serde_json::from_slice(&catalog.index_json()).map_err(|e| format!("index: {e}"))?;
    index
        .categories
        .into_iter()
        .map(|entry| {
            let served = catalog.category(entry.category_id).ok_or_else(|| {
                format!("category {} missing from the catalog", entry.category_id)
            })?;
            let body = decode_body(&served.body, &Limits::DEFAULT)
                .map_err(|e| format!("category {}: {e:?}", entry.category_id))?;
            Ok((entry, body))
        })
        .collect()
}

fn build(flags: &Flags) -> Result<ExitCode, String> {
    let (_, catalog) = load(flags.required("data-dir")?)?;
    println!("{}", String::from_utf8_lossy(&catalog.index_json()));
    Ok(ExitCode::SUCCESS)
}

fn compose_command(flags: &Flags) -> Result<ExitCode, String> {
    let (_, catalog) = load(flags.required("data-dir")?)?;
    let source = Path::new(flags.required("source")?);
    let out = Path::new(flags.required("out")?);
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut failed = 0;
    for (entry, body) in served_bodies(&catalog)? {
        let source_pak = source.join(&entry.pak);
        let out_pak = out.join(&entry.pak);
        let source_version =
            read_version(&source_pak).map_err(|e| format!("{}: {e}", source_pak.display()))?;
        let action = decide(existing_version(&out_pak), source_version, body.version);
        let (label, reason) = match action {
            Action::UpToDate => ("up_to_date", "versions_match"),
            Action::SourceNewer => {
                failed += 1;
                ("source_newer", "source_newer")
            }
            Action::Write => match write_category(&body, &source_pak, &out_pak) {
                Ok(()) => ("write", "written"),
                Err(reason) => {
                    failed += 1;
                    ("failed", reason)
                }
            },
        };
        println!("{} {} {label} {reason}", body.category_id, entry.pak);
    }
    Ok(exit_for(failed))
}

/// Compose one category from `source_pak` and the body, and write `out_pak`.
/// Returns the reason code on failure; the detail goes to stderr.
fn write_category(
    body: &CategoryBody,
    source_pak: &Path,
    out_pak: &Path,
) -> Result<(), &'static str> {
    let base = read_pak(source_pak)
        .map_err(|e| report("source_unreadable", source_pak, e))?
        .entries;
    let entries = compose(body, &base).map_err(|e| {
        let code = match e {
            ComposeError::BaseMissing { .. } => "base_missing",
            ComposeError::DigestMismatch => "digest_mismatch",
        };
        report(code, source_pak, e)
    })?;
    let file = File::create(out_pak).map_err(|e| report("write_failed", out_pak, e))?;
    write_pak(file, body.version, &entries).map_err(|e| report("write_failed", out_pak, e))
}

/// The version of an existing cache PAK. An absent or unreadable file gives
/// `None`: the tool composes over it as if the cache had no copy.
fn existing_version(path: &Path) -> Option<u32> {
    if !path.exists() {
        return None;
    }
    read_version(path).ok().flatten()
}

/// Print `path: error` to stderr and return the reason code.
fn report(code: &'static str, path: &Path, error: impl Display) -> &'static str {
    eprintln!("{}: {error}", path.display());
    code
}

fn check_command(flags: &Flags) -> Result<ExitCode, String> {
    let (cache, catalog) = load(flags.required("data-dir")?)?;
    let source = Path::new(flags.required("source")?);
    println!("category_id pak served_version source_version keys changed base_missing base_differs action");
    let mut bad = 0;
    for (entry, body) in served_bodies(&catalog)? {
        let source_pak = source.join(&entry.pak);
        let base = read_pak(&source_pak).map_err(|e| format!("{}: {e}", source_pak.display()))?;
        let served = &cache
            .category(body.category_id)
            .ok_or_else(|| format!("category {} missing from the cache", body.category_id))?
            .elements;
        let row = check_category(
            body.category_id,
            &entry.pak,
            &body,
            served,
            &base.entries,
            base.version,
        );
        print_row(&row);
        if !row.base_ok() {
            bad += 1;
        }
    }
    Ok(exit_for(bad))
}

fn print_row(row: &CheckRow) {
    let source_version = row
        .source_version
        .map_or_else(|| "-".to_string(), |v| v.to_string());
    println!(
        "{} {} {} {} {} {} {} {} {}",
        row.category_id,
        row.pak,
        row.served_version,
        source_version,
        row.keys,
        row.changed,
        row.base_missing,
        row.base_differs,
        action_name(row.action),
    );
}

fn action_name(action: Action) -> &'static str {
    match action {
        Action::UpToDate => "up_to_date",
        Action::Write => "write",
        Action::SourceNewer => "source_newer",
    }
}

fn layout(flags: &Flags) -> Result<ExitCode, String> {
    let pak = Path::new(flags.required("pak")?);
    let file = File::open(pak).map_err(|e| format!("{}: {e}", pak.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("{}: {e}", pak.display()))?;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| format!("{}: {e}", pak.display()))?;
        let name = entry
            .name()
            .map_err(|e| format!("{}: {e}", pak.display()))?;
        println!(
            "{} {:?} {} {}",
            name,
            entry.compression(),
            entry.compressed_size(),
            entry.size()
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn exit_for(problems: u32) -> ExitCode {
    if problems == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
