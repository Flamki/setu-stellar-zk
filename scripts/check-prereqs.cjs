// Prerequisite version checker.
//
// Setu's local setup (see README "Run Locally") depends on Rust, the Stellar
// CLI, Node.js, circom, snarkjs, and circomlib. New contributors often hit a
// wall because one of those is missing or on the wrong track, with no quick
// way to tell which. This script prints what is required next to what is
// actually found, and exits non-zero when something needs fixing so it can
// also be used as a CI/pre-flight gate, not just eyeballed.
//
// This is a prototype convenience check, not a hermetic environment
// validator: it shells out to whatever binaries resolve on PATH (or, for
// snarkjs/circomlib, whatever npm install has already produced) and reads
// their self-reported version strings. It does not sandbox, install, or pin
// anything for you.
//
// Run: node scripts/check-prereqs.cjs
//
// Mirrors the ok/fail/exit-non-zero style already used in
// scripts/docs-check.cjs and site/smoke-check.cjs so the repo has one
// recognizable assertion style across its Node scripts.

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const root = path.join(__dirname, "..");
const parentWorkspaceRoot = path.join(root, "..");

const results = [];

function record(name, required, found, status, hint) {
  results.push({ name, required, found, status, hint });
}

// Runs `command arg...` and returns its stdout+stderr as a single string, or
// null if the command could not be found/executed. `shell: true` is what
// makes this resolve `.exe`/`.cmd`/`.ps1` shims on Windows (PATHEXT) the same
// way it resolves plain PATH entries on macOS/Linux, so one code path covers
// both without any OS-specific branching.
function run(command, args) {
  const result = spawnSync(command, args, {
    shell: true,
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error || result.status === null) {
    return null;
  }
  const output = `${result.stdout || ""}${result.stderr || ""}`;
  return output.trim() || null;
}

function extractVersion(text) {
  if (!text) {
    return null;
  }
  const match = text.match(/(\d+\.\d+\.\d+)/);
  return match ? match[1] : null;
}

function readJsonVersion(packageJsonPath) {
  if (!fs.existsSync(packageJsonPath)) {
    return null;
  }
  try {
    const pkg = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
    return pkg.version || null;
  } catch {
    return null;
  }
}

// --- Rust -------------------------------------------------------------
// README: "Rust stable." No specific minimum version is pinned anywhere in
// the repo (no rust-toolchain.toml), so any working rustc counts as OK.
function checkRust() {
  const output = run("rustc", ["--version"]);
  const found = extractVersion(output);
  if (!found) {
    record(
      "Rust",
      "stable toolchain",
      null,
      "MISSING",
      "Install from https://rustup.rs, then re-run `rustc --version`."
    );
    return;
  }
  record("Rust", "stable toolchain", found, "OK");
}

// --- Stellar CLI --------------------------------------------------------
// README: "Stellar CLI 26.x." scripts/live_testnet_e2e.ps1 also resolves a
// `stellar` binary specifically (not `soroban`), so that is the command this
// repo actually uses; `soroban` is only checked as a legacy fallback so the
// hint can point contributors at the rename if that's what they have.
function checkStellarCli() {
  const required = "26.x";
  let output = run("stellar", ["--version"]);
  let toolName = "stellar";
  if (!output) {
    output = run("soroban", ["--version"]);
    toolName = "soroban";
  }
  const found = extractVersion(output);
  if (!found) {
    record(
      "Stellar CLI",
      required,
      null,
      "MISSING",
      "Install with `cargo install --locked stellar-cli` or see https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli."
    );
    return;
  }
  if (!found.startsWith("26.")) {
    record(
      "Stellar CLI",
      required,
      `${found} (via ${toolName})`,
      "MISMATCH",
      "Found a Stellar CLI outside the 26.x series this repo was built against; testnet flows may not match README output."
    );
    return;
  }
  record("Stellar CLI", required, `${found} (via ${toolName})`, "OK");
}

// --- Node.js -------------------------------------------------------------
// README only says "Node.js" with no pinned minimum. This script is itself
// running under the Node.js interpreter, so process.version is the ground
// truth — no need to shell out and re-discover what is already running it.
function checkNode() {
  record("Node.js", "any", process.version, "OK");
}

// --- circom ---------------------------------------------------------------
// README: "`circom` 2.2.x." Makefile and scripts/disclosure_e2e.sh both
// invoke `circom` directly with `--prime bls12381`, which only 2.2.x
// supports.
function checkCircom() {
  const required = "2.2.x";
  const output = run("circom", ["--version"]);
  const found = extractVersion(output);
  if (!found) {
    record(
      "circom",
      required,
      null,
      "MISSING",
      "Build from https://github.com/iden3/circom (2.2.x branch/tag) with `cargo build --release` and put the binary on PATH."
    );
    return;
  }
  if (!found.startsWith("2.2.")) {
    record(
      "circom",
      required,
      found,
      "MISMATCH",
      "This repo compiles circuits with `--prime bls12381`, which needs circom 2.2.x specifically."
    );
    return;
  }
  record("circom", required, found, "OK");
}

// --- snarkjs ---------------------------------------------------------------
// README: "`snarkjs` 0.7.x." scripts/live_testnet_e2e.ps1 resolves it either
// on PATH or at <workspace>/node_modules/snarkjs/build/cli.cjs, so this
// checks both without shelling out to `npx` (which would silently download a
// throwaway copy over the network just to answer a version question).
function checkSnarkjs() {
  const required = "0.7.x";
  const output = run("snarkjs", ["--version"]);
  const fromPath = extractVersion(output);
  if (fromPath) {
    finishSnarkjs(required, fromPath, "on PATH");
    return;
  }

  const candidates = [
    path.join(root, "node_modules", "snarkjs", "package.json"),
    path.join(parentWorkspaceRoot, "node_modules", "snarkjs", "package.json"),
  ];
  for (const candidate of candidates) {
    const version = readJsonVersion(candidate);
    if (version) {
      finishSnarkjs(required, version, path.relative(root, candidate));
      return;
    }
  }

  record(
    "snarkjs",
    required,
    null,
    "MISSING",
    "Run `npm install` in the parent workspace (see README > Run Locally), or `npm install -g snarkjs`."
  );
}

function finishSnarkjs(required, found, source) {
  if (!found.startsWith("0.7.")) {
    record(
      "snarkjs",
      required,
      `${found} (${source})`,
      "MISMATCH",
      "Other 0.x lines have produced incompatible proof/vkey JSON shapes for this repo's BLS12-381 flow."
    );
    return;
  }
  record("snarkjs", required, `${found} (${source})`, "OK");
}

// --- circomlib ---------------------------------------------------------------
// README: "`circomlib`." This is an npm package providing .circom include
// files, not a CLI — Makefile and scripts/disclosure_e2e.sh both resolve it
// as a directory of circuits (via a hardcoded path or the CIRCOMLIB env var),
// not as a command. No version is pinned anywhere in the repo, so presence
// is all this checks.
function checkCircomlib() {
  const required = "present (no version pinned)";

  const envPath = process.env.CIRCOMLIB;
  if (envPath) {
    // disclosure_e2e.sh and the Makefile point CIRCOMLIB at a
    // `.../circomlib/circuits` directory; the package.json lives one level up.
    const packageJsonPath = path.join(path.dirname(envPath), "package.json");
    const version = readJsonVersion(packageJsonPath) || readJsonVersion(path.join(envPath, "package.json"));
    if (fs.existsSync(envPath)) {
      record(
        "circomlib",
        required,
        version ? `${version} (via $CIRCOMLIB)` : `found (via $CIRCOMLIB, version unknown)`,
        "OK"
      );
      return;
    }
  }

  const candidates = [
    path.join(root, "node_modules", "circomlib", "package.json"),
    path.join(parentWorkspaceRoot, "node_modules", "circomlib", "package.json"),
  ];
  for (const candidate of candidates) {
    const version = readJsonVersion(candidate);
    if (version) {
      record("circomlib", required, `${version} (${path.relative(root, candidate)})`, "OK");
      return;
    }
  }

  record(
    "circomlib",
    required,
    null,
    "MISSING",
    "Run `npm install` in the parent workspace (see README > Run Locally), or set $CIRCOMLIB to a circomlib checkout's `circuits` directory."
  );
}

function printReport() {
  const nameWidth = Math.max(...results.map((r) => r.name.length), 4);
  const requiredWidth = Math.max(...results.map((r) => r.required.length), 8);

  console.log(
    `${"Tool".padEnd(nameWidth)}  ${"Required".padEnd(requiredWidth)}  Found`
  );
  console.log(`${"-".repeat(nameWidth)}  ${"-".repeat(requiredWidth)}  -----`);
  for (const r of results) {
    const foundText = r.found || "not found";
    const marker = r.status === "OK" ? "OK  " : r.status === "MISMATCH" ? "WARN" : "FAIL";
    console.log(
      `${marker} ${r.name.padEnd(nameWidth)}  ${r.required.padEnd(requiredWidth)}  ${foundText}`
    );
    if (r.status !== "OK" && r.hint) {
      console.log(`       ${r.hint}`);
    }
  }
}

function main() {
  checkRust();
  checkStellarCli();
  checkNode();
  checkCircom();
  checkSnarkjs();
  checkCircomlib();

  printReport();

  const problems = results.filter((r) => r.status !== "OK");
  if (problems.length > 0) {
    console.error(
      `\n${problems.length} of ${results.length} prerequisite check(s) need attention.`
    );
    process.exit(1);
  }
  console.log(`\nAll ${results.length} prerequisite checks passed.`);
}

main();
