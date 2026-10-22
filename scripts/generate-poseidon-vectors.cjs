// Reference outputs come only from the repository's Circom circuits.
// Rust consumes the checked-in JSON without needing a circuit toolchain.
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const { createHash } = require("node:crypto");

const root = path.resolve(__dirname, "..");
const testsDir = path.join(root, "circuits/test");
const fixturePath = path.join(root, "cli/coinutils/tests/fixtures/poseidon_vectors.json");
const modulus = 52435875175126190479447740508185965837690552500527637822603658699938581184513n;
const circom = process.env.CIRCOM_BIN || "circom";
const check = process.argv[2] === "--check";
if (process.argv.length > (check ? 3 : 2)) {
  throw new Error("Usage: node scripts/generate-poseidon-vectors.cjs [--check]");
}

const u32 = 1n << 32n;
const u64 = 1n << 64n;
const u128 = 1n << 128n;
const high = 1n << 254n;
const max = modulus - 1n;
const hashCases = [
  ["zero", [0n, 0n, 0n]],
  ["ones", [1n, 1n, 1n]],
  ["distinct", [1n, 2n, 3n]],
  ["reversed", [3n, 2n, 1n]],
  ["zero-first", [0n, 1n, 2n]],
  ["zero-middle", [1n, 0n, 2n]],
  ["zero-last", [1n, 2n, 0n]],
  ["u32-boundary", [u32 - 1n, u32, u32 + 1n]],
  ["u64-boundary", [u64 - 1n, u64, u64 + 1n]],
  ["u128-boundary", [u128 - 1n, u128, u128 + 1n]],
  ["high-bit", [high, high + 1n, 1n]],
  ["maximum", [max, max, max]],
  ["near-modulus", [max, max - 1n, max - 2n]],
  ["mixed-zero-max", [0n, max, 1n]],
  ["mixed-max-zero", [max, 0n, 1n]],
];
const commitmentCases = [
  ["zero", [0n, 0n, 0n, 0n]],
  ["ones", [1n, 1n, 1n, 1n]],
  ["distinct", [1n, 2n, 3n, 4n]],
  ["swapped-value-label", [2n, 1n, 3n, 4n]],
  ["swapped-nullifier-secret", [1n, 2n, 4n, 3n]],
  ["disclosure-sample", [100n, 42n, 12345n, 67890n]],
  ["u32-boundary", [u32 - 1n, u32, u32 + 1n, u32 + 2n]],
  ["u64-boundary", [u64 - 1n, u64, u64 + 1n, u64 + 2n]],
  ["u128-boundary", [u128 - 1n, u128, u128 + 1n, u128 + 2n]],
  ["high-bit", [high, high + 1n, high + 2n, high + 3n]],
  ["maximum", [max, max, max, max]],
  ["mixed-zero-max", [0n, max, max - 1n, 1n]],
];

function decimal(value) {
  if (value < 0n || value >= modulus) throw new Error("Non-canonical field input");
  return value.toString();
}

async function main() {
  const snarkjsEntry = require.resolve("snarkjs", { paths: [testsDir] });
  const snarkjs = require(snarkjsEntry);
  const snarkjsVersion = JSON.parse(fs.readFileSync(path.join(path.dirname(snarkjsEntry), "../package.json"), "utf8")).version;
  if (snarkjsVersion !== "0.7.6") throw new Error(`Regeneration requires snarkjs 0.7.6; found ${snarkjsVersion}`);
  const circomVersion = execFileSync(circom, ["--version"], { encoding: "utf8" }).trim();
  if (circomVersion !== "circom compiler 2.2.2") {
    throw new Error(`Regeneration requires Circom 2.2.2; found ${circomVersion}`);
  }
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "setu-poseidon-vectors-"));
  try {
    function compile(name) {
      const source = path.join(testsDir, `${name}.circom`);
      execFileSync(circom, [source, "--prime", "bls12381", "--O1", "--r1cs", "--wasm", "--sym", "-o", scratch], { stdio: "pipe" });
      const symbols = new Map(fs.readFileSync(path.join(scratch, `${name}.sym`), "utf8")
        .trim().split(/\r?\n/).map((line) => {
          const columns = line.split(",");
          return [columns[3], Number(columns[1])];
        }));
      return { name, symbols };
    }

    async function outputs(circuit, input, names) {
      const witnessPath = path.join(scratch, `${circuit.name}.wtns`);
      await snarkjs.wtns.calculate(input, path.join(scratch, `${circuit.name}_js`, `${circuit.name}.wasm`), witnessPath);
      if (!await snarkjs.wtns.check(path.join(scratch, `${circuit.name}.r1cs`), witnessPath)) {
        throw new Error(`Unsatisfied constraints in ${circuit.name}`);
      }
      const witness = await snarkjs.wtns.exportJson(witnessPath);
      return names.map((name) => {
        const index = circuit.symbols.get(`main.${name}`);
        if (!Number.isInteger(index) || index < 1) throw new Error(`Missing output: ${name}`);
        return decimal(BigInt(witness[index]));
      });
    }

    const hashes = compile("test_poseidon_vectors");
    const commitments = compile("test_commitment");
    const sources = ["circuits/poseidon255.circom", "circuits/poseidon255_constants.circom",
      "circuits/commitment.circom", "circuits/test/test_poseidon_vectors.circom", "circuits/test/test_commitment.circom"];
    const fixture = {
      schema_version: 1,
      field: "BLS12-381 scalar",
      modulus: modulus.toString(),
      provenance: {
        generator: "node scripts/generate-poseidon-vectors.cjs",
        circom: "2.2.2",
        snarkjs: "0.7.6",
        compiler_flags: "--prime bls12381 --O1 --r1cs --wasm --sym",
        source_sha256_lf: Object.fromEntries(sources.map((source) => [source,
          createHash("sha256").update(fs.readFileSync(path.join(root, source), "utf8").replace(/\r\n/g, "\n")).digest("hex")])),
      },
      hashes: [],
      commitments: [],
    };
    for (const [id, values] of hashCases) {
      const inputs = values.map(decimal);
      fixture.hashes.push({ id, inputs, outputs: await outputs(hashes, { inputs }, ["hash1", "hash2", "hash3"]) });
    }
    console.log(`Checked ${hashCases.length * 3} Poseidon outputs against Circom constraints.`);
    for (const [id, values] of commitmentCases) {
      const [value, label, nullifier, secret] = values.map(decimal);
      const input = { value, label, nullifier, secret };
      const [commitment, nullifier_hash] = await outputs(commitments, input, ["commitment", "nullifierHash"]);
      fixture.commitments.push({ id, ...input, commitment, nullifier_hash });
    }
    const content = `${JSON.stringify(fixture, null, 2)}\n`;
    if (check) {
      if (fs.readFileSync(fixturePath, "utf8").replace(/\r\n/g, "\n") !== content) {
        throw new Error("Circom vectors differ from the checked-in fixture. Regenerate and review the changes.");
      }
      console.log("PASS: regenerated vectors match the checked-in fixture.");
    } else {
      fs.mkdirSync(path.dirname(fixturePath), { recursive: true });
      fs.writeFileSync(fixturePath, content);
      console.log(`Wrote ${hashCases.length * 3 + commitmentCases.length * 2} reference outputs to ${path.relative(root, fixturePath)}.`);
    }
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
}

// snarkjs's constraint checker keeps worker threads alive after its promise resolves.
main().then(() => process.exit(0)).catch((error) => {
  console.error(error);
  process.exit(1);
});
