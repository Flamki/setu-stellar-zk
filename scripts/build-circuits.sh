#!/usr/bin/env bash
#
# build-circuits.sh — regenerate the Setu circuit artifacts with one command.
#
# Produces (all already gitignored, so a fresh clone can regenerate them):
#   circuits/build       withdrawal ("main") circuit + dummy/test circuits
#   circuits/output      withdrawal Groth16 dev setup:
#                          main_final.zkey, main_verification_key.json
#   circuits/build_disc  selective-disclosure circuit + Groth16 dev setup
#                          (delegates to scripts/disclosure_e2e.sh)
#
# The Groth16 setup written here is a single-contributor local/staging setup.
# It is NOT a production trusted setup; do not treat these artifacts as ceremony
# output. See docs/privacy-compliance-limitations.md.
#
# Usage:
#   bash scripts/build-circuits.sh [--main|--disclosure|--all]
#                                  [--clean] [--check] [--dry-run]
#                                  [--pot-power N]
#
#   --main         build only the withdrawal circuit + its dev setup
#   --disclosure   build only the selective-disclosure circuit
#   --all          build both (default)
#   --clean        delete generated artifact directories first (safe: only
#                  circuits/build, circuits/output, circuits/build_disc)
#   --check        only verify prerequisites and print versions, build nothing
#   --dry-run      print the commands that would run, execute nothing
#   --pot-power N  powersoftau power to use (default: auto from the .r1cs)
#
# Environment:
#   CIRCOMLIB   path to circomlib's `circuits` include directory. If unset the
#               script searches ../node_modules, ./node_modules and `npm root`.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

CIRC="circuits"
MAIN_COMPILE_OUT="$CIRC/build"
MAIN_SETUP_OUT="$CIRC/output"
DISC_OUT="$CIRC/build_disc"

MODE="all"
DO_CLEAN=0
CHECK_ONLY=0
DRY_RUN=0
POT_POWER="${POT_POWER:-}"

usage() {
  cat <<'EOF'
build-circuits.sh — regenerate the Setu circuit artifacts with one command.

Usage:
  bash scripts/build-circuits.sh [--main|--disclosure|--all]
                                 [--clean] [--check] [--dry-run] [--pot-power N]

  --main         build only the withdrawal circuit + its dev Groth16 setup
  --disclosure   build only the selective-disclosure circuit
  --all          build both (default)
  --clean        delete generated artifact directories first (only
                 circuits/build, circuits/output, circuits/build_disc)
  --check        only verify prerequisites and print versions; build nothing
  --dry-run      print the commands that would run; execute nothing
  --pot-power N  powersoftau power to use (default: auto from the .r1cs)
  -h, --help     show this help

Environment:
  CIRCOMLIB   path to circomlib's `circuits` include directory. When unset the
              script searches ../node_modules, ./node_modules and `npm root`.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --main) MODE="main" ;;
    --disclosure) MODE="disclosure" ;;
    --all) MODE="all" ;;
    --clean) DO_CLEAN=1 ;;
    --check) CHECK_ONLY=1 ;;
    --dry-run) DRY_RUN=1 ;;
    --pot-power)
      shift
      [ "$#" -gt 0 ] || { echo "ERROR: --pot-power needs a value" >&2; exit 2; }
      POT_POWER="$1"
      ;;
    -h|--help) usage; exit 0 ;;
    *) echo "ERROR: unknown argument: $1" >&2; echo; usage; exit 2 ;;
  esac
  shift
done

# ─── helpers ────────────────────────────────────────────────────────────────

run() {
  if [ "$DRY_RUN" -eq 1 ]; then
    printf '  [dry-run] %s\n' "$*"
  else
    "$@"
  fi
}

say() { printf '\n== %s ==\n' "$*"; }

