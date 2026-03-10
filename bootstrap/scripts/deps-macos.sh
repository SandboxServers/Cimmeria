#!/usr/bin/env bash
# Install Cimmeria dependencies via Homebrew (macOS).
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

install_deps_macos() {
    log_step "INSTALLING DEPENDENCIES (macOS / Homebrew)"

    if ! command -v brew &>/dev/null; then
        die "Homebrew not found. Install from https://brew.sh"
    fi

    log_status "Updating Homebrew..."
    brew update

    local packages=(
        cmake
        boost
        openssl@3
        python@3.12
        postgresql@17
        sdl12-compat
    )

    log_status "Installing packages: ${packages[*]}"
    brew install "${packages[@]}"

    # Qt 5 — try qt@5, fall back to qt
    if brew list qt@5 &>/dev/null || brew install qt@5 2>/dev/null; then
        log_ok "Qt 5 installed."
    else
        log_warn "qt@5 not available, installing qt (6.x). ServerEd may need adjustments."
        brew install qt
    fi

    # Ensure postgresql@17 tools are on PATH
    if ! command -v psql &>/dev/null; then
        log_warn "psql not on PATH. Add to your shell profile:"
        log_warn "  export PATH=\"/opt/homebrew/opt/postgresql@17/bin:\$PATH\""
    fi

    # Print summary
    log_ok "Dependencies installed."
    log_status "Boost:    $(brew info --json=v2 boost 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["formulae"][0]["versions"]["stable"])' 2>/dev/null || echo 'installed')" "$GRAY"
    log_status "OpenSSL:  $(openssl version 2>/dev/null || echo 'installed')" "$GRAY"
    log_status "Python:   $(python3 --version 2>/dev/null || echo 'installed')" "$GRAY"
    log_status "CMake:    $(cmake --version 2>/dev/null | head -1 || echo 'installed')" "$GRAY"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_deps_macos
fi
