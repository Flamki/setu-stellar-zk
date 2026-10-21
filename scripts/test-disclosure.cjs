#!/usr/bin/env node
// Local characterization/regression tests, not a production trusted setup.
// Every run compiles current sources and preserves its actual commands/results.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { createHash } = require("node:crypto");
const { createRequire } = require("node:module");
const { executeCommand, requireCompletedCommand } = require("./lib/test-command.cjs");

const root = path.resolve(__dirname, "..");
const testRoot = path.join(root, "circuits", "test");
const evidenceRoot = path.join(root, "circuits/build_disc/disclosure-tests");
fs.mkdirSync(evidenceRoot, { recursive: true });
const runDir = fs.mkdtempSync(path.join(evidenceRoot,
  `${new Date().toISOString().replace(/[:.]/g, "-")}-`));
const hashNames = ["nullifierHash", "commitment", "discloseHash", "auditorTag"];
const publicNames = [...hashNames, "receiptNonce"];
const report = {
  status: "running",
  startedAt: new Date().toISOString(),
  runDir,
  node: process.version,
  sources: {},
  commands: [],
  tests: [],
  observations: [],
};
const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));
const output = (name) => path.join(runDir, name);
const writeJson = (file, value) => fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
const saveReport = () => writeJson(output("report.json"), report);

function command(label, executable, args, allowFailure = false) {
  console.log(`  ${label}`);
  const record = executeCommand(label, executable, args, { cwd: root });
  const log = `${String(report.commands.length + 1).padStart(2, "0")}-${label}.json`;
  writeJson(output(log), record);
  report.commands.push({ label, log, status: record.status });
  saveReport();
  requireCompletedCommand(record, allowFailure);
  return record;
}

function test(id, description, check, continueOnFailure = false) {
  const result = { id, description, status: "running" };
  report.tests.push(result);
  try {
    result.observed = check();
    result.status = "passed";
    console.log(`PASS ${id}: ${description}`);
  } catch (error) {
    result.status = "failed";
    result.error = error.stack;
    console.error(`FAIL ${id}: ${error.message}`);
    if (!continueOnFailure) throw error;
  } finally {
    saveReport();
  }
}

