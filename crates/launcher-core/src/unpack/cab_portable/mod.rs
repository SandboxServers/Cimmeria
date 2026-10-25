//! Expand a MakeCAB cabinet chain in Rust, without Windows' `cabinet.dll`.
//!
//! The Windows launcher expands the 2009 installer's cabinets with FDI
//! ([`super::fdi`]). Other hosts use this module. It handles what that set
//! needs: MSZIP (and uncompressed) folders, and folders that continue from
//! one cabinet into the next. Quantum and LZX folders are refused.
//!
//! A spanning set works like this. When a cabinet fills up mid-file, its last
//! folder continues as the first folder of the next cabinet: the file is
//! listed in both cabinets, marked `CONTINUED_TO_NEXT` in the first and
//! `CONTINUED_FROM_PREV` in the second, and the folder's last data block may
//! be split, its first part (with an uncompressed size of 0) at the end of
//! one cabinet and the rest at the start of the next. Following libmspack,
//! the continuation markers decide which folders join. A joined folder is one
//! compressed stream, so MSZIP history carries across the boundary too.
//!
//! The behaviour matches the FDI path: every name is checked and the file
//! count compared with the INF before anything is created, each file gets
//! the cabinet's DOS date and time, and progress is reported per file.

mod format;
mod mszip;
mod positional;
mod schedule;
pub(super) mod single;

#[cfg(test)]
pub(super) mod fixture;
#[cfg(test)]
mod parallel_tests;
#[cfg(test)]
mod tests;

use std::fs::File;
use std::io::{BufReader, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use format::{Cabinet, Compression};
use positional::PositionalReader;

use super::{dos_time, entry_inventory::EntryInventory, UnpackError, UnpackSink};

/// Most folders expanded at once. Each thread holds a decoder and a read
/// buffer; more than a few gain little on the cabinets' sizes.
const MAX_WORKERS: usize = 4;

/// One cabinet's share of a folder's data blocks.
struct Part {
    cabinet: usize,
    data_offset: u32,
    blocks: u16,
    /// Compressed bytes this part spans: up to the next folder's data in its
    /// cabinet, or to the cabinet's end.
    bytes: u64,
}

struct PlannedFile {
    path: PathBuf,
    start: u64,
    end: u64,
    date: u16,
    time: u16,
}

/// A folder as one stream, joined across the cabinets it spans.
struct Folder {
    compression: Compression,
    parts: Vec<Part>,
    files: Vec<PlannedFile>,
}

/// Expand `cabinets` (bare file names in `cab_dir`, in chain order) into
/// `dest`. `total` is the file count from the set's INF; a different count
/// is an error rather than a partial install.
pub(super) fn expand_chain(
    cab_dir: &Path,
    cabinets: &[String],
    dest: &Path,
    total: usize,
    sink: &UnpackSink,
) -> Result<(), UnpackError> {
    let workers = std::thread::available_parallelism().map_or(1, NonZeroUsize::get);
    expand_chain_with(
        cab_dir,
        cabinets,
        dest,
        total,
        sink,
        workers.min(MAX_WORKERS),
    )
}

/// [`expand_chain`] with at most `workers` threads. Folders are the unit of
/// work, so the count is also capped by the number of folders.
pub(super) fn expand_chain_with(
    cab_dir: &Path,
    cabinets: &[String],
    dest: &Path,
    total: usize,
    sink: &UnpackSink,
    workers: usize,
) -> Result<(), UnpackError> {
    let mut cabinet_names = EntryInventory::default();
    let mut sources = Vec::new();
    let mut lens = Vec::new();
    let mut parsed = Vec::new();
    for name in cabinets {
        sink.check_cancel()?;
        if name.contains(['/', '\\']) {
            return Err(UnpackError::UnsafePath(name.clone()));
        }
        cabinet_names.insert(name, false)?;
        // Held open until expansion ends, so every read sees the same file.
        let file = File::open(cab_dir.join(name))
            .map_err(|error| UnpackError::Cab(format!("cannot open {name}: {error}")))?;
        parsed.push(format::parse(&file).map_err(|error| in_cabinet(name, error))?);
        lens.push(file.metadata()?.len());
        sources.push(file);
    }

    // Preflight: every name, and the count, before anything is created.
    let mut inventory = EntryInventory::default();
    let mut paths = Vec::new();
    for cabinet in &parsed {
        for file in cabinet.files.iter().filter(|f| !f.is_continuation()) {
            paths.push(inventory.insert(&file.name, false)?);
        }
    }
    if paths.len() != total {
        return Err(UnpackError::Cab(format!(
            "the cabinets held {} files but their index lists {total}",
            paths.len()
        )));
    }
    let folders = plan(cabinets, &parsed, &lens, paths)?;

    std::fs::create_dir_all(dest)?;
    let progress = Progress {
        sink,
        done: AtomicUsize::new(0),
        total,
        gate: Mutex::new(()),
    };
    let set = Set {
        names: cabinets,
        cabinets: &parsed,
        sources: &sources,
        lens: &lens,
    };
    let sizes: Vec<u64> = folders
        .iter()
        .map(|folder| folder.parts.iter().map(|part| part.bytes).sum())
        .collect();
    let lanes = schedule::assign(&sizes, workers.min(folders.len()).max(1));
    schedule::run(&lanes, |index, stop| {
        expand_folder(&folders[index], &set, dest, &progress, stop)
    })
}

/// The cabinet set a folder reads from: names for messages, parsed headers,
/// open handles, and each file's length.
struct Set<'a> {
    names: &'a [String],
    cabinets: &'a [Cabinet],
    sources: &'a [File],
    lens: &'a [u64],
}

