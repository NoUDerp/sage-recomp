#!/usr/bin/env bash
# sage-recomp coordinator helper (project owner only): reference build, golden
# minting, repository checks, task publication, and the foreground polling loop.
# Usage: scripts/coordinator.sh <subcommand> [args]   (runnable from any directory)

set -euo pipefail

# The framework is vendored in this repository (framework/, see UPSTREAM.md):
# the coordinator runs the vendored tree, contributors install the same tree
# from this repository at the commit named by [toolchain].framework_rev.
FRAMEWORK_REPO="https://github.com/pickles976/sage-recomp.git"
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

say() { printf '%s\n' "$*"; }
step() { printf '\n== %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

usage() {
  cat <<'EOF'
sage-recomp coordinator helper (owner only)

usage: scripts/coordinator.sh <subcommand> [args]

  setup [--yes] [--force]  install the contributor set plus the minting extras
                           (cmake, ninja-build, g++, ccache)
  reference                clone and build the pinned upstream core into .cache/
  mint [task...]           mint goldens for the given task ids (default: all)
  check                    re-mint-check every golden, run the plugin selftest
                           and cargo test
  image [--push]           build the digest-pinned toolchain image from the
                           plugin, the guest commands and the pinned framework
  rootfs                   build the Firecracker guest rootfs from that image
  publish                  publish the task index as GitHub Issues
  serve [args]             run the foreground polling loop (one sudo prompt)
  doctor                   check owner prerequisites and the project gate
  help                     print this list

Git LFS is not needed anywhere in this project.
EOF
}

refuse_root() {
  [ "$(id -u)" -ne 0 ] || die "do not run this script as root; it asks for sudo only when needed"
}

need_recomp() {
  have recomp || die "recomp is not installed or not on PATH; run: scripts/coordinator.sh setup"
}

python_ok() {
  have python3 && python3 -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 11) else 1)'
}

github_field() {
  python3 - "$ROOT/recomp.toml" "$1" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as fh:
    cfg = tomllib.load(fh).get("github", {})
value = cfg.get(sys.argv[2], "")
if not value:
    raise SystemExit(f"recomp.toml has no [github].{sys.argv[2]}")
print(value)
PY
}

confirm() {
  local prompt="$1"
  if [ "${ASSUME_YES:-0}" = "1" ]; then
    say "$prompt yes (--yes)"
    return 0
  fi
  if [ ! -t 0 ]; then
    die "no terminal to confirm on; re-run with --yes to accept the plan above"
  fi
  local reply=""
  read -r -p "$prompt [y/N] " reply
  case "$reply" in
    y | Y | yes | YES) return 0 ;;
    *) die "aborted; nothing was installed" ;;
  esac
}

apt_install() {
  [ "$#" -gt 0 ] || return 0
  if ! have apt-get; then
    say "this host has no apt-get; install these packages yourself: $*"
    return 0
  fi
  step "Packages to install with sudo"
  say "  sudo apt-get update"
  say "  sudo apt-get install -y $*"
  confirm "Run the two commands above?"
  sudo apt-get update
  sudo apt-get install -y "$@"
}

missing_packages() {
  local -n out="$1"
  shift
  out=()
  have git || out+=(git)
  python_ok || out+=(python3 python3-venv)
  have pip3 || have pipx || out+=(python3-pip)
  have pipx || out+=(pipx)
  have curl || out+=(curl)
  if ! have docker && ! have podman; then
    out+=(podman)
  fi
  have cmake || out+=(cmake)
  have ninja || out+=(ninja-build)
  have g++ || out+=(g++)
  have ccache || out+=(ccache)
}

install_framework() {
  local force="$1"
  local spec="git+${FRAMEWORK_REPO}@$(python3 -c 'import tomllib,pathlib;print(tomllib.loads(pathlib.Path("recomp.toml").read_text())["toolchain"]["framework_rev"])')#subdirectory=framework"
  if have recomp && [ "$force" != "1" ]; then
    say "recomp already installed at $(command -v recomp); re-run with --force to reinstall"
    return 0
  fi
  step "Installing Recomp@Home from this repository (vendored framework)"
  if have pipx; then
    pipx install --force "$spec" && return 0
    say "pipx install failed; falling back to pip"
  fi
  python3 -m pip install --user "$spec" && return 0
  say "pip --user failed (managed environment?); retrying with --break-system-packages"
  python3 -m pip install --user --break-system-packages "$spec"
}

install_mulch() {
  if have mulch; then
    say "mulch already installed at $(command -v mulch)"
    return 0
  fi
  step "Installing mulch (contribution context records)"
  if have bun; then
    bun install -g @os-eco/mulch-cli && return 0
    say "bun install failed; falling back to npm"
  fi
  if have npm; then
    npm install -g @os-eco/mulch-cli && return 0
  fi
  say "WARNING: neither bun nor npm is available; install mulch manually:"
  say "  bun install -g @os-eco/mulch-cli   # or: npm install -g @os-eco/mulch-cli"
  say "  see CONTRIBUTING.md § Recording context for the coordinator"
}

