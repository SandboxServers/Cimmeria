#!/usr/bin/env bash
# Verify runtime library resolution for built executables.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

verify_runtime() {
    log_step "VERIFYING RUNTIME LIBRARIES"

    local bin_dir="$BUILD_DIR/bin"
    if [[ ! -d "$bin_dir" ]]; then
        die "Build directory not found: $bin_dir. Run 'make build' first."
    fi

    local all_ok=true

    for exe in "$bin_dir"/*; do
        [[ -x "$exe" ]] || continue
        [[ -f "$exe" ]] || continue
        local name
        name=$(basename "$exe")

        log_status "Checking $name..."

        if [[ "$OS" == "macos" ]]; then
            local missing
            missing=$(otool -L "$exe" 2>/dev/null | grep "not found" || true)
            if [[ -n "$missing" ]]; then
                log_err "  Missing libraries:"
                echo "$missing" | while read -r line; do log_err "    $line"; done
                all_ok=false
            else
                log_ok "  $name: all libraries resolved."
            fi
        elif [[ "$OS" == "linux" ]]; then
            local missing
            missing=$(ldd "$exe" 2>/dev/null | grep "not found" || true)
            if [[ -n "$missing" ]]; then
                log_err "  Missing libraries:"
                echo "$missing" | while read -r line; do log_err "    $line"; done
                all_ok=false
            else
                log_ok "  $name: all libraries resolved."
            fi
        fi
    done

    if [[ "$all_ok" == "true" ]]; then
        log_ok "All runtime libraries resolved."
    else
        die "Some libraries are missing. Check output above."
    fi
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    verify_runtime
fi