# Resolve circomlib's `circuits` include directory.
resolve_circomlib() {
  if [ -n "${CIRCOMLIB:-}" ] && [ -d "$CIRCOMLIB" ]; then
    printf '%s\n' "$CIRCOMLIB"
    return 0
  fi
  local candidate
  for candidate in \
    "$REPO_ROOT/../node_modules/circomlib/circuits" \
    "$REPO_ROOT/node_modules/circomlib/circuits" \
    "$(npm root 2>/dev/null || true)/circomlib/circuits" \
    "$(npm root -g 2>/dev/null || true)/circomlib/circuits"; do
    if [ -n "$candidate" ] && [ -d "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

# Clean only generated, gitignored directories. Refuses anything unexpected.
clean_artifacts() {
  say "cleaning generated artifacts"
  local dir
  for dir in "$MAIN_COMPILE_OUT" "$MAIN_SETUP_OUT" "$DISC_OUT"; do
    case "$dir" in
      "$CIRC"/*) ;;
      *) echo "ERROR: refusing to clean unexpected path: $dir" >&2; exit 1 ;;
    esac
    if [ -e "$dir" ]; then
      echo "  removing $dir"
      run rm -rf -- "$dir"
    else
      echo "  (nothing at $dir)"
    fi
  done
}

tool_version() {
  { "$@" 2>&1 || true; } | tr -d '\r' | head -n 1
}

# Write a versions manifest next to the artifacts so a build can be tied back
# to the exact toolchain that produced it.
record_versions() {
  local out="$1"
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "  [dry-run] record versions -> $out/versions.txt"
    return 0
  fi
  mkdir -p "$out"
  {
    echo "generated_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "git_commit=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
    echo "circom=$(tool_version circom --version)"
    echo "snarkjs=$(tool_version snarkjs --version)"
    echo "circomlib=$(node -e 'try{console.log(require(process.argv[1]).version)}catch(e){console.log("unknown")}' "$CIRCOMLIB_PATH/../package.json" 2>/dev/null || echo unknown)"
    echo "curve=bls12-381"
    echo "note=single-contributor dev setup, not a production trusted setup"
  } > "$out/versions.txt"
  echo "  recorded versions -> $out/versions.txt"
}

# Choose the smallest powersoftau power whose 2^N is strictly greater than the
# circuit's constraint count (headroom for internal constraints).
compute_power() {
  local r1cs="$1" constraints
  constraints="$(snarkjs r1cs info "$r1cs" 2>/dev/null | grep -i 'constraints' | grep -oE '[0-9]+' | head -n 1 || true)"
  if [ -z "$constraints" ]; then
    echo "ERROR: could not read a constraint count from $r1cs" >&2
    return 1
  fi
  node -e 'const n = Number(process.argv[1]); if (!Number.isFinite(n) || n <= 0) process.exit(1); console.log(Math.max(12, Math.floor(Math.log2(n)) + 1));' "$constraints"
}

# ─── prerequisites ──────────────────────────────────────────────────────────

MISSING=0
for tool in node circom snarkjs; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "ERROR: required tool '$tool' was not found on PATH." >&2
    case "$tool" in
      node) echo "  install Node.js 20+ (https://nodejs.org)." >&2 ;;
      circom) echo "  install circom 2.2.x: cargo install --git https://github.com/iden3/circom circom" >&2 ;;
      snarkjs) echo "  install snarkjs 0.7.x: npm install -g snarkjs" >&2 ;;
    esac
    MISSING=1
  fi
done

CIRCOMLIB_PATH="$(resolve_circomlib || true)"
if [ -z "$CIRCOMLIB_PATH" ]; then
  echo "ERROR: circomlib include directory not found." >&2
  echo "  run 'npm install' in the parent workspace, or set CIRCOMLIB to a circomlib checkout's 'circuits' directory." >&2
  MISSING=1
fi

if [ "$MISSING" -ne 0 ]; then
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "WARNING: prerequisites missing, continuing because --dry-run was requested." >&2
  else
    echo "" >&2
    echo "Prerequisites are missing; nothing was built." >&2
    echo "Run 'node scripts/check-prereqs.cjs' for a full toolchain report (it also checks Rust and the Stellar CLI)." >&2
    exit 1
  fi
fi

if [ "$MISSING" -eq 0 ]; then
  say "toolchain"
  echo "  circom   : $(tool_version circom --version)"
  echo "  snarkjs  : $(tool_version snarkjs --version)"
  echo "  circomlib: $(node -e 'try{console.log(require(process.argv[1]).version)}catch(e){console.log("unknown")}' "$CIRCOMLIB_PATH/../package.json" 2>/dev/null || echo unknown) ($CIRCOMLIB_PATH)"
fi

if [ "$CHECK_ONLY" -eq 1 ]; then
  say "prerequisite check only (--check); nothing built"
  exit 0
fi

# ─── build ──────────────────────────────────────────────────────────────────

if [ "$DO_CLEAN" -eq 1 ]; then
  clean_artifacts
fi

build_main() {
  say "[main] compiling withdrawal circuit into $MAIN_COMPILE_OUT"
  run mkdir -p "$MAIN_COMPILE_OUT"
  ( cd "$CIRC" && run circom main.circom --r1cs --wasm --sym -o build -l "$CIRCOMLIB_PATH" --prime bls12381 )
  ( cd "$CIRC" && run circom dummy.circom --r1cs --wasm --sym -o build -l "$CIRCOMLIB_PATH" --prime bls12381 )
  ( cd "$CIRC/test" && run circom test_merkleProof.circom --wasm -o ../build -l "$CIRCOMLIB_PATH" --prime bls12381 )

  say "[main] dev Groth16 setup into $MAIN_SETUP_OUT"
  run mkdir -p "$MAIN_SETUP_OUT"
  local power="$POT_POWER"
  if [ -z "$power" ]; then
    if [ "$DRY_RUN" -eq 1 ]; then
      power="auto"
    else
      power="$(compute_power "$MAIN_COMPILE_OUT/main.r1cs")"
    fi
  fi
  echo "  powersoftau power: $power"
  run snarkjs powersoftau new bls12-381 "$power" "$MAIN_SETUP_OUT/pot_0.ptau" -v
  run snarkjs powersoftau contribute "$MAIN_SETUP_OUT/pot_0.ptau" "$MAIN_SETUP_OUT/pot_1.ptau" --name=setu-dev -e="dev1"
  run snarkjs powersoftau prepare phase2 "$MAIN_SETUP_OUT/pot_1.ptau" "$MAIN_SETUP_OUT/pot.ptau" -v
  run snarkjs groth16 setup "$MAIN_COMPILE_OUT/main.r1cs" "$MAIN_SETUP_OUT/pot.ptau" "$MAIN_SETUP_OUT/main_0.zkey"
  run snarkjs zkey contribute "$MAIN_SETUP_OUT/main_0.zkey" "$MAIN_SETUP_OUT/main_final.zkey" --name=setu-main -e="dev2"
  run snarkjs zkey export verificationkey "$MAIN_SETUP_OUT/main_final.zkey" "$MAIN_SETUP_OUT/main_verification_key.json"
  record_versions "$MAIN_SETUP_OUT"
}

build_disclosure() {
  say "[disclosure] building selective-disclosure artifacts into $DISC_OUT"
  # Reuse the existing, tested disclosure pipeline so there is a single source
  # of truth for the disclosure setup.
  run bash "$REPO_ROOT/scripts/disclosure_e2e.sh"
  record_versions "$DISC_OUT"
}

case "$MODE" in
  main) build_main ;;
  disclosure) build_disclosure ;;
  all) build_main; build_disclosure ;;
esac

say "done"
echo "  withdrawal artifacts : $MAIN_COMPILE_OUT (compile), $MAIN_SETUP_OUT (setup)"
echo "  disclosure artifacts : $DISC_OUT"
echo "  prototype note       : the Groth16 setups above are local single-contributor"
echo "                         dev setups, not a production trusted setup."