cmd_setup() {
  local force=0
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --yes | -y) ASSUME_YES=1 ;;
      --force) force=1 ;;
      *) die "setup: unknown option $1" ;;
    esac
    shift
  done

  step "Plan"
  say "  1. contributor set: git, Python 3.11+, Docker or Podman"
  say "  2. mulch (context records): bun/npm install -g @os-eco/mulch-cli"
  say "  3. owner-only minting extras: cmake, ninja-build, g++, ccache"
  say "     (git-lfs is deliberately NOT installed; this project never uses it)"
  say "  4. pip/pipx install the vendored framework at [toolchain].framework_rev:
       python3 -m pip install \"git+${FRAMEWORK_REPO}@<framework_rev>#subdirectory=framework\""
  say "Already-present tools are left untouched."

  local pkgs=()
  missing_packages pkgs
  if [ "${#pkgs[@]}" -eq 0 ]; then
    say ""
    say "all system packages are already present"
  else
    apt_install "${pkgs[@]}"
  fi

  python_ok || die "Python 3.11 or newer is required; this host has $(python3 -V 2>&1 || echo 'no python3')"
  install_framework "$force"
  install_mulch
  say ""
  say "next: scripts/coordinator.sh reference"
}

cmd_reference() {
  python_ok || die "Python 3.11 or newer is required"
  for tool in cmake ninja g++ git; do
    have "$tool" || die "$tool is required to build the reference; run: scripts/coordinator.sh setup"
  done
  step "Reference build"
  say "Cloning and building the pinned upstream core (openbfme_core, headless)."
  say "The first build takes about 20 minutes on 8 cores."
  say "Everything lands in .cache/, which is reused between sessions and is"
  say "never committed; a cached build is detected and skipped."
  cd "$ROOT"
  python3 - "$ROOT" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
sys.path.insert(0, str(root / "tools"))
import mint_goldens

config = mint_goldens.upstream_config()
mint_goldens.ensure_checkout(config)
mint_goldens.ensure_build(config)
PY
  say ""
  say "reference ready: .cache/reference/build/libopenbfme_core.a"
}

cmd_mint() {
  python_ok || die "Python 3.11 or newer is required"
  cd "$ROOT"
  if [ "$#" -eq 0 ]; then
    step "Minting every task in plugins/sage-port/task-index.json"
    python3 tools/mint_goldens.py --all
    return
  fi
  local args=()
  local task
  for task in "$@"; do
    args+=(--task "$task")
  done
  step "Minting $*"
  python3 tools/mint_goldens.py "${args[@]}"
}

cmd_check() {
  python_ok || die "Python 3.11 or newer is required"
  cd "$ROOT"
  local failed=()

  step "goldens reproduce byte-for-byte"
  if python3 tools/mint_goldens.py --all --check; then
    say "ok: tools/mint_goldens.py --all --check"
  else
    failed+=("tools/mint_goldens.py --all --check")
  fi

  step "plugin selftest"
  if python3 plugins/sage-port/runner.py selftest <<< '{}'; then
    say "ok: plugins/sage-port/runner.py selftest"
  else
    failed+=("plugins/sage-port/runner.py selftest")
  fi

  step "cargo test"
  if have cargo; then
    if (cd recovery && cargo test); then
      say "ok: cargo test"
    else
      failed+=("cargo test (recovery/)")
    fi
  else
    failed+=("cargo test (recovery/): cargo not installed")
  fi

  step "Summary"
  if [ "${#failed[@]}" -eq 0 ]; then
    say "all checks passed"
    return 0
  fi
  local item
  for item in "${failed[@]}"; do
    say "FAILED: $item"
  done
  return 1
}

cmd_publish() {
  need_recomp
  cd "$ROOT"
  step "Publishing tasks as Issues"
  recomp github sync-tasks
}

