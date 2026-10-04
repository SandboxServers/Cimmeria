//! Real pinned Wine, isolated: a copy of the runtime, a throwaway prefix, Wine's own
//! Notepad and a bundle labelled as a fixture. Never the game, the saved installation,
//! its prefix or the game's bundle identifier.
use super::*;
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const FIXTURE: Identity = Identity {
    identifier: "app.cimmeria.fixture.wine-identity",
    name: "Cimmeria Wine Identity Fixture",
};

#[derive(Debug)]
struct Application {
    pid: i32,
    kind: String,
    identifier: Option<String>,
    executable: PathBuf,
    /// Wine retitles each process to its Windows command line.
    command: String,
}

fn output(program: &str, arguments: &[&str]) -> String {
    let output = Command::new(program).args(arguments).output().unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Launch Services' running applications whose executable is under `root`.
fn applications(root: &Path) -> Vec<Application> {
    let quoted = |line: &str| line.split('"').nth(1).map(str::to_owned);
    let (mut identifier, mut executable, mut found) = (None, None, Vec::new());
    for line in output("/usr/bin/lsappinfo", &["list"]).lines() {
        if !line.starts_with(' ') {
            (identifier, executable) = (None, None);
        }
        let line = line.trim();
        if line.starts_with("bundleID=") {
            identifier = quoted(line);
        } else if line.starts_with("executable path=") {
            executable = quoted(line).map(PathBuf::from);
        } else if let Some(rest) = line.strip_prefix("pid = ") {
            let pid = rest.split_whitespace().next().unwrap_or_default();
            let kind = rest
                .split("type=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next());
            if let Some(executable) = executable.take().filter(|path| path.starts_with(root)) {
                found.push(Application {
                    pid: pid.parse().unwrap(),
                    kind: kind.unwrap_or("unknown").to_owned(),
                    identifier: identifier.take(),
                    executable,
                    command: output("/bin/ps", &["-o", "args=", "-p", pid]),
                });
            }
        }
    }
    found
}

mod window_server {
    use std::ffi::c_void;
    type Ref = *const c_void;
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub(super) struct Bounds {
        x: f64,
        y: f64,
        pub(super) width: f64,
        pub(super) height: f64,
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        static kCGWindowOwnerPID: Ref;
        static kCGWindowLayer: Ref;
        static kCGWindowBounds: Ref;
        fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> Ref;
        fn CGRectMakeWithDictionaryRepresentation(dictionary: Ref, rect: *mut Bounds) -> bool;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFArrayGetCount(array: Ref) -> isize;
        fn CFArrayGetValueAtIndex(array: Ref, index: isize) -> Ref;
        fn CFDictionaryGetValue(dictionary: Ref, key: Ref) -> Ref;
        fn CFNumberGetValue(number: Ref, kind: isize, value: *mut i32) -> bool;
        fn CFRelease(object: Ref);
    }
    const ALL_WINDOWS: u32 = 0;
    const SINT32: isize = 3;

    /// Bounds of the ordinary-layer windows `pid` owns, on any Space. Read-only, and
    /// needs no permission: titles and contents are not read.
    pub(super) fn windows(pid: i32) -> Vec<Bounds> {
        let mut found = Vec::new();
        // SAFETY: the array is owned by this call and released once; its elements are
        // dictionaries borrowed only while it lives, and a missing key is checked
        // before use. Each out-pointer is a live local of the type the call writes.
        unsafe {
            let list = CGWindowListCopyWindowInfo(ALL_WINDOWS, 0);
            if list.is_null() {
                return found;
            }
            for index in 0..CFArrayGetCount(list) {
                let window = CFArrayGetValueAtIndex(list, index);
                let number = |key: Ref| {
                    let (value, mut number) = (CFDictionaryGetValue(window, key), 0);
                    (!value.is_null() && CFNumberGetValue(value, SINT32, &mut number))
                        .then_some(number)
                };
                let bounds = CFDictionaryGetValue(window, kCGWindowBounds);
                let mut rect = Bounds::default();
                if number(kCGWindowOwnerPID) == Some(pid)
                    && number(kCGWindowLayer) == Some(0)
                    && !bounds.is_null()
                    && CGRectMakeWithDictionaryRepresentation(bounds, &mut rect)
                {
                    found.push(rect);
                }
            }
            CFRelease(list);
        }
        found
    }
}

fn ended(child: &mut Child, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    while matches!(child.try_wait(), Ok(None)) {
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    true
}

#[test]
#[ignore = "runs real Wine from CIMMERIA_WINE_RUNTIME_CLONE and shows a Notepad window"]
fn real_wine_window_owner_alone_carries_the_bundle_identity() {
    let runtime = std::env::var_os("CIMMERIA_WINE_RUNTIME_CLONE")
        .map(PathBuf::from)
        .expect("CIMMERIA_WINE_RUNTIME_CLONE: a copy of the managed runtime directory")
        .canonicalize()
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let bundle = stage(&root, &runtime, &FIXTURE).unwrap();
    let prefix = root.join("home/bottle");
    fs::create_dir_all(&prefix).unwrap();
    let mut environment = environment(&runtime, &prefix).unwrap();
    // The interactive policy: Wine's Mac driver stays enabled, as in Play.
    environment.insert(
        "WINEDLLOVERRIDES".into(),
        "winemenubuilder.exe,mscoree,mshtml=d".into(),
    );
    let wine = |program: PathBuf, arguments: &[&str]| {
        Command::new(program)
            .args(arguments)
            .env_clear()
            .envs(&environment)
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    };
    let stock = runtime.join("bin/wine");
    let server = runtime.join("bin/wineserver");
    // Play always finds an existing prefix. Creating one shows a progress window,
    // which would start the stock desktop host whatever the keeper did.
    assert!(ended(
        &mut wine(stock.clone(), &["wineboot", "--init"]),
        Duration::from_secs(120)
    ));
    assert!(ended(
        &mut wine(server.clone(), &["-w"]),
        Duration::from_secs(60)
    ));

    // Production order: the stock desktop host first, then the staged loader.
    let host = DesktopHost::start(&stock, &environment, &|| false, desktop_host::LIMITS).unwrap();
    // A 32-bit parent starting a 32-bit child, the shape of the launch worker and SGW.exe.
    let mut guest = wine(
        bundle.join("Contents/MacOS/wine"),
        &[
            r"C:\windows\syswow64\cmd.exe",
            "/c",
            "start",
            "/wait",
            r"C:\windows\syswow64\notepad.exe",
        ],
    );
    let is_notepad = |application: &Application| {
        application.kind == "Foreground" && application.command.contains(r"syswow64\notepad.exe")
    };
    let deadline = Instant::now() + Duration::from_secs(120);
    let all_windows = |applications: &[Application]| {
        let windows = |application: &Application| window_server::windows(application.pid);
        applications.iter().flat_map(windows).collect::<Vec<_>>()
    };
    let (staged, hosts, windows, hidden) = loop {
        let found = applications(&bundle).iter().any(is_notepad);
        if found || Instant::now() > deadline {
            // Let a late desktop host from the bundle show up before counting.
            std::thread::sleep(Duration::from_secs(3));
            let (staged, hosts) = (applications(&bundle), applications(&runtime));
            let (windows, hidden) = (all_windows(&staged), all_windows(&hosts));
            break (staged, hosts, windows, hidden);
        }
        std::thread::sleep(Duration::from_millis(500));
    };

    // Ask Notepad to close, release the keeper, and let Wine end the session itself.
    let closed = ended(
        &mut wine(stock.clone(), &["taskkill", "/im", "notepad.exe"]),
        Duration::from_secs(30),
    ) && ended(&mut guest, Duration::from_secs(30));
    drop(host);
    let ended_alone = ended(&mut wine(server.clone(), &["-w"]), Duration::from_secs(30));
    // Only this prefix's session, whatever happened above.
    wine(server, &["-k"]).wait().unwrap();
    let _ = guest.wait();
    // Launch Services records a bundle once a process from it checks in.
    let _ = Command::new(LSREGISTER).arg("-u").arg(&bundle).status();
    std::thread::sleep(Duration::from_secs(1));
    let left = (applications(&bundle), applications(&runtime));

    // Exactly one running application has the identifier, and it owns the window.
    assert_eq!(staged.len(), 1, "{staged:?}");
    assert!(is_notepad(&staged[0]), "{staged:?}");
    assert_eq!(staged[0].identifier.as_deref(), Some(FIXTURE.identifier));
    assert_eq!(staged[0].executable, bundle.join("Contents/MacOS/wine"));
    // Every Wine process owns one small hidden window; the desktop host has only that.
    assert!(
        windows.iter().any(|window| window.width >= 200.0
            && window.height >= 200.0
            && !hidden.contains(window)),
        "{windows:?} {hidden:?}"
    );
    // The windowless desktop host runs from the stock loader, without the identifier.
    let desktop = hosts
        .iter()
        .find(|application| application.command.contains(r"explorer.exe /desktop"));
    let desktop = desktop.unwrap_or_else(|| panic!("no stock desktop host: {hosts:?}"));
    assert_eq!(desktop.identifier, None, "{desktop:?}");
    assert_ne!(desktop.kind, "Foreground", "{desktop:?}");
    // Nothing is left to stop: the fixture closed, and the session followed by itself.
    assert!(closed, "the fixture Notepad did not close");
    assert!(ended_alone, "the Wine session outlived its last user");
    assert!(left.0.is_empty() && left.1.is_empty(), "{left:?}");
}
