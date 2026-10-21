const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");

function executeCommand(label, executable, args, { cwd, env, timeoutMs = 300_000 } = {}) {
  const started = Date.now();
  const result = spawnSync(executable, args, {
    cwd, env, timeout: timeoutMs, killSignal: "SIGKILL", encoding: "utf8",
    windowsHide: true, maxBuffer: 16 * 1024 * 1024,
  });
  return {
    label, executable, args, timeoutMs, elapsedMs: Date.now() - started,
    status: result.status, signal: result.signal,
    stdout: result.stdout || "", stderr: result.stderr || "",
    error: result.error ? result.error.message : null,
    errorCode: result.error ? result.error.code : null,
  };
}

function requireCompletedCommand(record, allowFailure = false) {
  assert.equal(record.error, null, `${record.label}: ${record.error}`);
  assert.equal(record.signal, null, `${record.label}: terminated by ${record.signal}`);
  assert.notEqual(record.status, null, `${record.label}: process did not complete`);
  if (!allowFailure) {
    assert.equal(record.status, 0, `${record.label} failed:\n${record.stdout}${record.stderr}`);
  }
}

module.exports = { executeCommand, requireCompletedCommand };