function main() {
  console.log(`Evidence: ${runDir}`);
  const deps = createRequire(path.join(testRoot, "package.json"));
  // Resolve the pinned test dependency, never an implicit npx installation.
  const snarkRoot = path.dirname(path.dirname(deps.resolve("snarkjs")));
  const snarkCli = path.join(snarkRoot, "build", "cli.cjs");
  report.snarkjs = readJson(path.join(snarkRoot, "package.json")).version;
  assert.equal(report.snarkjs, "0.7.6");
  const snark = (label, args, allowFailure) => command(label, process.execPath, [snarkCli, ...args], allowFailure);
  const compiler = command("circom-version", "circom", ["--version"]);
  report.circom = compiler.stdout.trim();
  assert.match(report.circom, /compiler 2\.2\./);
  const revision = command("git-revision", "git", ["rev-parse", "HEAD"]);
  report.revision = revision.stdout.trim();
  report.gitStatus = command("git-status", "git", ["status", "--short", "--untracked-files=all"]).stdout;

  for (const name of ["disclosure", "disclosure_witness", "auditor_recompute"]) {
    const source = path.join(root, "circuits", `${name}.circom`);
    command(`compile-${name}`, "circom", [source, "--r1cs", "--wasm", "--sym",
      "--prime", "bls12381", "-l", path.join(root, "circuits"), "-o", runDir]);
  }
  for (const file of [
    "scripts/test-disclosure.cjs", "scripts/lib/test-command.cjs", "circuits/disclosure.circom",
    "circuits/disclosure_witness.circom", "circuits/auditor_recompute.circom",
    "circuits/poseidon255.circom", "circuits/poseidon255_constants.circom",
    "circuits/test/package.json", "circuits/test/package-lock.json",
    "circuits/test/fixtures/disclosure/valid.json",
    "circuits/test/fixtures/disclosure/tampered-value.json",
  ]) {
    report.sources[file] = createHash("sha256").update(fs.readFileSync(path.join(root, file))).digest("hex");
  }

  function witness(label, circuit, input, allowFailure = false) {
    const inputPath = output(`${label}.input.json`);
    const witnessPath = output(`${label}.wtns`);
    writeJson(inputPath, input);
    const generated = path.join(runDir, `${circuit}_js`);
    const result = command(label, process.execPath, [
      path.join(generated, "generate_witness.js"),
      path.join(generated, `${circuit}.wasm`), inputPath, witnessPath,
    ], allowFailure);
    return { result, witnessPath };
  }
  function exportWitness(label, witnessPath) {
    const json = output(`${label}.witness.json`);
    snark(`export-${label}`, ["wtns", "export", "json", witnessPath, json]);
    return readJson(json);
  }
  function checkWitness(label, witnessPath) {
    snark(`check-${label}`, ["wtns", "check", output("disclosure.r1cs"), witnessPath]);
  }
  function rejectWitness(label, input) {
    const { result } = witness(label, "disclosure", input, true);
    assert.notEqual(result.status, 0, `${label}: tampered witness was accepted`);
    const diagnostic = result.stdout + result.stderr;
    // A missing tool, malformed JSON or other crash is never a passing negative.
    assert.match(diagnostic, /Assert Failed/);
    assert.match(diagnostic, /Error in template Disclosure_/);
    return { rejected: true, diagnostic: diagnostic.trim() };
  }

  const valid = readJson(path.join(testRoot, "fixtures/disclosure/valid.json"));
  const tampered = readJson(path.join(testRoot, "fixtures/disclosure/tampered-value.json"));
  assert.deepEqual(tampered, { ...valid, value: "101" }, "tampered fixture must change only value");
  const privateInput = readJson(path.join(root, "circuits/disclosure_private.json"));
  const derive = (label, input) => {
    const w = witness(label, "disclosure_witness", input);
    const values = exportWitness(label, w.witnessPath);
    return { ...input, ...Object.fromEntries(hashNames.map((name, i) => [name, values[i + 1]])) };
  };
  assert.deepEqual(derive("derive-100", privateInput), valid, "committed valid fixture drifted");
  const changed = derive("derive-101", { ...privateInput, value: "101" });
  assert.notEqual(changed.commitment, valid.commitment);
  assert.notEqual(changed.discloseHash, valid.discloseHash);
  assert.equal(changed.nullifierHash, valid.nullifierHash);
  assert.equal(changed.auditorTag, valid.auditorTag);

  let validWitness;
  test("T1", "original amount satisfies the compiled constraints", () => {
    validWitness = witness("valid-100", "disclosure", valid).witnessPath;
    checkWitness("valid-100", validWitness);
    return { value: "100", witnessChecked: true };
  });
  test("T2", "changing only the amount is rejected", () => rejectWitness("tampered-value", tampered));
  test("T3", "recomputed disclosure cannot bypass the original commitment", () =>
    rejectWitness("commitment-binding", { ...tampered, discloseHash: changed.discloseHash }));
  test("T4", "recomputed commitment cannot bypass the original disclosure hash", () =>
    rejectWitness("disclosure-binding", { ...tampered, commitment: changed.commitment }));
  test("T5", "a consistent different commitment is valid at the circuit layer", () => {
    const w = witness("valid-101", "disclosure", changed);
    checkWitness("valid-101", w.witnessPath);
    return { value: "101", differentCommitment: true, witnessChecked: true };
  });

  // Single-contributor setup with public test entropy. Never use these keys in production.
  snark("pot-new", ["powersoftau", "new", "bls12-381", "12", output("pot-0.ptau")]);
  snark("pot-contribute", ["powersoftau", "contribute", output("pot-0.ptau"), output("pot-1.ptau"),
    "--name=disclosure-tests", "-e=public-test-entropy-not-production"]);
  snark("pot-phase2", ["powersoftau", "prepare", "phase2", output("pot-1.ptau"), output("pot.ptau")]);
  snark("groth16-setup", ["groth16", "setup", output("disclosure.r1cs"), output("pot.ptau"), output("disc-0.zkey")]);
  snark("zkey-contribute", ["zkey", "contribute", output("disc-0.zkey"), output("disc.zkey"),
    "--name=disclosure-tests", "-e=public-test-phase2-not-production"]);
  snark("export-vk", ["zkey", "export", "verificationkey", output("disc.zkey"), output("vk.json")]);
  snark("prove-valid-100", ["groth16", "prove", output("disc.zkey"), validWitness, output("proof.json"), output("public.json")]);
  const publicSignals = readJson(output("public.json"));
  assert.deepEqual(publicSignals, publicNames.map((name) => valid[name]), "proof public signal order drifted");
  snark("verify-valid-100", ["groth16", "verify", output("vk.json"), output("public.json"), output("proof.json")]);
  report.validProofVerified = true;

  test("T6", "a real proof rejects a changed canonical disclosure hash", () => {
    const signals = [...publicSignals];
    signals[2] = changed.discloseHash;
    writeJson(output("tampered.public.json"), signals);
    const result = snark("verify-tampered-public", ["groth16", "verify", output("vk.json"),
      output("tampered.public.json"), output("proof.json")], true);
    assert.equal(result.status, 1);
    assert.match(result.stdout + result.stderr, /Invalid proof/);
    return { proofVerified: false, changedPublicSignal: 2 };
  });
  test("T7", "unchanged proof verifies but altered cleartext fails auditor comparison", () => {
    snark("verify-unchanged-proof", ["groth16", "verify", output("vk.json"), output("public.json"), output("proof.json")]);
    const audit = (label, value) => {
      const w = witness(label, "auditor_recompute", {
        value, recipientId: valid.recipientId, purpose: valid.purpose,
        viewingKey: valid.viewingKey, nullifierHash: valid.nullifierHash, receiptNonce: valid.receiptNonce,
      });
      return exportWitness(label, w.witnessPath);
    };
    const original = audit("audit-100", "100");
    assert.equal(original[1], publicSignals[2], "valid cleartext must match");
    assert.equal(original[2], publicSignals[3]);
    const altered = audit("audit-101", "101");
    assert.equal(altered[1], changed.discloseHash);
    assert.notEqual(altered[1], publicSignals[2]);
    assert.equal(altered[2], publicSignals[3]);
    return { proofVerified: true, originalCleartextMatches: true, alteredCleartextMatches: false };
  });

  test("T8", "a real proof rejects a changed public receipt nonce", () => {
    const signals = [...publicSignals];
    signals[4] = (BigInt(valid.receiptNonce) + 1n).toString();
    writeJson(output("tampered-nonce.public.json"), signals);
    const result = snark("verify-tampered-nonce", ["groth16", "verify", output("vk.json"),
      output("tampered-nonce.public.json"), output("proof.json")], true);
    assert.equal(result.status, 1);
    assert.match(result.stdout + result.stderr, /Invalid proof/);
    return { proofVerified: false, changedPublicSignal: 4 };
  });

  const rejection = (result, diagnosticPattern) => {
    assert.notEqual(result.status, 0, `${result.label}: invalid input was accepted`);
    const diagnostic = result.stdout + result.stderr;
    if (diagnosticPattern) assert.match(diagnostic, diagnosticPattern);
    const failureKind = /TypeError/.test(diagnostic) ? "input-type-exception"
      : /SyntaxError/.test(diagnostic) ? "input-syntax-exception"
      : /Invalid proof/.test(diagnostic) ? "invalid-proof" : "input-rejection";
    return { exitCode: result.status, failureKind, diagnostic: diagnostic.trim() };
  };
  const exceptional = (id, description, check) => test(id, description, check, true);
  const missingValue = { ...valid };
  delete missingValue.value;
  for (const [id, label, input, pattern] of [
    ["E1", "missing-value", missingValue, /Not all inputs have been set/],
    ["E2", "unknown-input", { ...valid, unknownInput: "100" }, /Signal unknownInput not found/],
    ["E3", "array-value", { ...valid, value: ["100", "101"] }, /Too many values for input signal value/],
    ["E4", "non-numeric-value", { ...valid, value: "100x" }, /Cannot convert.*BigInt/],
    ["E5", "fractional-value", { ...valid, value: "100.5" }, /Cannot convert.*BigInt/],
    ["E6", "null-value", { ...valid, value: null }, /TypeError: Cannot convert undefined or null to object/],
  ]) {
    exceptional(id, `input parsing rejects ${label} before proving`, () =>
      rejection(witness(label, "disclosure", input, true).result, pattern));
  }
  exceptional("E7", "malformed witness JSON is a parsing failure", () => {
    const inputPath = output("malformed.input.json");
    fs.writeFileSync(inputPath, '{"value":');
    return rejection(command("malformed-input-json", process.execPath, [
      output("disclosure_js/generate_witness.js"), output("disclosure_js/disclosure.wasm"),
      inputPath, output("malformed.wtns"),
    ], true), /SyntaxError/);
  });
  exceptional("E8", "malformed proof JSON cannot verify", () => {
    fs.writeFileSync(output("malformed.proof.json"), '{"pi_a":');
    return rejection(snark("verify-malformed-proof", ["groth16", "verify", output("vk.json"),
      output("public.json"), output("malformed.proof.json")], true), /SyntaxError/);
  });
  const scalarModulus = 52435875175126190479447740508185965837690552500527637822603658699938581184513n;
  for (const [id, label, signals, pattern] of [
    ["E9", "missing-public-signal", publicSignals.slice(0, -1), /Invalid proof/],
    ["E10", "extra-public-signal", [...publicSignals, "0"], /TypeError: Cannot read properties of undefined/],
    ["E11", "noncanonical-public-signal", publicSignals.map((value, i) =>
      i === 2 ? (BigInt(value) + scalarModulus).toString() : value), /Public inputs are not valid/],
  ]) {
    exceptional(id, `verifier rejects ${label}`, () => {
      writeJson(output(`${label}.public.json`), signals);
      return rejection(snark(`verify-${label}`, ["groth16", "verify", output("vk.json"),
        output(`${label}.public.json`), output("proof.json")], true), pattern);
    });
  }
  exceptional("E12", "malformed proof coordinates cannot verify", () => {
    const proof = readJson(output("proof.json"));
    proof.pi_a = ["0", "0", "1"];
    writeJson(output("bad-coordinates.proof.json"), proof);
    return rejection(snark("verify-bad-coordinates", ["groth16", "verify", output("vk.json"),
      output("public.json"), output("bad-coordinates.proof.json")], true), /Invalid proof|Proof commitments are not valid/);
  });

  snark("prove-valid-101", ["groth16", "prove", output("disc.zkey"), output("valid-101.wtns"),
    output("proof-101.json"), output("public-101.json")]);
  snark("verify-valid-101", ["groth16", "verify", output("vk.json"), output("public-101.json"), output("proof-101.json")]);
  assert.deepEqual(readJson(output("public-101.json")), publicNames.map((name) => changed[name]));
  exceptional("E13", "proof and publicSignals from two independently valid receipts cannot be mixed", () =>
    rejection(snark("verify-mixed-receipts", ["groth16", "verify", output("vk.json"),
      output("public-101.json"), output("proof.json")], true), /Invalid proof/));

  snark("setup-other-key", ["groth16", "setup", output("disclosure.r1cs"), output("pot.ptau"), output("other.zkey")]);
  snark("export-other-vk", ["zkey", "export", "verificationkey", output("other.zkey"), output("other-vk.json")]);
  assert.notDeepEqual(readJson(output("other-vk.json")), readJson(output("vk.json")));
  exceptional("E14", "a different well-formed verificationKey rejects the original proof", () =>
    rejection(snark("verify-other-key", ["groth16", "verify", output("other-vk.json"),
      output("public.json"), output("proof.json")], true), /Invalid proof/));
  exceptional("E15", "a truncated witness cannot satisfy R1CS", () => {
    const bytes = fs.readFileSync(validWitness);
    fs.writeFileSync(output("truncated.wtns"), bytes.subarray(0, 16));
    return rejection(snark("check-truncated-witness", ["wtns", "check", output("disclosure.r1cs"),
      output("truncated.wtns")], true), /SyntaxError/);
  });

  const observeValue = (label, input, changes) => {
    const { result, witnessPath } = witness(label, "disclosure", input, true);
    if (result.status === 0) checkWitness(label, witnessPath);
    report.observations.push({ label, changes, witnessAccepted: result.status === 0,
      diagnostic: (result.stdout + result.stderr).trim(),
      meaning: "Observed field/input semantics; this is not evidence of canonical amount validation." });
    saveReport();
  };
  observeValue("value-plus-scalar-modulus", { ...valid, value: (100n + scalarModulus).toString() },
    { value: "100 -> 100 + scalarModulus", commitment: "unchanged", discloseHash: "unchanged" });
  const negativeOpening = derive("derive-scalar-minus-one", { ...privateInput, value: (scalarModulus - 1n).toString() });
  observeValue("negative-value", { ...negativeOpening, value: "-1" },
    { value: "scalarModulus - 1 -> -1", commitment: "unchanged", discloseHash: "unchanged" });
  report.completed = true;
}

try {
  saveReport();
  main();
  report.status = report.tests.some((result) => result.status === "failed") ? "failed" : "passed";
  if (report.status === "failed") process.exitCode = 1;
} catch (error) {
  report.status = "failed";
  report.error = error.stack;
  console.error(error.stack);
  process.exitCode = 1;
} finally {
  report.finishedAt = new Date().toISOString();
  saveReport();
  console.log(`${report.status.toUpperCase()}: ${report.tests.filter((t) => t.status === "passed").length}/${report.tests.length} executed tests passed`);
  console.log(`Report: ${output("report.json")}`);
}
