#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only

set -euo pipefail

worker_entry="apps/worker/build/worker/shim.mjs"

# Workers Builds may run the package build command before Wrangler. Avoid
# compiling twice when Wrangler subsequently invokes the custom build command.
if [[ -f "$worker_entry" ]] &&
  [[ -z "$(find \
    Cargo.toml Cargo.lock rust-toolchain.toml \
    apps/worker/Cargo.toml apps/worker/src \
    crates/core/Cargo.toml crates/core/src \
    -type f -newer "$worker_entry" -print -quit)" ]]; then
  echo "Rust Worker build is already up to date."
  exit 0
fi

task_build_cache="${FORUM_BUILD_CACHE:-${PWD}/.build-cache}"
task_cargo_home="${CARGO_HOME:-${task_build_cache}/cargo}"
task_rustup_home="${RUSTUP_HOME:-${task_build_cache}/rustup}"
task_cache_home="${XDG_CACHE_HOME:-${task_build_cache}/xdg}"
export CARGO_HOME="$task_cargo_home"
export RUSTUP_HOME="$task_rustup_home"
export XDG_CACHE_HOME="$task_cache_home"
export PATH="$CARGO_HOME/bin:$PATH"

mkdir -p "$CARGO_HOME" "$RUSTUP_HOME" "$XDG_CACHE_HOME"

if ! command -v cargo >/dev/null 2>&1; then
  if ! command -v curl >/dev/null 2>&1; then
    echo "Rust is unavailable and curl is required to install rustup." >&2
    exit 1
  fi

  echo "Rust is not present in this build image; installing the minimal stable toolchain."
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error \
    https://sh.rustup.rs | sh -s -- -y --no-modify-path \
      --profile minimal --default-toolchain stable
fi

rustup target add wasm32-unknown-unknown

# Some minimal CI images ship OpenSSL headers and libraries without pkg-config.
# Point openssl-sys at those existing files instead of requiring root access to
# install additional operating-system packages.
if ! command -v pkg-config >/dev/null 2>&1 && [[ -d /usr/include/openssl ]]; then
  task_multiarch="$(gcc -print-multiarch 2>/dev/null || true)"
  if [[ -n "$task_multiarch" ]] && [[ -d "/usr/lib/$task_multiarch" ]]; then
    export OPENSSL_INCLUDE_DIR=/usr/include
    export OPENSSL_LIB_DIR="/usr/lib/$task_multiarch"
  fi
fi

cargo install worker-build --version 0.8.1 --locked

(
  cd apps/worker
  worker-build --release
)
