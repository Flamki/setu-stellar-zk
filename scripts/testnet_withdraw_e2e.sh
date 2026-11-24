#!/usr/bin/env bash
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:-setu_operator}"
SCOPE="${SCOPE:-setu_pool}"
TOKEN_ADDRESS="${TOKEN_ADDRESS:-CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC}"
# Pool denomination in the token's smallest unit. The deployed pool and the
# generated coin must use the same value (see docs/denominations.md).
DENOMINATION="${DENOMINATION:-1000000000}"

# Artifacts are written to an evidence directory (gitignored) instead of the repo root.
EVIDENCE_DIR="${EVIDENCE_DIR:-evidence/local}"
mkdir -p "$EVIDENCE_DIR"

COIN_FILE="${COIN_FILE:-$EVIDENCE_DIR/testnet_coin.json}"
STATE_FILE="${STATE_FILE:-$EVIDENCE_DIR/testnet_state.json}"
ASSOCIATION_FILE="${ASSOCIATION_FILE:-$EVIDENCE_DIR/testnet_association.json}"
WITHDRAWAL_FILE="${WITHDRAWAL_FILE:-$EVIDENCE_DIR/withdrawal_input.json}"
VK_HEX_FILE="$EVIDENCE_DIR/vk_hex.txt"
PROOF_HEX_FILE="$EVIDENCE_DIR/proof_hex.txt"
PUBLIC_HEX_FILE="$EVIDENCE_DIR/public_hex.txt"

echo "Starting Setu privacy-pool testnet run..."

echo "Cleaning generated testnet files..."
rm -f "$COIN_FILE" "$STATE_FILE" "$ASSOCIATION_FILE" \
  "$VK_HEX_FILE" "$PROOF_HEX_FILE" "$PUBLIC_HEX_FILE" "$WITHDRAWAL_FILE" \
  circuits/witness.wtns circuits/proof.json circuits/public.json

echo "Checking prerequisites..."
command -v jq >/dev/null 2>&1 || { echo "Error: jq is required."; exit 1; }
command -v stellar >/dev/null 2>&1 || { echo "Error: stellar CLI is required."; exit 1; }
command -v snarkjs >/dev/null 2>&1 || { echo "Error: snarkjs is required."; exit 1; }

echo "Ensuring source account is funded: $SOURCE"
if ! stellar keys ls 2>/dev/null | grep -q "^${SOURCE}$"; then
  stellar keys generate "$SOURCE" >/dev/null 2>&1
fi
stellar keys fund "$SOURCE" --network "$NETWORK" >/dev/null 2>&1 || true

echo "Building and optimizing contract..."
cargo build --target wasm32v1-none --release -p privacy-pools
stellar contract optimize \
  --wasm target/wasm32v1-none/release/privacy_pools.wasm \
  --wasm-out target/wasm32v1-none/release/privacy_pools.optimized.wasm

echo "Converting withdrawal verification key..."
cargo run --bin stellar-circom2soroban vk circuits/output/main_verification_key.json > "$VK_HEX_FILE"
VK_HEX="$(grep -o '[0-9a-f]*$' "$VK_HEX_FILE")"
if [ -z "$VK_HEX" ]; then
  echo "Error: failed to extract verification key hex."
  exit 1
fi

echo "Deploying contract to $NETWORK..."
DEPLOY_OUTPUT="$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/privacy_pools.optimized.wasm \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- \
  --vk_bytes "$VK_HEX" \
  --token_address "$TOKEN_ADDRESS" \
  --admin "$SOURCE" \
  --denomination "$DENOMINATION" 2>&1)"

CONTRACT_ID="$(echo "$DEPLOY_OUTPUT" | grep -oE 'C[A-Z0-9]{55}' | tail -1)"
if [ -z "$CONTRACT_ID" ]; then
  echo "Error: failed to extract contract ID from deployment."
  echo "$DEPLOY_OUTPUT"
  exit 1