fn in_cabinet(name: &str, error: UnpackError) -> UnpackError {
    match error {
        UnpackError::Cab(message) => UnpackError::Cab(format!("{name}: {message}")),
        other => other,
    }
}

/// Join continued folders across cabinets and assign each file to its
/// folder. `paths` are the preflighted install paths, in listing order and
/// `lens` the cabinets' file lengths.
fn plan(
    names: &[String],
    cabinets: &[Cabinet],
    lens: &[u64],
    paths: Vec<PathBuf>,
) -> Result<Vec<Folder>, UnpackError> {
    let mut paths = paths.into_iter();
    let mut folders: Vec<Folder> = Vec::new();
    let mut open_to_next = false;
    for (index, cabinet) in cabinets.iter().enumerate() {
        let name = &names[index];
        let continues = cabinet.files.iter().any(|f| f.is_continuation());
        if continues != open_to_next {
            return Err(UnpackError::Cab(if continues {
                format!("{name} continues a file that no earlier cabinet starts")
            } else {
                format!(
                    "{} continues into {name}, which does not carry it on",
                    names[index - 1]
                )
            }));
        }
        if continues {
            let prev = &cabinets[index - 1];
            if !(prev.has_next
                && cabinet.has_prev
                && prev.set_id == cabinet.set_id
                && prev.index.checked_add(1) == Some(cabinet.index))
            {
                return Err(UnpackError::Cab(format!(
                    "{name} is not the cabinet after {}",
                    names[index - 1]
                )));
            }
        }
        if cabinet.folders.is_empty() && !cabinet.files.is_empty() {
            return Err(UnpackError::Cab(format!(
                "{name} lists files but no folders"
            )));
        }

        let mut map = Vec::with_capacity(cabinet.folders.len());
        for (slot, folder) in cabinet.folders.iter().enumerate() {
            if matches!(
                folder.compression,
                Compression::Quantum | Compression::Lzx | Compression::Unknown(_)
            ) {
                return Err(UnpackError::Cab(format!(
                    "{name} uses {:?} compression; only MSZIP and stored cabinets \
                     expand outside Windows",
                    folder.compression
                )));
            }
            // Folders sit in data order within a cabinet, so the next one's
            // offset ends this one's data; the last runs to the cabinet's end.
            let data_end = cabinet
                .folders
                .get(slot + 1)
                .map_or(lens[index], |next| u64::from(next.data_offset));
            let part = Part {
                cabinet: index,
                data_offset: folder.data_offset,
                blocks: folder.blocks,
                bytes: data_end.saturating_sub(u64::from(folder.data_offset)),
            };
            match folders.last_mut() {
                Some(joined) if slot == 0 && continues => {
                    if joined.compression != folder.compression {
                        return Err(UnpackError::Cab(format!(
                            "{name} continues a folder with a different compression"
                        )));
                    }
                    joined.parts.push(part);
                }
                _ => folders.push(Folder {
                    compression: folder.compression,
                    parts: vec![part],
                    files: Vec::new(),
                }),
            }
            map.push(folders.len() - 1);
        }

        for file in cabinet.files.iter().filter(|f| !f.is_continuation()) {
            let slot = if file.folder == format::CONTINUED_TO_NEXT {
                cabinet.folders.len() - 1
            } else {
                usize::from(file.folder)
            };
            let start = u64::from(file.folder_offset);
            folders[map[slot]].files.push(PlannedFile {
                path: paths.next().expect("one preflighted path per listed file"),
                start,
                end: start + u64::from(file.size),
                date: file.date,
                time: file.time,
            });
        }
        open_to_next = cabinet.files.iter().any(|f| f.continues_to_next());
    }
    if open_to_next {
        return Err(UnpackError::Cab(format!(
            "{} continues into a cabinet the set does not list",
            names.last().map_or("", String::as_str)
        )));
    }
    Ok(folders)
}

/// Files finished so far across all workers, for the progress reports.
struct Progress<'a> {
    sink: &'a UnpackSink,
    done: AtomicUsize,
    total: usize,
    gate: Mutex<()>,
}

