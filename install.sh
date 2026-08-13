#!/usr/bin/env bash
#
# TypeMaster installer — downloads the prebuilt binary for the current OS and
# architecture from the latest GitHub release and installs it.
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/t43void/touchwiz/main/install.sh | bash
#   PREFIX=/opt/typemaster ./install.sh
#

set -euo pipefail

REPO="t43void/touchwiz"
PREFIX="${PREFIX:-$HOME/.local/bin}"
BIN_NAME="typemaster"

die() {
  echo "error: $*" >&2
  exit 1
}

warn() {
  echo "warning: $*" >&2
}

url_for_target() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$os" in
    linux)
      case "$arch" in
        x86_64 | amd64) echo "typemaster-x86_64-unknown-linux-gnu.tar.gz" ;;
        *) die "no prebuilt binary for linux/$arch; build from source (see README)" ;;
      esac
      ;;
    darwin)
      case "$arch" in
        x86_64) echo "typemaster-x86_64-apple-darwin.tar.gz" ;;
        arm64) echo "typemaster-aarch64-apple-darwin.tar.gz" ;;
        *) die "no prebuilt binary for darwin/$arch; build from source (see README)" ;;
      esac
      ;;
    *) die "unsupported operating system: $os (Windows: grab the zip from Releases)" ;;
  esac
}

main() {
  local asset url tmp
  asset="$(url_for_target)"
  url="https://github.com/${REPO}/releases/latest/download/${asset}"

  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT

  echo "Downloading ${asset} ..."
  curl -fL --proto '=https' --tlsv1.2 -o "${tmp}/${asset}" "${url}" \
    || die "download failed (is there a release with ${asset}?)"

  mkdir -p "$PREFIX"
  tar -xzf "${tmp}/${asset}" -C "$tmp"
  install -m 0755 "${tmp}/${BIN_NAME}" "${PREFIX}/${BIN_NAME}"

  echo "Installed ${BIN_NAME} to ${PREFIX}"

  if [[ ":$PATH:" != *":${PREFIX}:"* ]]; then
    warn "${PREFIX} is not on your PATH; add it, or run ${PREFIX}/${BIN_NAME} directly"
  fi
}

main "$@"