fi
echo "Contract deployed: $CONTRACT_ID"

stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" -- get_admin

echo "Generating note commitment..."
cargo run --bin stellar-coinutils generate "$SCOPE" -o "$COIN_FILE" --denomination "$DENOMINATION"
COMMITMENT_HEX="$(jq -r '.commitment_hex' "$COIN_FILE" | sed 's/^0x//')"
if [ -z "$COMMITMENT_HEX" ] || [ "$COMMITMENT_HEX" = "null" ]; then
  echo "Error: failed to extract commitment hex."
  exit 1
fi

echo "Depositing commitment..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- deposit \
  --from "$SOURCE" \
  --commitment "$COMMITMENT_HEX"

COMMITMENT="$(jq -r '.coin.commitment' "$COIN_FILE")"
jq -n --arg commitment "$COMMITMENT" --arg scope "$SCOPE" \
  '{commitments: [$commitment], scope: $scope}' > "$STATE_FILE"

echo "Creating association set..."
LABEL="$(jq -r '.coin.label' "$COIN_FILE")"
cargo run --bin stellar-coinutils update-association "$ASSOCIATION_FILE" "$LABEL"

ASSOCIATION_ROOT_DECIMAL="$(jq -r '.root' "$ASSOCIATION_FILE")"
if [ -z "$ASSOCIATION_ROOT_DECIMAL" ] || [ "$ASSOCIATION_ROOT_DECIMAL" = "null" ]; then
  echo "Error: failed to extract association root."
  exit 1
fi
ASSOCIATION_ROOT_HEX="$(python3 -c 'import sys; print(hex(int(sys.argv[1]))[2:].zfill(64))' "$ASSOCIATION_ROOT_DECIMAL")"

echo "Setting association root..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- set_association_root \
  --caller "$SOURCE" \
  --association_root "$ASSOCIATION_ROOT_HEX"

echo "Creating withdrawal proof..."
cargo run --bin stellar-coinutils withdraw "$COIN_FILE" "$STATE_FILE" "$ASSOCIATION_FILE" -o "$WITHDRAWAL_FILE"

pushd circuits >/dev/null
node build/main_js/generate_witness.js build/main_js/main.wasm "../$WITHDRAWAL_FILE" "../$EVIDENCE_DIR/witness.wtns"
snarkjs groth16 prove output/main_final.zkey "../$EVIDENCE_DIR/witness.wtns" "../$EVIDENCE_DIR/proof.json" "../$EVIDENCE_DIR/public.json"
popd >/dev/null

echo "Converting proof and public signals for Soroban..."
cargo run --bin stellar-circom2soroban proof "$EVIDENCE_DIR/proof.json" > "$PROOF_HEX_FILE"
cargo run --bin stellar-circom2soroban public "$EVIDENCE_DIR/public.json" > "$PUBLIC_HEX_FILE"
PROOF_HEX="$(sed -n '/^Proof Hex encoding:/{n;p;}' "$PROOF_HEX_FILE" | tr -d '[:space:]' | sed -E 's/^0x//i')"
PUBLIC_HEX="$(sed -n '/^Public signals Hex encoding:/{n;p;}' "$PUBLIC_HEX_FILE" | tr -d '[:space:]' | sed -E 's/^0x//i')"
if [ -z "$PROOF_HEX" ] || [ -z "$PUBLIC_HEX" ]; then
  echo "Error: failed to extract proof or public signals."
  exit 1
fi

echo "Withdrawing from pool..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- withdraw \
  --to "$SOURCE" \
  --proof_bytes "$PROOF_HEX" \
  --pub_signals_bytes "$PUBLIC_HEX"

echo "Verifying resulting contract state..."
stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" -- get_nullifiers
stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" -- get_balance

echo "Setu testnet flow completed successfully."
echo "Evidence written to $EVIDENCE_DIR (gitignored; not part of the release package)."
