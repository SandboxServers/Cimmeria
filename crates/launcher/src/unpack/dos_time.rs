//! Archive DOS date/time stamps applied to extracted files.
//!
//! MakeCAB cabinets and zip entries store each file's modified time as an
//! MS-DOS date/time pair, in the local time of the machine that built the
//! archive. The 2009 client's cabinets date its files 2009-06-30, and the
//! game depends on that: Unreal Engine 3 records each `Default*.ini`
//! timestamp in the `[INIVersion]` section of the generated
//! `Documents\My Games\Firesky\SGWGame\Config\SGW*.ini`, and when a
//! `Default*.ini` mtime no longer matches, it asks on launch whether to
//! regenerate ("Your ini ... file is outdated"). An extractor that leaves
//! the extraction time on every file triggers that dialog for anyone who
//! has run SGW before.
//!
//! The conversion copies what Windows' own extractors (the stock installer,
//! `expand.exe`, `extrac32.exe`, the FDI SDK sample) do:
//! `DosDateTimeToFileTime`, then `LocalFileTimeToFileTime`. The second call
//! applies the time-zone bias in effect *now*, not the historical one for
//! the file's date, so a summer install and a winter install of the same
//! cabinet differ by the DST hour. Matching that quirk is the point: it is
//! what a stock install on the same machine would have produced.

use std::fs::{File, FileTimes};
use std::path::Path;
use std::time::SystemTime;

use tracing::{debug, warn};

/// The zip crate's stand-in when an entry records no time (1980-01-01
/// 00:00:00, the DOS epoch). Treated as "no timestamp".
const DOS_EPOCH_DATE: u16 = (1 << 5) | 1;

/// Convert a DOS date/time (local time) to UTC the way Windows' extractors
/// do. `None` for a stamp that is not a real date: a zero date, month 0 or
/// 13, day 0, hour 24, and so on.
pub(crate) fn to_system_time(date: u16, time: u16) -> Option<SystemTime> {
    if date == 0 {
        return None;
    }
    imp::to_system_time(date, time)
}

/// Set `file`'s modified and accessed times from an archive's DOS stamp.
/// Never fails the extraction: an invalid stamp leaves the extraction time
/// (logged at debug), and a failed `SetFileTime` is logged at warn.
pub(crate) fn apply(file: &File, date: u16, time: u16, path: &Path) {
    let Some(t) = to_system_time(date, time) else {
        debug!(
            path = %path.display(),
            dos_date = format_args!("{date:#06x}"),
            dos_time = format_args!("{time:#06x}"),
            "archive entry has no valid DOS timestamp; keeping the extraction time"
        );
        return;
    };
    if let Err(e) = file.set_times(FileTimes::new().set_modified(t).set_accessed(t)) {
        warn!(
            path = %path.display(),
            error = %e,
            "could not set the archived modified time on an extracted file"
        );
    }
}

/// [`apply`] for a zip entry's optional timestamp. A missing stamp, or the
/// DOS-epoch placeholder the zip crate writes when none was given, leaves
/// the extraction time.
pub(crate) fn apply_zip(file: &File, stamp: Option<::zip::DateTime>, path: &Path) {
    match stamp {
        Some(dt) if !(dt.datepart() == DOS_EPOCH_DATE && dt.timepart() == 0) => {
            apply(file, dt.datepart(), dt.timepart(), path)
        }
        _ => debug!(
            path = %path.display(),
            "zip entry has no timestamp; keeping the extraction time"
        ),
    }
}

#[cfg(windows)]
mod imp {
    use std::time::{Duration, SystemTime};

    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::Storage::FileSystem::LocalFileTimeToFileTime;
    use windows_sys::Win32::System::WindowsProgramming::DosDateTimeToFileTime;

    /// 100 ns ticks from 1601-01-01 (FILETIME's epoch) to 1970-01-01.
    const UNIX_EPOCH_TICKS: u64 = 116_444_736_000_000_000;

    pub(super) fn to_system_time(date: u16, time: u16) -> Option<SystemTime> {
        let mut local = FILETIME::default();
        let mut utc = FILETIME::default();
        // SAFETY: both out-pointers are valid, writable FILETIMEs.
        let ok = unsafe {
            DosDateTimeToFileTime(date, time, &mut local) != 0
                && LocalFileTimeToFileTime(&local, &mut utc) != 0
        };
        if !ok {
            return None;
        }
        let ticks = (u64::from(utc.dwHighDateTime) << 32) | u64::from(utc.dwLowDateTime);
        let since_unix = ticks.checked_sub(UNIX_EPOCH_TICKS)?;
        SystemTime::UNIX_EPOCH.checked_add(Duration::new(
            since_unix / 10_000_000,
            (since_unix % 10_000_000) as u32 * 100,
        ))
    }
}

// The launcher ships for Windows only; this keeps the crate building on
// other hosts. chrono's `Local` applies the historical offset for the date,
// which can differ from Windows' current-bias rule by the DST hour.
#[cfg(not(windows))]
mod imp {
    use std::time::SystemTime;

    use chrono::{Local, NaiveDate, TimeZone};

    pub(super) fn to_system_time(date: u16, time: u16) -> Option<SystemTime> {
        let day = NaiveDate::from_ymd_opt(
            1980 + i32::from(date >> 9),
            u32::from((date >> 5) & 0x0f),
            u32::from(date & 0x1f),
        )?;
        let at = day.and_hms_opt(
            u32::from(time >> 11),
            u32::from((time >> 5) & 0x3f),
            u32::from(time & 0x1f) * 2,
        )?;
        Local
            .from_local_datetime(&at)
            .earliest()
            .map(SystemTime::from)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{FIXTURE_DOS_DATE, FIXTURE_DOS_TIME};
    use super::*;

    #[test]
    fn invalid_stamps_are_none() {
        assert_eq!(to_system_time(0, 0), None);
        // Month 13.
        assert_eq!(
            to_system_time(((2009 - 1980) << 9) | (13 << 5) | 1, 0),
            None
        );
        // Day 0.
        assert_eq!(to_system_time(((2009 - 1980) << 9) | (6 << 5), 0), None);
    }

    // The value must be the local wall-clock time the stamp names, which is
    // what Explorer and UE3 read back.
    #[test]
    fn a_valid_stamp_round_trips_to_local_wall_clock_time() {
        let t = to_system_time(FIXTURE_DOS_DATE, FIXTURE_DOS_TIME).expect("valid stamp");
        let local = chrono::DateTime::<chrono::Local>::from(t);
        assert_eq!(
            local.format("%Y-%m-%d").to_string(),
            "2009-06-30",
            "{local}"
        );
        // The hour can shift by the DST hour when the test runs in the other
        // DST season (current-bias rule, see the module docs); minutes and
        // seconds cannot.
        assert!(
            ["11:00:00", "12:00:00", "13:00:00"]
                .contains(&local.format("%H:%M:%S").to_string().as_str()),
            "{local}"
        );
    }

    #[test]
    fn apply_sets_modified_and_leaves_invalid_alone() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.ini");
        let f = File::create(&p).unwrap();
        let before = f.metadata().unwrap().modified().unwrap();
        apply(&f, 0, 0, &p);
        assert_eq!(f.metadata().unwrap().modified().unwrap(), before);
        apply(&f, FIXTURE_DOS_DATE, FIXTURE_DOS_TIME, &p);
        drop(f);
        assert_eq!(
            std::fs::metadata(&p).unwrap().modified().unwrap(),
            to_system_time(FIXTURE_DOS_DATE, FIXTURE_DOS_TIME).unwrap()
        );
    }
}
