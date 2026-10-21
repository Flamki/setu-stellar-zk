const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const { executeCommand, requireCompletedCommand } = require("./lib/test-command.cjs");

const root = path.resolve(__dirname, "..");
const evidenceRoot = path.join(root, "circuits/build_disc/process-tests");
fs.mkdirSync(evidenceRoot, { recursive: true });
const evidence = fs.mkdtempSync(path.join(evidenceRoot, "run-"));
console.log(`Process evidence: ${evidence}`);

function run(label, executable, args, options) {
  const record = executeCommand(label, executable, args, { cwd: root, ...options });
  fs.writeFileSync(path.join(evidence, `${label}.json`), `${JSON.stringify(record, null, 2)}\n`);
  return record;
}

test("a successful command preserves stdout and completes", () => {
  const record = run("success", process.execPath, ["-e", "process.stdout.write('completed')"]);
  requireCompletedCommand(record);
  assert.equal(record.stdout, "completed");
  assert.equal(record.status, 0);
});

test("an ordinary command failure cannot become success", () => {
  const record = run("prover-error", process.execPath, ["-e", "throw new Error('prover failed')"]);
  assert.equal(record.status, 1);
  assert.match(record.stderr, /prover failed/);
  assert.throws(() => requireCompletedCommand(record));
});

test("an expected proof rejection remains an explicit nonzero result", () => {
  const record = run("expected-rejection", process.execPath,
    ["-e", "console.error('Invalid proof'); process.exit(1)"]);
  requireCompletedCommand(record, true);
  assert.equal(record.status, 1);
  assert.match(record.stderr, /Invalid proof/);
});

test("a missing executable is an infrastructure error even when rejection is allowed", () => {
  const record = run("missing-executable", path.join(evidence, "missing-program.exe"), []);
  assert.equal(record.errorCode, "ENOENT");
  assert.throws(() => requireCompletedCommand(record, true));
});

test("a timed-out command is killed and cannot count as expected rejection", () => {
  const record = run("timeout", process.execPath, ["-e", "setInterval(() => {}, 1000)"], { timeoutMs: 500 });
  assert.equal(record.errorCode, "ETIMEDOUT");
  assert.throws(() => requireCompletedCommand(record, true));
});

test("the real runner saves a failed report when circom is unavailable", () => {
  const record = run("runner-missing-circom", process.execPath, [path.join(__dirname, "test-disclosure.cjs")], {
    env: { ...process.env, PATH: path.dirname(process.execPath) },
  });
  assert.equal(record.status, 1);
  const reportPath = /^Report: (.+)$/m.exec(record.stdout)?.[1].trim();
  assert.ok(reportPath, record.stdout + record.stderr);
  const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
  assert.equal(report.status, "failed");
  assert.equal(report.tests.length, 0);
  assert.notEqual(report.validProofVerified, true);
  assert.equal(report.commands[0].status, null);
  assert.match(report.error, /ENOENT/);
});
