# Dependency Patches for VS2015+ (MSVC v140+)

These files are patched versions of vendored dependency source files that must be
applied after extracting the original archives. The `setup-dependencies.ps1` script
applies them automatically.

Each patch fixes compilation errors that occur when building 2012-era libraries
with modern MSVC toolsets (VS2015 and later).

## Patch Files

### 1. `boost_auto_link.hpp`

- **Target:** `external/boost/boost/config/auto_link.hpp`
- **Original:** Boost 1.55.0 (December 2013)
- **Problem:** Boost 1.55.0 only knows MSVC toolsets through `vc120` (VS2012).
  When compiling with VS2015+ the auto-link `#pragma comment(lib, ...)` requests
  `libboost_*-vc120-*.lib` but our libs are built with a `vc140`/`vc145` tag.
  This causes LNK1104 "cannot open file" errors at link time.
- **Fix:** Adds `vc140` (VS2015, `_MSC_VER < 1910`) and `vc145` (VS2017+, catch-all)
  entries to the `#elif` chain that maps `_MSC_VER` ranges to toolset name strings.
- **Breaking MSVC version:** VS2015 (v140, `_MSC_VER` 1900)

### 2. `soci_platform.h`

- **Target:** `external/soci/src/core/soci-platform.h`
- **Original:** SOCI 3.2.1 (April 2013)
- **Problem:** Two issues:
  1. Unconditional `#define snprintf _snprintf` fails on VS2015+ which provides
     `snprintf` natively per C99/C11. Results in "macro redefinition" errors.
  2. `strtoll`/`strtoull` wrappers around `_strtoi64`/`_strtoui64` cause
     "function already defined" errors on VS2013+ which provides them natively.
- **Fix:** Adds `_MSC_VER` version guards:
  - `snprintf` macro only defined when `_MSC_VER < 1900`
  - `strtoll`/`strtoull` wrappers only defined when `_MSC_VER < 1800`
- **Breaking MSVC version:** VS2013 (v120, `_MSC_VER` 1800) for strtoll;
  VS2015 (v140, `_MSC_VER` 1900) for snprintf

### 3. `openssl_e_os.h`

- **Target:** `external/openssl_src/e_os.h`
- **Original:** OpenSSL 1.0.1e (February 2013)
- **Problem:** Redefines `stdin`, `stdout`, `stderr` using `__iob_func()` which
  was removed in the VS2015 Universal CRT (UCRT). The UCRT replaced it with
  `__acrt_iob_func()` as inline functions. The old redefinition causes
  "identifier not found" errors on VS2015+.
- **Fix:** Adds `_MSC_VER >= 1900` guard to skip the `stdin`/`stdout`/`stderr`
  redefinition block on modern MSVC, where the UCRT handles these correctly.
- **Breaking MSVC version:** VS2015 (v140, `_MSC_VER` 1900)

### 4. `openssl_e_padlock.c`

- **Target:** `external/openssl_src/engines/e_padlock.c`
- **Original:** OpenSSL 1.0.1e (February 2013)
- **Problem:** Contains `_asm` inline assembly blocks for VIA PadLock hardware
  crypto acceleration. 64-bit MSVC (`_M_X64`) does not support inline assembly
  at all — only 32-bit MSVC (`_M_IX86`) does. Compiling for x64 produces
  "inline assembler not supported" errors.
- **Fix:** Adds `_M_IX86` guard to the `#if defined(_MSC_VER)` MSVC asm section
  so the inline assembly code is only compiled for x86 targets. x64 builds
  correctly skip the PadLock engine (it's x86-only hardware anyway).
- **Breaking MSVC version:** Any MSVC targeting x64 (the original code was
  never tested with x64 builds)
