# Self-hosted GitHub Actions runner for Cimmeria's Linux CI jobs.
#
# Ubuntu 24.04 to match `ubuntu-latest` (Python 3.12, the same apt package names),
# with what the test.yml jobs expect a GitHub-hosted image to already have baked in:
# git, gh, curl/unzip, python3, a C/C++ toolchain, clang + mold, lua5.1, psql 17,
# the docker CLI (service containers run on the paired dind sidecar), rustup with the
# pinned toolchain, and sccache. Jobs still run their own `apt-get install` and
# rust-toolchain steps; here those are fast no-ops.
FROM ubuntu:24.04

ARG RUNNER_VERSION=2.338.0
ARG RUST_VERSION=1.98.1
ARG SCCACHE_VERSION=0.18.0

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl wget gnupg lsb-release sudo tini \
        git git-lfs jq unzip zip xz-utils zstd bzip2 file \
        build-essential clang lld mold cmake ninja-build pkg-config \
        libssl-dev libdbus-1-dev libsqlite3-dev libudev-dev \
        python3 python3-pip python3-venv python-is-python3 \
        lua5.1 \
    && install -d -m 0755 /etc/apt/keyrings \
    # psql 17, matching the postgres:17.9 service container.
    && curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc \
        | gpg --dearmor -o /etc/apt/keyrings/pgdg.gpg \
    && echo "deb [signed-by=/etc/apt/keyrings/pgdg.gpg] https://apt.postgresql.org/pub/repos/apt noble-pgdg main" \
        > /etc/apt/sources.list.d/pgdg.list \
    # docker CLI + buildx/compose plugins; the daemon is the dind sidecar.
    && curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc \
    && echo "deb [signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu noble stable" \
        > /etc/apt/sources.list.d/docker.list \
    # gh, used by the `changes` job.
    && curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
        -o /etc/apt/keyrings/githubcli.gpg \
    && echo "deb [signed-by=/etc/apt/keyrings/githubcli.gpg] https://cli.github.com/packages stable main" \
        > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update && apt-get install -y --no-install-recommends \
        postgresql-client-17 docker-ce-cli docker-buildx-plugin docker-compose-plugin gh \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
        | tar -xz -C /usr/local/bin --strip-components=1 --wildcards '*/sccache' \
    && chmod 0755 /usr/local/bin/sccache

# The runner user: passwordless sudo, like the hosted `runner` account. uid/gid 1001
# is fixed because compose.yml hands the dind socket to gid 1001.
RUN useradd -m -s /bin/bash -u 1001 -U runner \
    && echo 'runner ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/runner \
    && chmod 0440 /etc/sudoers.d/runner

WORKDIR /home/runner/actions-runner
RUN curl -fsSL "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" \
        | tar -xz \
    && ./bin/installdependencies.sh \
    && rm -rf /var/lib/apt/lists/* \
    && chown -R runner:runner /home/runner

USER runner
ENV CARGO_HOME=/home/runner/.cargo \
    RUSTUP_HOME=/home/runner/.rustup \
    PATH=/home/runner/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "${RUST_VERSION}" \
        --component rustfmt,clippy,llvm-tools-preview \
    && rm -rf "${RUSTUP_HOME}/downloads" "${RUSTUP_HOME}/tmp" \
    # Mount points for the per-runner volumes; created here so a fresh named
    # volume inherits runner ownership.
    && mkdir -p /home/runner/_work /home/runner/state /home/runner/sccache \
        "${CARGO_HOME}/registry" "${CARGO_HOME}/git"

COPY --chown=runner:runner entrypoint.sh /home/runner/entrypoint.sh
COPY --chown=runner:runner hooks/ /home/runner/hooks/
RUN chmod 0755 /home/runner/entrypoint.sh /home/runner/hooks/*.sh

# -g: deliver stop signals to the whole process group. Without it only run.sh (bash)
# gets SIGTERM, the listener is killed without closing its GitHub session, and the
# next container retries on "A session for this runner already exists" for minutes.
ENTRYPOINT ["/usr/bin/tini", "-g", "--", "/home/runner/entrypoint.sh"]
