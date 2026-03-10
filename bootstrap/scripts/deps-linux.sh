#!/usr/bin/env bash
# Install Cimmeria dependencies via apt (Ubuntu/Debian).
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

install_deps_linux() {
    log_step "INSTALLING DEPENDENCIES (Linux / apt)"

    if ! command -v apt-get &>/dev/null; then
        die "apt-get not found. This script supports Ubuntu/Debian. For other distros, install equivalent packages manually."
    fi

    log_status "Updating package lists..."
    sudo apt-get update

    local packages=(
        cmake
        build-essential
        libboost-all-dev
        libssl-dev
        python3-dev
        libpq-dev
        libsdl1.2-dev
        qtbase5-dev
        docker.io
        docker-compose-v2
    )

    log_status "Installing packages: ${packages[*]}"
    sudo apt-get install -y "${packages[@]}"

    log_ok "Dependencies installed."
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_deps_linux
fi
