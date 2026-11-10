#!/usr/bin/env node
// Clean or check generated local proving/testnet artifacts before release packaging.
// Usage:
//   node scripts/clean-artifacts.cjs --check   (exits non-zero if generated artifacts exist)
//   node scripts/clean-artifacts.cjs --clean   (removes generated local artifacts)

const fs = require("fs");
const path = require("path");

const root = path.join(__dirname, "..");
const mode = process.argv.includes("--clean") ? "clean" : "check";

// Generated paths that must not be packaged into clean source releases.
// Note: Checked-in test fixtures under libs/zk/testdata/ are intentional and preserved.
const generatedTargets = [
  "output",
  "circuits/build",
  "circuits/disclosure_build",
  "site/auth-config.js",
  "site/.env",
  ".env",
];

const found = [];

for (const rel of generatedTargets) {
  const full = path.join(root, rel);
  if (fs.existsSync(full)) {
    found.push(rel);
    if (mode === "clean") {
      fs.rmSync(full, { recursive: true, force: true });
      console.log(`removed ${rel}`);
    }
  }
}

if (mode === "check") {
  if (found.length > 0) {
    console.error("Found generated local/testnet artifacts that should be cleaned before packaging:");
    for (const rel of found) {
      console.error(`- ${rel}`);
    }
    console.error("Run `node scripts/clean-artifacts.cjs --clean` to remove them.");
    process.exit(1);
  }
  console.log("ok   workspace is free of generated local proving/testnet artifacts");
} else {
  console.log(`clean complete (${found.length} path(s) removed)`);
}
