//! Hand-rolled argument parsing for the `extract_map` CLI.
//!
//! Two rules drive the shape here, both because the tool's output is
//! *measurement*:
//!
//! - A flag with no value, a bare positional, a repeat, or a typo is an
//!   error. Silently ignoring `--chunkfilter` would report coverage for
//!   a whole map while the operator believed one chunk ran.
//! - Every default is derived from another flag (`--report` from
//!   `--out`, `--detail` from `--obj-dir`) rather than hard-coded to a
//!   CWD-relative path, so two runs never overwrite each other's
//!   reports by accident.

use std::path::PathBuf;

use cimmeria_navmesh_extractor::floor_probe::{report as probe_report, AxisMapping, ProbeConfig};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExtractArgs {
    pub cooked_root: PathBuf,
    pub map: String,
    pub out: PathBuf,
    pub index: PathBuf,
    pub chunk_filter: Option<String>,
    pub report: Option<PathBuf>,
    pub classes: Option<PathBuf>,
    /// Opt-in whole-map OBJ. Defaults to not writing one: a non
    /// `<hex8>o.obj` file in the per-chunk directory breaks
    /// NavBuilder's chunked build (and NavBuilder still exits 0).
    pub combined: Option<PathBuf>,
}

impl ExtractArgs {
    /// `<cooked_root>/Maps/<map>`.
    pub fn map_dir(&self) -> PathBuf {
        self.cooked_root.join("Maps").join(&self.map)
    }
    pub fn report_path(&self) -> PathBuf {
        self.report
            .clone()
            .unwrap_or_else(|| self.out.join("coverage.tsv"))
    }
    pub fn classes_path(&self) -> PathBuf {
        self.classes
            .clone()
            .unwrap_or_else(|| self.out.join("coverage_classes.tsv"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProbeArgs {
    pub obj_dir: PathBuf,
    pub mappings: Vec<AxisMapping>,
    pub points: Option<PathBuf>,
    pub report: Option<PathBuf>,
    pub detail: Option<PathBuf>,
    pub config: ProbeConfig,
}

impl ProbeArgs {
    pub fn report_path(&self) -> PathBuf {
        self.report
            .clone()
            .unwrap_or_else(|| self.obj_dir.join("probe_mappings.tsv"))
    }
    pub fn detail_path(&self) -> PathBuf {
        self.detail
            .clone()
            .unwrap_or_else(|| self.obj_dir.join("probe_points.tsv"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Args {
    Extract(ExtractArgs),
    Probe(ProbeArgs),
}

impl Args {
    /// Parse `argv` (already stripped of `argv[0]`).
    ///
    /// `Ok(None)` means "the user asked for help" — print usage, exit 0.
    pub fn parse(argv: &[String]) -> Result<Option<Args>, String> {
        let Some(mode) = argv.first() else {
            return Ok(None);
        };
        if mode == "-h" || mode == "--help" || mode == "help" {
            return Ok(None);
        }

        // Positional shorthand for the extract path, so the
        // `tools/build-navmesh.*` wrappers can call
        // `extract_map <cooked-root> <map> <out> <index>` without
        // knowing the flag names.
        if mode != "extract" && mode != "probe" && !mode.starts_with("--") {
            return positional_extract(argv).map(Some);
        }

        let mut flags = Flags::collect(&argv[1..])?;
        let parsed = match mode.as_str() {
            "extract" => Args::Extract(ExtractArgs {
                cooked_root: flags.take_required_path("--cooked-root")?,
                map: flags.take_required("--map")?,
                out: flags.take_required_path("--out")?,
                index: flags.take_required_path("--index")?,
                chunk_filter: flags.take("--chunk-filter"),
                report: flags.take_path("--report"),
                classes: flags.take_path("--classes"),
                combined: flags.take_path("--combined"),
            }),
            "probe" => {
                let mappings = match flags.take("--mapping") {
                    None => AxisMapping::all(),
                    Some(sel) => {
                        probe_report::parse_mapping_selector(&sel).map_err(|e| e.to_string())?
                    }
                };
                let mut config = ProbeConfig::default();
                if let Some(v) = flags.take_f32("--below")? {
                    config.below = v;
                }
                if let Some(v) = flags.take_f32("--above")? {
                    config.above = v;
                }
                if let Some(v) = flags.take_f32("--neighbourhood")? {
                    config.neighbourhood = v;
                }
                Args::Probe(ProbeArgs {
                    obj_dir: flags.take_required_path("--obj-dir")?,
                    mappings,
                    points: flags.take_path("--points"),
                    report: flags.take_path("--report"),
                    detail: flags.take_path("--detail"),
                    config,
                })
            }
            other => {
                return Err(format!(
                    "unknown mode {other:?} (want `extract` or `probe`)"
                ))
            }
        };
        flags.finish()?;
        Ok(Some(parsed))
    }
}

/// `extract_map <cooked-root> <map-name> <out-dir> <index-path>`.
fn positional_extract(argv: &[String]) -> Result<Args, String> {
    if argv.len() != 4 {
        return Err(format!(
            "positional form takes exactly 4 arguments \
             (<cooked-root> <map-name> <out-dir> <index-path>), got {}",
            argv.len()
        ));
    }
    if let Some(flagged) = argv.iter().find(|a| a.starts_with("--")) {
        return Err(format!(
            "positional form takes no flags, but saw {flagged:?}"
        ));
    }
    Ok(Args::Extract(ExtractArgs {
        cooked_root: PathBuf::from(&argv[0]),
        map: argv[1].clone(),
        out: PathBuf::from(&argv[2]),
        index: PathBuf::from(&argv[3]),
        chunk_filter: None,
        report: None,
        classes: None,
        combined: None,
    }))
}

/// `--flag value` pairs, consumed by name so an unrecognised flag can be
/// reported instead of silently ignored.
#[derive(Debug, Default)]
struct Flags {
    pairs: Vec<(String, String)>,
}

impl Flags {
    fn collect(rest: &[String]) -> Result<Self, String> {
        let mut pairs: Vec<(String, String)> = Vec::new();
        let mut i = 0;
        while i < rest.len() {
            let key = &rest[i];
            if !key.starts_with("--") {
                return Err(format!("expected a --flag, got {key:?}"));
            }
            let Some(value) = rest.get(i + 1) else {
                return Err(format!("{key} needs a value"));
            };
            if value.starts_with("--") {
                return Err(format!("{key} needs a value, got the flag {value:?}"));
            }
            if pairs.iter().any(|(k, _)| k == key) {
                return Err(format!("{key} given more than once"));
            }
            pairs.push((key.clone(), value.clone()));
            i += 2;
        }
        Ok(Self { pairs })
    }

    fn take(&mut self, key: &str) -> Option<String> {
        let pos = self.pairs.iter().position(|(k, _)| k == key)?;
        Some(self.pairs.remove(pos).1)
    }

    fn take_path(&mut self, key: &str) -> Option<PathBuf> {
        self.take(key).map(PathBuf::from)
    }

    fn take_required(&mut self, key: &str) -> Result<String, String> {
        self.take(key).ok_or_else(|| format!("{key} is required"))
    }

    fn take_required_path(&mut self, key: &str) -> Result<PathBuf, String> {
        self.take_required(key).map(PathBuf::from)
    }

    fn take_f32(&mut self, key: &str) -> Result<Option<f32>, String> {
        match self.take(key) {
            None => Ok(None),
            Some(v) => v
                .parse::<f32>()
                .map(Some)
                .map_err(|_| format!("{key} wants a number, got {v:?}")),
        }
    }

    /// Error on anything left over — a typo'd flag must not be silently
    /// dropped when the whole point of the tool is measurement.
    fn finish(self) -> Result<(), String> {
        if let Some((k, _)) = self.pairs.first() {
            return Err(format!("unrecognised flag {k:?}"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn no_args_and_help_request_usage() {
        assert_eq!(Args::parse(&[]).unwrap(), None);
        assert_eq!(Args::parse(&argv(&["--help"])).unwrap(), None);
        assert_eq!(Args::parse(&argv(&["-h"])).unwrap(), None);
        assert_eq!(Args::parse(&argv(&["help"])).unwrap(), None);
    }

    #[test]
    fn a_bare_word_that_is_not_a_mode_is_read_as_the_positional_form() {
        // Four bare words: the `tools/build-navmesh.*` wrapper shape.
        let a = Args::parse(&argv(&[
            "/c/CookedPC",
            "Castle",
            "/tmp/out",
            "/tmp/index.bin",
        ]))
        .unwrap()
        .unwrap();
        let Args::Extract(a) = a else {
            panic!("wrong mode")
        };
        assert_eq!(a.cooked_root, PathBuf::from("/c/CookedPC"));
        assert_eq!(a.map, "Castle");
        assert_eq!(a.out, PathBuf::from("/tmp/out"));
        assert_eq!(a.index, PathBuf::from("/tmp/index.bin"));
        assert_eq!(a.combined, None, "never into the per-chunk dir by default");

        // Anything other than exactly four is an error, not a partial run.
        let err = Args::parse(&argv(&["frobnicate"])).unwrap_err();
        assert!(err.contains("exactly 4 arguments"), "{err}");
        let err = Args::parse(&argv(&["/c", "Castle", "/o", "/i", "extra"])).unwrap_err();
        assert!(err.contains("exactly 4 arguments"), "{err}");
    }

    #[test]
    fn the_positional_form_rejects_a_stray_flag() {
        let err = Args::parse(&argv(&["/c", "Castle", "/o", "--index"])).unwrap_err();
        assert!(err.contains("takes no flags"), "{err}");
    }

    #[test]
    fn extract_parses_the_required_four_and_derives_defaults() {
        let a = Args::parse(&argv(&[
            "extract",
            "--cooked-root",
            "/c/CookedPC",
            "--map",
            "Castle",
            "--out",
            "/tmp/out",
            "--index",
            "/tmp/index.bin",
        ]))
        .unwrap()
        .unwrap();
        let Args::Extract(a) = a else {
            panic!("wrong mode")
        };
        assert_eq!(a.map, "Castle");
        assert_eq!(a.map_dir(), PathBuf::from("/c/CookedPC/Maps/Castle"));
        assert_eq!(a.report_path(), PathBuf::from("/tmp/out/coverage.tsv"));
        assert_eq!(
            a.classes_path(),
            PathBuf::from("/tmp/out/coverage_classes.tsv")
        );
        assert_eq!(a.chunk_filter, None);
        // No whole-map OBJ unless asked: one in the per-chunk dir kills
        // NavBuilder's chunked build while it still exits 0.
        assert_eq!(a.combined, None);
    }

    #[test]
    fn extract_accepts_the_optional_flags() {
        let a = Args::parse(&argv(&[
            "extract",
            "--cooked-root",
            "/c/CookedPC",
            "--map",
            "Castle",
            "--out",
            "/tmp/out",
            "--index",
            "/tmp/index.bin",
            "--chunk-filter",
            "000a0002",
            "--report",
            "/tmp/r.tsv",
            "--classes",
            "/tmp/c.tsv",
            "--combined",
            "/tmp/whole/castle.obj",
        ]))
        .unwrap()
        .unwrap();
        let Args::Extract(a) = a else {
            panic!("wrong mode")
        };
        assert_eq!(a.chunk_filter.as_deref(), Some("000a0002"));
        assert_eq!(a.report_path(), PathBuf::from("/tmp/r.tsv"));
        assert_eq!(a.classes_path(), PathBuf::from("/tmp/c.tsv"));
        assert_eq!(a.combined, Some(PathBuf::from("/tmp/whole/castle.obj")));
    }

    #[test]
    fn extract_rejects_a_missing_required_flag() {
        let err = Args::parse(&argv(&["extract", "--map", "Castle"])).unwrap_err();
        assert!(err.contains("--cooked-root is required"), "{err}");
    }

    /// A typo'd flag must fail rather than be dropped — silently
    /// ignoring `--chunkfilter` would make the tool report coverage for
    /// the whole map while the operator thinks it ran on one chunk.
    #[test]
    fn an_unrecognised_flag_is_rejected() {
        let err = Args::parse(&argv(&[
            "extract",
            "--cooked-root",
            "/c",
            "--map",
            "Castle",
            "--out",
            "/o",
            "--index",
            "/i",
            "--chunkfilter",
            "x",
        ]))
        .unwrap_err();
        assert!(err.contains("unrecognised flag"), "{err}");
    }

    #[test]
    fn a_flag_without_a_value_is_rejected() {
        let err = Args::parse(&argv(&["extract", "--map"])).unwrap_err();
        assert!(err.contains("--map needs a value"), "{err}");

        let err = Args::parse(&argv(&["extract", "--map", "--out"])).unwrap_err();
        assert!(err.contains("needs a value"), "{err}");
    }

    #[test]
    fn a_bare_positional_is_rejected() {
        let err = Args::parse(&argv(&["extract", "Castle"])).unwrap_err();
        assert!(err.contains("expected a --flag"), "{err}");
    }

    #[test]
    fn a_repeated_flag_is_rejected() {
        let err = Args::parse(&argv(&[
            "extract",
            "--map",
            "Castle",
            "--map",
            "Harset",
            "--cooked-root",
            "/c",
            "--out",
            "/o",
            "--index",
            "/i",
        ]))
        .unwrap_err();
        assert!(err.contains("more than once"), "{err}");
    }

    #[test]
    fn probe_defaults_to_all_48_mappings_and_the_builtin_points() {
        let a = Args::parse(&argv(&["probe", "--obj-dir", "/tmp/out"]))
            .unwrap()
            .unwrap();
        let Args::Probe(a) = a else {
            panic!("wrong mode")
        };
        assert_eq!(a.mappings.len(), 48);
        assert_eq!(a.points, None);
        assert_eq!(
            a.report_path(),
            PathBuf::from("/tmp/out/probe_mappings.tsv")
        );
        assert_eq!(a.detail_path(), PathBuf::from("/tmp/out/probe_points.tsv"));
        assert_eq!(a.config.below, 1.5);
        assert_eq!(a.config.above, 0.5);
    }

    #[test]
    fn probe_accepts_a_mapping_list_and_numeric_tolerances() {
        let a = Args::parse(&argv(&[
            "probe",
            "--obj-dir",
            "/tmp/out",
            "--mapping",
            "+Y+Z+X,+Z+Y+X",
            "--below",
            "3",
            "--above",
            "0.25",
            "--neighbourhood",
            "12.5",
        ]))
        .unwrap()
        .unwrap();
        let Args::Probe(a) = a else {
            panic!("wrong mode")
        };
        assert_eq!(
            a.mappings,
            vec![AxisMapping::CA05, AxisMapping::NAVBUILDER_ON_RAW_UE3]
        );
        assert_eq!(a.config.below, 3.0);
        assert_eq!(a.config.above, 0.25);
        assert_eq!(a.config.neighbourhood, 12.5);
    }

    #[test]
    fn probe_rejects_a_non_numeric_tolerance_and_a_bad_mapping() {
        let err = Args::parse(&argv(&["probe", "--obj-dir", "/o", "--below", "deep"])).unwrap_err();
        assert!(err.contains("--below wants a number"), "{err}");

        let err =
            Args::parse(&argv(&["probe", "--obj-dir", "/o", "--mapping", "+Q+Z+X"])).unwrap_err();
        assert!(err.contains("axis-mapping label"), "{err}");
    }

    #[test]
    fn probe_requires_an_obj_dir() {
        let err = Args::parse(&argv(&["probe", "--mapping", "all"])).unwrap_err();
        assert!(err.contains("--obj-dir is required"), "{err}");
    }
}