/// Decode one folder and write its files as their bytes stream past.
fn expand_folder(
    folder: &Folder,
    set: &Set<'_>,
    dest: &Path,
    progress: &Progress<'_>,
    stop: &schedule::Stop,
) -> Result<(), UnpackError> {
    let mut files: Vec<&PlannedFile> = folder.files.iter().collect();
    files.sort_by_key(|f| (f.start, f.end));
    let mut writer = FolderWriter {
        files,
        next: 0,
        pos: 0,
        current: None,
        dest,
    };
    writer.open_ready(progress)?;

    let mut decoder = mszip::Mszip::new();
    // A block split across a cabinet boundary collects here.
    let mut block = Vec::new();
    for part in &folder.parts {
        let name = &set.names[part.cabinet];
        let source = PositionalReader::new(
            &set.sources[part.cabinet],
            set.lens[part.cabinet],
            u64::from(part.data_offset),
        );
        let mut r = BufReader::with_capacity(64 * 1024, source);
        let reserve = set.cabinets[part.cabinet].data_reserve;
        for n in 0..part.blocks {
            progress.sink.check_cancel()?;
            stop.check()?;
            let header =
                format::read_block_header(&mut r, reserve).map_err(|e| in_cabinet(name, e))?;
            format::read_block_data(&mut r, header, &mut block).map_err(|e| in_cabinet(name, e))?;
            if header.uncompressed == 0 {
                // The first part of a split block; the rest opens the next
                // cabinet's share of this folder.
                if n + 1 != part.blocks {
                    return Err(UnpackError::Cab(format!(
                        "{name}: an empty data block sits mid-folder"
                    )));
                }
                continue;
            }
            let size = usize::from(header.uncompressed);
            let bytes = match folder.compression {
                Compression::Mszip => decoder
                    .block(&block, size)
                    .map_err(|message| UnpackError::Cab(format!("{name}: {message}")))?,
                _ if block.len() == size => &block[..],
                _ => {
                    return Err(UnpackError::Cab(format!(
                        "{name}: a stored data block's sizes disagree"
                    )))
                }
            };
            writer.write(bytes, progress)?;
            block.clear();
        }
    }
    if !block.is_empty() {
        return Err(UnpackError::Cab(
            "the cabinet data ends inside a split block".into(),
        ));
    }
    writer.finish(progress)
}

/// Routes a folder's uncompressed stream into the files it holds.
struct FolderWriter<'a> {
    /// Sorted by start offset.
    files: Vec<&'a PlannedFile>,
    next: usize,
    pos: u64,
    current: Option<(File, &'a PlannedFile)>,
    dest: &'a Path,
}

impl FolderWriter<'_> {
    /// Open every file that starts at the current position; zero-length ones
    /// are complete at once.
    fn open_ready(&mut self, progress: &Progress<'_>) -> Result<(), UnpackError> {
        while self.current.is_none() {
            let Some(&file) = self.files.get(self.next) else {
                return Ok(());
            };
            if file.start > self.pos {
                return Ok(());
            }
            if file.start < self.pos {
                return Err(UnpackError::Cab(format!(
                    "{} overlaps the file before it",
                    file.path.display()
                )));
            }
            self.next += 1;
            let out = self.dest.join(&file.path);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let handle = File::create(&out)?;
            if file.end == file.start {
                complete(handle, file, &out, progress);
            } else {
                self.current = Some((handle, file));
            }
        }
        Ok(())
    }

    fn write(&mut self, mut data: &[u8], progress: &Progress<'_>) -> Result<(), UnpackError> {
        while !data.is_empty() {
            self.open_ready(progress)?;
            let until = match &self.current {
                Some((_, file)) => file.end,
                None => self.files.get(self.next).map_or(u64::MAX, |f| f.start),
            };
            let n = usize::try_from(until - self.pos).map_or(data.len(), |n| n.min(data.len()));
            if let Some((handle, _)) = &mut self.current {
                handle.write_all(&data[..n])?;
            }
            self.pos += n as u64;
            data = &data[n..];
            if self
                .current
                .as_ref()
                .is_some_and(|(_, f)| f.end == self.pos)
            {
                let (handle, file) = self.current.take().expect("checked above");
                complete(handle, file, &self.dest.join(&file.path), progress);
            }
        }
        Ok(())
    }

    fn finish(mut self, progress: &Progress<'_>) -> Result<(), UnpackError> {
        self.open_ready(progress)?;
        let pending = self
            .current
            .as_ref()
            .map(|(_, f)| *f)
            .or(self.files.get(self.next).copied());
        match pending {
            Some(file) => Err(UnpackError::Cab(format!(
                "the cabinet data ends before {} is complete",
                file.path.display()
            ))),
            None => Ok(()),
        }
    }
}

/// Stamp the cabinet's date and time (see [`dos_time`]), close, report.
fn complete(handle: File, file: &PlannedFile, path: &Path, progress: &Progress<'_>) {
    dos_time::apply(&handle, file.date, file.time, path);
    drop(handle);
    // One report at a time, so `current` never goes backwards at the sink.
    let _turn = progress.gate.lock().unwrap_or_else(PoisonError::into_inner);
    let done = progress.done.fetch_add(1, Ordering::Relaxed) + 1;
    progress
        .sink
        .report("expanding cabinets", done, progress.total, path);
}
