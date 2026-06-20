param(
    [string]$ContractId = "",
    [string]$Source = "demo_user",
    [string]$Network = "testnet",
    [string]$TokenAddress = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
    [string]$Stellar = "",
    [string]$Snarkjs = ""
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

$RepoRoot = Split-Path -Parent $PSScriptRoot
$WorkspaceRoot = Split-Path -Parent $RepoRoot

function Resolve-FirstExistingPath($Candidates) {
    foreach ($candidate in $Candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            return (Resolve-Path $candidate).Path
        }
    }
    return ""
}

function Resolve-CommandPath($Name) {
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }
    return ""
}

function Extract-Hex($Text) {
    (($Text -split "`n") | Select-String -Pattern '^[0-9a-fA-F]+$' | Select-Object -Last 1).Matches.Value
}

if (-not $Stellar) {
    $Stellar = Resolve-FirstExistingPath @(
        (Join-Path $WorkspaceRoot "bin\stellar.exe"),
        (Resolve-CommandPath "stellar")
    )
}
if (-not $Snarkjs) {
    $Snarkjs = Resolve-FirstExistingPath @(
        (Join-Path $WorkspaceRoot "node_modules\snarkjs\build\cli.cjs"),
        (Resolve-CommandPath "snarkjs")
    )
}

if (-not (Test-Path $Stellar)) {
    throw "stellar CLI not found. Pass -Stellar or add stellar to PATH."
}
if (-not (Test-Path $Snarkjs)) {
    throw "snarkjs CLI not found. Run npm install in the workspace or pass -Snarkjs."
}

$pathParts = @(
    (Join-Path $WorkspaceRoot "node_modules\.bin"),
    (Join-Path $WorkspaceRoot "bin"),
    (Join-Path $HOME ".cargo\bin")
) | Where-Object { Test-Path $_ }
$env:PATH = (($pathParts + $env:PATH) -join [IO.Path]::PathSeparator)

if (-not $ContractId) {
    cargo build --target wasm32v1-none --release -p privacy-pools
    & $Stellar contract optimize `
        --wasm target/wasm32v1-none/release/privacy_pools.wasm `
        --wasm-out target/wasm32v1-none/release/privacy_pools.optimized.wasm

    $vkHex = Extract-Hex (cargo run -q --bin stellar-circom2soroban vk circuits/output/main_verification_key.json 2>$null)
    if (-not $vkHex) { throw "failed to extract withdrawal VK hex" }

    $deployOut = & $Stellar contract deploy `
        --wasm target/wasm32v1-none/release/privacy_pools.optimized.wasm `
        --source $Source `
        --network $Network `
        -- `
        --vk_bytes $vkHex `
        --token_address $TokenAddress `
        --admin $Source 2>&1

    $deployText = $deployOut -join "`n"
    $deployText | Set-Content -LiteralPath deploy_output.txt
    $ContractId = ([regex]::Matches($deployText, 'C[A-Z0-9]{55}') | Select-Object -Last 1).Value
    if (-not $ContractId) { throw "failed to extract deployed contract id" }
    $ContractId | Set-Content -NoNewline -LiteralPath setu_contract_id.txt
}

Write-Host "Contract: $ContractId"

cargo run -q --bin stellar-coinutils generate demo_pool -o demo_coin.json
$coinFile = Get-Content -Raw -LiteralPath demo_coin.json | ConvertFrom-Json
$commitmentHex = $coinFile.commitment_hex -replace '^0x', ''

& $Stellar contract invoke --id $ContractId --source $Source --network $Network -- deposit --from $Source --commitment $commitmentHex

$state = [ordered]@{
    commitments = @($coinFile.coin.commitment)
    scope = "demo_pool"
} | ConvertTo-Json -Depth 4
Set-Content -LiteralPath demo_state.json -Value $state

Remove-Item -LiteralPath demo_association.json -ErrorAction SilentlyContinue
cargo run -q --bin stellar-coinutils update-association demo_association.json $coinFile.coin.label
$association = Get-Content -Raw -LiteralPath demo_association.json | ConvertFrom-Json
$rootHex = node -e "console.log(BigInt(process.argv[1]).toString(16).padStart(64,'0'))" $association.root

& $Stellar contract invoke --id $ContractId --source $Source --network $Network -- set_association_root --caller $Source --association_root $rootHex

