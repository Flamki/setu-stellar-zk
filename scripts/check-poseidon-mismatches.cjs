const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const fixturePath = "cli/coinutils/tests/fixtures/poseidon_vectors.json";
const hashTest = "poseidon_matches_circom_vectors";
const commitmentTest = "commitments_and_nullifiers_match_circom_vectors";

function cargoArgs(test) {
  const args = ["test", "--locked", "-p", "coinutils", "--test", "poseidon_parity"];
  return test ? [...args, test, "--", "--exact"] : args;
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    windowsHide: true,
    maxBuffer: 10 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  return { status: result.status, output: `${result.stdout || ""}${result.stderr || ""}` };
}

function verifyBaseline() {
  const result = run("cargo", cargoArgs());
  if (result.status !== 0) throw new Error(`Unmodified parity tests failed:\n${result.output}`);
}

function fixtureChange(update) {
  return (text) => {
    const fixture = JSON.parse(text);
    update(fixture);
    return `${JSON.stringify(fixture, null, 2)}\n`;
  };
}

function replaceOnce(before, after) {
  return (text) => {
    if (text.split(before).length !== 2) throw new Error(`Expected one mutation target: ${before}`);
    return text.replace(before, after);
  };
}

function probe({ id, file = fixturePath, change, test, diagnostic, generator = false }) {
  const target = path.join(root, file);
  const original = fs.readFileSync(target);
  try {
    const changed = change(original.toString("utf8"));
    if (changed === original.toString("utf8")) throw new Error(`No mutation applied: ${id}`);
    fs.writeFileSync(target, changed);
    const result = generator
      ? run(process.execPath, ["scripts/generate-poseidon-vectors.cjs", "--check"])
      : run("cargo", cargoArgs(test));
    const expectedStatus = generator ? 1 : 101;
    if (result.status !== expectedStatus || !result.output.includes(diagnostic)) {
      throw new Error(`Mismatch was not detected as expected: ${id}\n${result.output}`);
    }
    console.log(`PASS: detected ${id}`);
  } finally {
    fs.writeFileSync(target, original);
    if (!fs.readFileSync(target).equals(original)) throw new Error(`Could not restore ${file}`);
  }
}

function main() {
  verifyBaseline();
  for (let arity = 1; arity <= 3; arity++) {
    probe({
      id: `incorrect expected hash, arity ${arity}`,
      change: fixtureChange((f) => {
        f.hashes[0].outputs[arity - 1] = (BigInt(f.hashes[0].outputs[arity - 1]) + 1n).toString();
      }),
      test: hashTest,
      diagnostic: `case=zero, arity=${arity}`,
    });
  }
  for (const field of ["commitment", "nullifier_hash"]) {
    probe({
      id: `incorrect expected ${field}`,
      change: fixtureChange((f) => {
        f.commitments[0][field] = (BigInt(f.commitments[0][field]) + 1n).toString();
      }),
      test: commitmentTest,
      diagnostic: `Circom/Rust ${field === "commitment" ? "commitment" : "nullifier hash"} mismatch: case=zero`,
    });
  }
  probe({
    id: "reversed Rust Poseidon inputs",
    file: "cli/coinutils/src/crypto/poseidon.rs",
    change: replaceOnce("for input in inputs.iter() {", "for input in inputs.iter().rev() {"),
    test: hashTest,
    diagnostic: "Circom/Rust Poseidon mismatch: case=distinct, arity=2",
  });
  probe({
    id: "swapped Rust commitment fields",
    file: "cli/coinutils/src/crypto/coin.rs",
    change: replaceOnce("poseidon_hash(env, &[value, label, precommitment])", "poseidon_hash(env, &[label, value, precommitment])"),
    test: commitmentTest,
    diagnostic: "Circom/Rust commitment mismatch: case=distinct",
  });
  probe({
    id: "stale Circom reference fixture",
    change: fixtureChange((f) => {
      f.hashes[0].outputs[0] = (BigInt(f.hashes[0].outputs[0]) + 1n).toString();
    }),
    generator: true,
    diagnostic: "Circom vectors differ from the checked-in fixture",
  });
  verifyBaseline();
  console.log("PASS: all 8 mismatches detected; original files restored; unmodified parity tests pass.");
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
