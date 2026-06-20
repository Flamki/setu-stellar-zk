#!/usr/bin/env bash
# Setu disclosure circuit end-to-end: compile, dev setup, prove, verify.
# Run from repo root:
#   bash scripts/disclosure_e2e.sh
#
# Requires circom 2.2.x, snarkjs, and circomlib. This generates a local
# single-contributor Groth16 setup for staging/testing only, not production.
set -euo pipefail

CIRC=circuits
OUT=circuits/build_disc
CL="${CIRCOMLIB:-$(npm root)/circomlib/circuits}"
mkdir -p "$OUT"

echo "== compile twin (derives correct public signals) =="
circom "$CIRC/disclosure_witness.circom" --r1cs --wasm --sym -o "$OUT" -l "$CIRC" -l "$CL" --prime bls12381 >/dev/null

echo "== compile disclosure (the real circuit) =="
circom "$CIRC/disclosure.circom" --r1cs --wasm --sym -o "$OUT" -l "$CIRC" -l "$CL" --prime bls12381 >/dev/null

echo "== derive public signals via the twin =="
node "$OUT/disclosure_witness_js/generate_witness.js" \
  "$OUT/disclosure_witness_js/disclosure_witness.wasm" "$CIRC/disclosure_private.json" "$OUT/twin.wtns"
snarkjs wtns export json "$OUT/twin.wtns" "$OUT/twin.json" >/dev/null
node -e '
const w=require("./'"$OUT"'/twin.json"), p=require("./'"$CIRC"'/disclosure_private.json");
const o=Object.assign({},p,{nullifierHash:w[1],commitment:w[2],discloseHash:w[3],auditorTag:w[4]});
require("fs").writeFileSync("'"$OUT"'/disclosure_input.json",JSON.stringify(o,null,2));
'

echo "== dev trusted setup (single contributor; not production) =="
snarkjs powersoftau new bls12-381 12 "$OUT/pot_0.ptau" -v >/dev/null
snarkjs powersoftau contribute "$OUT/pot_0.ptau" "$OUT/pot_1.ptau" --name=setu-dev -e="dev1" >/dev/null
snarkjs powersoftau prepare phase2 "$OUT/pot_1.ptau" "$OUT/pot.ptau" -v >/dev/null
snarkjs groth16 setup "$OUT/disclosure.r1cs" "$OUT/pot.ptau" "$OUT/disc_0.zkey" >/dev/null
snarkjs zkey contribute "$OUT/disc_0.zkey" "$OUT/disc_final.zkey" --name=setu-p2 -e="dev2" >/dev/null
snarkjs zkey export verificationkey "$OUT/disc_final.zkey" "$OUT/disc_vk.json" >/dev/null

echo "== prove (witness enforces all constraints) =="
node "$OUT/disclosure_js/generate_witness.js" \
  "$OUT/disclosure_js/disclosure.wasm" "$OUT/disclosure_input.json" "$OUT/disc.wtns"
snarkjs groth16 prove "$OUT/disc_final.zkey" "$OUT/disc.wtns" "$OUT/proof.json" "$OUT/public.json" >/dev/null

echo "== verify =="
snarkjs groth16 verify "$OUT/disc_vk.json" "$OUT/public.json" "$OUT/proof.json"

echo "== convert for Soroban =="
echo "  cargo run --bin stellar-circom2soroban vk     $OUT/disc_vk.json"
echo "  cargo run --bin stellar-circom2soroban proof  $OUT/proof.json"
echo "  cargo run --bin stellar-circom2soroban public $OUT/public.json"
echo "  then: set_disclosure_vk(admin, <dvk_hex>); verify_disclosure(proof_hex, public_hex)"
