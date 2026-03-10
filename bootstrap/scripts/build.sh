#!/usr/bin/env bash
# CMake configure + build wrapper.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

CONFIGURATION="${CIMMERIA_CONFIG:-Debug}"
PRESET="${CIMMERIA_PRESET:-}"

cmake_configure() {
    log_step "CONFIGURING CMAKE"

    if ! command -v cmake &>/dev/null; then
        die "CMake not found. Run 'make deps' first."
    fi

    local cmake_args=(-B "$BUILD_DIR")

    if [[ -n "$PRESET" ]]; then
        cmake_args=(--preset "$PRESET")
    else
        cmake_args+=(-DCMAKE_BUILD_TYPE="$CONFIGURATION")
        cmake_args+=(-DCMAKE_EXPORT_COMPILE_COMMANDS=ON)

        if [[ "$OS" == "macos" ]]; then
            cmake_args+=(-DOPENSSL_ROOT_DIR="$(brew --prefix openssl@3 2>/dev/null || echo '/opt/homebrew/opt/openssl@3')")
            cmake_args+=(-DPython3_ROOT_DIR="$(brew --prefix python@3.12 2>/dev/null || echo '/opt/homebrew/opt/python@3.12')")
            # libpq (PostgreSQL client library) is keg-only in Homebrew
            local pg_prefix
            pg_prefix="$(brew --prefix libpq 2>/dev/null)" || true
            if [[ -n "$pg_prefix" && -d "$pg_prefix/lib" ]]; then
                cmake_args+=(-DPostgreSQL_ROOT="$pg_prefix")
            fi
        fi
    fi

    log_status "cmake ${cmake_args[*]}"
    cmake "${cmake_args[@]}" "$PROJECT_ROOT"
    log_ok "CMake configured."
}

cmake_build() {
    log_step "BUILDING"

    if [[ ! -d "$BUILD_DIR" ]]; then
        cmake_configure
    fi

    local jobs
    if [[ "$OS" == "macos" ]]; then
        jobs=$(sysctl -n hw.ncpu)
    else
        jobs=$(nproc)
    fi

    log_status "Building with $jobs parallel jobs..."
    cmake --build "$BUILD_DIR" --parallel "$jobs"

    log_status "Verifying build output..."
    local all_found=true
    for exe in AuthenticationServer BaseApp CellApp NavBuilder; do
        if [[ -f "$BUILD_DIR/bin/$exe" ]] || [[ -f "$BUILD_DIR/bin/${exe}_d" ]]; then
            log_ok "  Found: $exe"
        else
            log_err "  Missing: $exe"
            all_found=false
        fi
    done

    if [[ "$all_found" != "true" ]]; then
        log_warn "Not all executables were produced. Check build output."
    fi

    log_ok "Build complete."
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-}" in
        configure) cmake_configure ;;
        build)     cmake_build ;;
        *)         cmake_build ;;
    esac
fi
