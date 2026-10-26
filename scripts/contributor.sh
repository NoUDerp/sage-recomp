#!/usr/bin/env bash
# sage-recomp contributor helper: install the toolchain and drive the task loop.
# Usage: scripts/contributor.sh <subcommand> [args]   (runnable from any directory)

set -euo pipefail

FRAMEWORK_REPO="https://github.com/pickles976/DecompAtHome.git"
FRAMEWORK_PIN=""            # empty means: read [toolchain].framework_rev
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

say() { printf '%s\n' "$*"; }
step() { printf '\n== %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

usage() {
  cat <<'EOF'
sage-recomp contributor helper

usage: scripts/contributor.sh <subcommand> [args]

  setup [--yes] [--force]  install git, Python 3.11+, a container runtime and
                           the pinned Recomp@Home framework, then run doctor
  doctor                   run the readiness gate (must print GREENLIGHT)
  next [repo-url]          show the next claimable Issue
  start <issue-url>        claim an Issue and create its branch
  work                     scaffold the claimed task's output module
  verify                   verify the module against the committed goldens
  submit                   re-verify, push and open the pull request
  help                     print this list

No subcommand takes --target: this project declares no target binary.
EOF
}

refuse_root() {
  [ "$(id -u)" -ne 0 ] || die "do not run this script as root; it asks for sudo only when needed"
}

need_recomp() {
  have recomp || die "recomp is not installed or not on PATH; run: scripts/contributor.sh setup"
}

framework_pin() {
  # One pin, and it is the project's: the CLI, the container, and the task
  # records must all be the same framework revision or doctor and verify
  # disagree. A checkout with no [toolchain] section cannot run tasks anyway.
  if [ -n "$FRAMEWORK_PIN" ]; then printf '%s' "$FRAMEWORK_PIN"; return 0; fi
  [ -f "$ROOT/recomp.toml" ] || die "no recomp.toml in $ROOT; run this from the repository root"
  python3 - "$ROOT/recomp.toml" <<'PY'
import pathlib, sys, tomllib
rev = tomllib.loads(pathlib.Path(sys.argv[1]).read_text()).get("toolchain", {}).get("framework_rev", "")
if len(rev) != 40:
    sys.exit("recomp.toml [toolchain].framework_rev must be a 40-character commit")
print(rev)
PY
}

repo_url() {
  python3 - "$ROOT/recomp.toml" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as fh:
    cfg = tomllib.load(fh).get("github", {})
owner, repository = cfg.get("owner", ""), cfg.get("repository", "")
if not owner or not repository:
    raise SystemExit("recomp.toml has no [github] owner/repository")
print(f"https://github.com/{owner}/{repository}")
PY
}

python_ok() {
  have python3 && python3 -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 11) else 1)'
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
  # $@ = package names; no-op when everything is already present.
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
  have gh || out+=(gh-cli)   # absent from Ubuntu's default mirrors; apt_install
                             # prints a manual fallback when apt cannot provide it
}

install_framework() {
  local force="$1" pin spec
  pin="$(framework_pin)"; spec="git+${FRAMEWORK_REPO}@${pin}"
  if have recomp && [ "$force" != "1" ]; then
    say "recomp already installed at $(command -v recomp); re-run with --force to reinstall"
    return 0
  fi
  step "Installing Recomp@Home pinned at ${pin:0:12}"
  if have pipx; then
    pipx install --force "$spec" && return 0
    say "pipx install failed; falling back to pip"
  fi
  python3 -m pip install --user "$spec" && return 0
  say "pip --user failed (managed environment?); retrying with --break-system-packages"
  python3 -m pip install --user --break-system-packages "$spec"
}

check_gh() {
  # gh is not in Ubuntu's default mirrors, so it is reported, not apt-installed;
  # claims and pulls are made with your own identity either way.
  if have gh; then
    say "GitHub CLI: $(gh --version | head -1)"
  else
    say "note: the GitHub CLI (gh) is missing; \`recomp\` works with GITHUB_TOKEN, but"
    say "      \`gh auth login\` is the usual sign-in. Install gh from"
    say "      https://github.com/cli/cli#installation"
  fi
  if [ -z "${GITHUB_TOKEN:-}" ] && [ -z "${GH_TOKEN:-}" ]; then
    say "note: before \`scripts/contributor.sh start\`, sign in with \`gh auth login\`"
    say "      or export GITHUB_TOKEN (a fine-grained PAT with Issues and pull"
    say "      requests access on this repository). The coordinator App's token is"
    say "      not a contributor credential and GitHub rejects it for claims."
  fi
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
  say "  1. check git, Python 3.11+, a container runtime (Docker or Podman)"
  say "  2. install the missing ones with apt-get (asks before any sudo)"
  say "  3. pip/pipx install the pinned framework:"
  say "       python3 -m pip install \"git+${FRAMEWORK_REPO}@$(framework_pin)\""
  say "  4. confirm the GitHub CLI (gh) is available; apt may not carry it, so install"
  say "     it from https://github.com/cli/cli#installation if it is missing"
  say "  5. note how to authenticate: gh auth login, or GITHUB_TOKEN with a"
  say "     fine-grained PAT for Issues and pull requests on this repository"
  say "  6. run the readiness gate: recomp task doctor --project ."
  say "Nothing here needs a copy of the game, an emulator, or a C++ toolchain."

  local pkgs=()
  missing_packages pkgs
  if [ "${#pkgs[@]}" -eq 0 ]; then
    say ""
    say "all system packages are already present"
  else
    apt_install "${pkgs[@]}"
  fi

  python_ok || die "Python 3.11 or newer is required; this host has $(python3 -V 2>&1 || echo 'no python3')"
  if ! have docker && ! have podman; then
    die "no container runtime found; install Docker or Podman and re-run setup"
  fi

  check_gh
  install_framework "$force"
  case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) say "note: add \$HOME/.local/bin to PATH to reach the recomp command" ;;
  esac

  step "Readiness gate"
  cmd_doctor
}

cmd_doctor() {
  need_recomp
  cd "$ROOT"
  recomp task doctor --project .
}

cmd_next() {
  need_recomp
  cd "$ROOT"
  local url="${1:-$(repo_url)}"
  recomp task next "$url"
}

cmd_start() {
  need_recomp
  cd "$ROOT"
  [ "$#" -ge 1 ] || die "start needs an Issue URL: scripts/contributor.sh start <issue-url>"
  recomp task start "$1"
}

cmd_work() {
  need_recomp
  cd "$ROOT"
  recomp task run
}

cmd_verify() {
  need_recomp
  cd "$ROOT"
  recomp task verify
}

cmd_submit() {
  need_recomp
  cd "$ROOT"
  recomp task submit
}

main() {
  refuse_root
  local action="${1:-help}"
  [ "$#" -gt 0 ] && shift
  case "$action" in
    setup) cmd_setup "$@" ;;
    doctor) cmd_doctor ;;
    next) cmd_next "$@" ;;
    start) cmd_start "$@" ;;
    work) cmd_work ;;
    verify) cmd_verify ;;
    submit) cmd_submit ;;
    help | --help | -h) usage ;;
    *)
      printf 'error: unknown subcommand: %s\n\n' "$action" >&2
      usage >&2
      exit 2
      ;;
  esac
}

main "$@"