cargo run -q --bin stellar-coinutils withdraw demo_coin.json demo_state.json demo_association.json -o withdrawal_input.json
node circuits/build/main_js/generate_witness.js circuits/build/main_js/main.wasm withdrawal_input.json circuits/main.wtns
node $Snarkjs groth16 prove circuits/output/main_final.zkey circuits/main.wtns circuits/main_proof.json circuits/main_public.json

$proofHex = Extract-Hex (cargo run -q --bin stellar-circom2soroban proof circuits/main_proof.json 2>$null)
$publicHex = Extract-Hex (cargo run -q --bin stellar-circom2soroban public circuits/main_public.json 2>$null)
if (-not $proofHex -or -not $publicHex) { throw "failed to extract withdrawal proof/public hex" }

& $Stellar contract invoke --id $ContractId --source $Source --network $Network -- withdraw --to $Source --proof_bytes $proofHex --pub_signals_bytes $publicHex

$withdrawPublic = Get-Content -Raw -LiteralPath circuits/main_public.json | ConvertFrom-Json
$private = [ordered]@{
    value = [string]$coinFile.coin.value
    label = [string]$coinFile.coin.label
    nullifier = [string]$coinFile.coin.nullifier
    secret = [string]$coinFile.coin.secret
    recipientId = "11111"
    purpose = "7"
    viewingKey = "99999"
} | ConvertTo-Json -Depth 4
Set-Content -LiteralPath circuits/build_disc/fresh_private.json -Value $private

node circuits/build_disc/disclosure_witness_js/generate_witness.js circuits/build_disc/disclosure_witness_js/disclosure_witness.wasm circuits/build_disc/fresh_private.json circuits/build_disc/fresh_twin.wtns
node $Snarkjs wtns export json circuits/build_disc/fresh_twin.wtns circuits/build_disc/fresh_twin.json
node -e "const w=require('./circuits/build_disc/fresh_twin.json'), p=require('./circuits/build_disc/fresh_private.json'); const o=Object.assign({}, p, {nullifierHash:w[1], commitment:w[2], discloseHash:w[3], auditorTag:w[4]}); require('fs').writeFileSync('circuits/build_disc/fresh_input.json', JSON.stringify(o, null, 2));"

$freshInput = Get-Content -Raw -LiteralPath circuits/build_disc/fresh_input.json | ConvertFrom-Json
if ([string]$freshInput.nullifierHash -ne [string]$withdrawPublic[0]) { throw "disclosure nullifierHash mismatch" }
if ([string]$freshInput.commitment -ne [string]$coinFile.coin.commitment) { throw "disclosure commitment mismatch" }

node circuits/build_disc/disclosure_js/generate_witness.js circuits/build_disc/disclosure_js/disclosure.wasm circuits/build_disc/fresh_input.json circuits/build_disc/fresh_disc.wtns
node $Snarkjs groth16 prove circuits/build_disc/disc_final.zkey circuits/build_disc/fresh_disc.wtns circuits/build_disc/fresh_proof.json circuits/build_disc/fresh_public.json
node $Snarkjs groth16 verify circuits/build_disc/disc_vk.json circuits/build_disc/fresh_public.json circuits/build_disc/fresh_proof.json

$dvkHex = Extract-Hex (cargo run -q --bin stellar-circom2soroban vk circuits/build_disc/disc_vk.json 2>$null)
$dproofHex = Extract-Hex (cargo run -q --bin stellar-circom2soroban proof circuits/build_disc/fresh_proof.json 2>$null)
$dpublicHex = Extract-Hex (cargo run -q --bin stellar-circom2soroban public circuits/build_disc/fresh_public.json 2>$null)
if (-not $dvkHex -or -not $dproofHex -or -not $dpublicHex) { throw "failed to extract disclosure hex" }

& $Stellar contract invoke --id $ContractId --source $Source --network $Network -- set_disclosure_vk --caller $Source --dvk_bytes $dvkHex
& $Stellar contract invoke --id $ContractId --source $Source --network $Network -- verify_disclosure --proof_bytes $dproofHex --pub_signals_bytes $dpublicHex

$last = $dpublicHex.Substring($dpublicHex.Length - 1, 1)
$tampered = $dpublicHex.Substring(0, $dpublicHex.Length - 1) + $(if ($last -eq "0") { "1" } else { "0" })
& $Stellar contract invoke --id $ContractId --source $Source --network $Network -- verify_disclosure --proof_bytes $dproofHex --pub_signals_bytes $tampered

Write-Host "Done. Valid disclosure should be true; tampered disclosure should be false."