cmd_image() {
  cd "$ROOT"
  have docker || have podman || die "image builds need Docker or Podman"
  local runtime push=0 src=""
  runtime="$(command -v docker || command -v podman)"
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --push) push=1 ;;
      --framework-src) shift; src="${1:?--framework-src needs a path}" ;;
      *) die "unknown option for image: $1" ;;
    esac
    shift
  done
  local base rev context
  base="$(python3 -c 'import tomllib,pathlib;print(tomllib.loads(pathlib.Path("recomp.toml").read_text())["toolchain"]["base_image"])')"
  rev="$(python3 -c 'import tomllib,pathlib;print(tomllib.loads(pathlib.Path("recomp.toml").read_text())["toolchain"]["framework_rev"])')"
  context="$(mktemp -d)"
  step "Assembling the build context (framework $rev)"
  cp -a toolchain/guest "$context/guest"
  cp -a plugins "$context/plugins"
  cp toolchain/Containerfile "$context/Containerfile"
  mkdir -p "$context/framework"
  if [ -n "$src" ]; then
    [ "$(git -C "$src" rev-parse HEAD)" = "$rev" ] \
      || die "$src is not at the pinned framework_rev $rev"
    cp -a "$src/src/recomp" "$context/framework/recomp"
  else
    # The vendored tree IS the pinned framework; its base is recorded in
    # framework/UPSTREAM.md, so no second repository is consulted here.
    cp -a framework/src/recomp "$context/framework/recomp"
  fi
  find "$context/framework" -name '__pycache__' -type d -prune -exec rm -rf {} +
  step "Building ghcr.io/$(github_field owner)/sage-recomp-toolchain"
  "$runtime" build --build-arg "BASE_IMAGE=$base" -f "$context/Containerfile" \
    -t "ghcr.io/$(github_field owner)/sage-recomp-toolchain:latest" "$context"
  if [ "$push" -eq 1 ]; then
    step "Pushing and reading back the registry digest"
    "$runtime" push "ghcr.io/$(github_field owner)/sage-recomp-toolchain:latest"
    "$runtime" inspect --format '{{index .RepoDigests 0}}' \
      "ghcr.io/$(github_field owner)/sage-recomp-toolchain:latest"
    say "put that digest in recomp.toml [toolchain].image and in the coordinator config"
  else
    say "built locally; re-run with --push to publish and get the digest"
  fi
  rm -rf "$context"
}

cmd_rootfs() {
  need_recomp
  cd "$ROOT"
  # The path must be the one the coordinator config pins; building somewhere
  # else is how the guest silently ends up with an unpinned rootfs.
  local name out
  name="$(python3 -c 'import tomllib;print(tomllib.load(open("recomp.toml","rb"))["project"]["name"])')"
  out="${XDG_DATA_HOME:-$HOME/.local/share}/recomp/firecracker/${name}-rootfs.ext4"
  step "Building the Firecracker guest rootfs from the pinned image -> $out"
  recomp toolchain rootfs --output "$out" "$@"
  say "pin it: recomp coordinator setup --rootfs $out   (then rerun coordinator doctor)"
}

cmd_serve() {
  need_recomp
  cd "$ROOT"
  local owner repository recomp_bin caps
  owner="$(github_field owner)"
  repository="$(github_field repository)"
  # The vendored framework (framework/src) is the verdict code: putting it on
  # PYTHONPATH shadows any installed recomp so the loop cannot drift from the
  # tree the repository publishes. No installed recomp is needed to serve.
  [ -f "$ROOT/framework/src/recomp/__main__.py" ] \
    || die "vendored framework missing: framework/src/recomp"
  caps="+sys_admin,+sys_chroot,+setuid,+setgid,+net_admin,+mknod"
  step "Running the coordinator for $owner/$repository"
  # Firecracker's jailer needs these capabilities to create its mount namespace,
  # then drops to this account before booting the guest. One sudo prompt starts the
  # polling loop; contributors never need sudo or KVM.
  exec sudo setpriv \
    --reuid="$(id -u)" --regid="$(id -g)" --init-groups \
    --inh-caps="$caps" --ambient-caps="$caps" -- \
    env HOME="$HOME" PATH="$PATH" PYTHONPATH="$ROOT/framework/src${PYTHONPATH:+:$PYTHONPATH}" \
    python3 -m recomp coordinator run \
      --owner "$owner" --repository "$repository" "$@"
}

cmd_doctor() {
  cd "$ROOT"
  step "Owner prerequisites"
  local tool status=0
  for tool in git python3 cmake ninja g++ ccache cargo recomp; do
    if have "$tool"; then
      say "ok      $tool"
    else
      say "MISSING $tool"
      status=1
    fi
  done
  if have docker || have podman; then
    say "ok      container runtime"
  else
    say "MISSING container runtime (Docker or Podman)"
    status=1
  fi

  step "Reference cache"
  if [ -f "$ROOT/.cache/reference/build/libopenbfme_core.a" ]; then
    say "ok      .cache/reference/build/libopenbfme_core.a"
  else
    say "absent  reference build; run: scripts/coordinator.sh reference"
  fi

  step "Project gate"
  if have recomp; then
    recomp task doctor --project . || status=1
  else
    say "skipped: recomp is not installed"
    status=1
  fi
  return "$status"
}

main() {
  refuse_root
  local action="${1:-help}"
  [ "$#" -gt 0 ] && shift
  case "$action" in
    setup) cmd_setup "$@" ;;
    reference) cmd_reference ;;
    mint) cmd_mint "$@" ;;
    check) cmd_check ;;
    image) cmd_image "$@" ;;
    rootfs) cmd_rootfs "$@" ;;
    publish) cmd_publish ;;
    serve) cmd_serve "$@" ;;
    doctor) cmd_doctor ;;
    help | --help | -h) usage ;;
    *)
      printf 'error: unknown subcommand: %s\n\n' "$action" >&2
      usage >&2
      exit 2
      ;;
  esac
}

main "$@"
