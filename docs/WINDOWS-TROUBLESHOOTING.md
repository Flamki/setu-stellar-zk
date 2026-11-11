# Windows & WSL2 Local Setup Troubleshooting

**Status: prototype.** Setu targets the **Stellar testnet** using single-contributor local Groth16 proving keys (`not production-secure`, `not audited`, and off-ramp/anchor flows remain `stub` prototypes; see [Privacy & Compliance Limitations](privacy-compliance-limitations.md)).

This repository mixes POSIX bash scripts (`scripts/build-circuits.sh`, `scripts/disclosure_e2e.sh`, `scripts/testnet_withdraw_e2e.sh`), a macOS-oriented `Makefile`, Node `.cjs` checks (`scripts/check-prereqs.cjs`, `scripts/docs-check.cjs`), and a native PowerShell testnet runner (`scripts/live_testnet_e2e.ps1`). The checklist and troubleshooting notes below document every Windows-specific path and CLI quirk so contributors can get running without guesswork.

---

## 1. New Windows Setup Checklist

Choose **Option A (WSL2 Ubuntu)** if you need to recompile `.circom` circuits with `scripts/build-circuits.sh`, or **Option B (Native PowerShell)** if you are running Rust tests, static site checks, or `scripts/live_testnet_e2e.ps1` against prebuilt artifacts.

### Option A: WSL2 Ubuntu 22.04 / 24.04 (Recommended for Full Circuit Builds)

- [ ] Install WSL2 from an elevated PowerShell prompt:
  ```powershell
  wsl --install -d Ubuntu
  ```
- [ ] Configure Git inside WSL2 to keep LF line endings on `.sh` scripts and clone inside the Linux filesystem (`~/workspace`, **not** `/mnt/c/...`, for fast Cargo builds):
  ```bash
  git config --global core.autocrlf input
  mkdir -p ~/workspace && cd ~/workspace
  git clone https://github.com/Flamki/setu-stellar-zk.git
  ```
- [ ] Install JS dependencies (`snarkjs@0.7.x` and `circomlib`) so both `../node_modules` and `./node_modules` lookups work:
  ```bash
  cd ~/workspace/setu-stellar-zk
  npm install snarkjs@^0.7.0 circomlib
  ```
- [ ] Install Rust (`rustup`), the Soroban Wasm target, `stellar-cli` `26.x`, and Rust `circom` `2.2.x`:
  ```bash
  rustup target add wasm32v1-none
  cargo install --locked stellar-cli --version 26.0.0
  ```
- [ ] Run the prerequisite, docs, site, and Rust workspace checks:
  ```bash
  node scripts/check-prereqs.cjs
  node scripts/docs-check.cjs
  npm --prefix site run smoke
  cargo test --workspace
  ```

### Option B: Native Windows PowerShell (Rust Tests, Site Smoke & Testnet Runner)

- [ ] Clone with LF line endings preserved so bash scripts under Git Bash / WSL stay runnable:
  ```powershell
  git config --global core.autocrlf input
  mkdir C:\dev\setu-workspace
  cd C:\dev\setu-workspace
  git clone https://github.com/Flamki/setu-stellar-zk.git
  ```
- [ ] Install `snarkjs` and `circomlib` in the **parent workspace directory** (`C:\dev\setu-workspace`) or inside the repo using `npm.cmd` (avoids PowerShell `.ps1` execution-policy errors):
  ```powershell
  cd C:\dev\setu-workspace
  npm.cmd init -y
  npm.cmd install snarkjs@^0.7.0 circomlib
  cd C:\dev\setu-workspace\setu-stellar-zk
  ```
- [ ] Add the Soroban Wasm target to `rustup`:
  ```powershell
  rustup target add wasm32v1-none
  ```
- [ ] Verify prerequisites and run offline checks (no secrets, testnet account, or circuit rebuild required because deterministic fixtures live in `libs/zk/testdata/`):
  ```powershell
  node scripts/check-prereqs.cjs
  node scripts/docs-check.cjs
  npm.cmd --prefix site run smoke
  cargo test --workspace
  ```

---

## 2. Repo-Specific Windows Path & CLI Quirks

### 2.1 PowerShell Execution Policy, Syntax & `npm.cmd`

- **Error message (`npm` / `npx` or `.ps1` blocked):**
  ```text
  npm : File C:\Program Files\nodejs\npm.ps1 cannot be loaded because running scripts is disabled on this system.
  + CategoryInfo          : SecurityError: (:) [], PSSecurityException
  + FullyQualifiedErrorId : UnauthorizedAccess
  ```
- **Why it happens:** Windows PowerShell defaults to the `Restricted` execution policy, which blocks both `npm.ps1` / `npx.ps1` shims and `.\scripts\live_testnet_e2e.ps1`. (Note: `node scripts/check-prereqs.cjs` is unaffected because it spawns tools via `cmd.exe` with `shell: true` and `PATHEXT`.)
- **Copyable fixes:**
  1. Invoke `npm.cmd` and `npx.cmd` directly instead of the `.ps1` wrappers:
     ```powershell
     npm.cmd --prefix site run smoke
     npx.cmd snarkjs --version
     ```
  2. Run `scripts/live_testnet_e2e.ps1` with a one-shot process bypass (never requires changing machine-wide policy):
     ```powershell
     powershell -ExecutionPolicy Bypass -File .\scripts\live_testnet_e2e.ps1 `
       -ContractId CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT
     ```
     Or unlock only the current PowerShell window:
     ```powershell
     Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
     ```
  3. Use PowerShell environment-variable and line-continuation syntax (`$env:NAME = "value"` and backtick `` ` ``, not POSIX `NAME=value \`):
     ```powershell
     $env:SETU_SUPABASE_URL = "https://<project-ref>.supabase.co"
     $env:SETU_SUPABASE_ANON_KEY = "<anon-or-publishable-key>"
     ```
     *(Never commit real keys or service-role secrets; `site/auth-config.js` is gitignored.)*

### 2.2 Stellar CLI (`stellar` 26.x) Path Resolution

- **Error messages:**
  ```text
  stellar CLI not found. Pass -Stellar or add stellar to PATH.
  ```
  ```text
  WARN Stellar CLI  26.x  25.x.x (via stellar)
         Found a Stellar CLI outside the 26.x series this repo was built against
  ```
  ```text
  error[E0463]: can't find crate for `core`
    = note: the `wasm32v1-none` target may not be installed
  ```
- **How Setu resolves `stellar`:**
  - `scripts/live_testnet_e2e.ps1` sets `$WorkspaceRoot = Split-Path -Parent $RepoRoot` (**one directory above** `setu-stellar-zk`) and checks, in order:
    1. `-Stellar <explicit-path>`
    2. `<parent-workspace>\bin\stellar.exe`
    3. `stellar` on `PATH` (`Get-Command stellar`)
  - `scripts/check-prereqs.cjs` requires the `stellar` `26.x` binary specifically (`soroban` is only inspected as a legacy fallback to warn about the CLI rename).
- **Copyable fixes:**
  ```powershell
  # Ensure Cargo's bin directory is on PATH in the current PowerShell session
  $env:PATH = "$HOME\.cargo\bin;$env:PATH"
  stellar --version

  # Or pass the binary path explicitly to live_testnet_e2e.ps1
  powershell -ExecutionPolicy Bypass -File .\scripts\live_testnet_e2e.ps1 `
    -Stellar "$HOME\.cargo\bin\stellar.exe" `
    -ContractId CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT

  # Install the missing Soroban Rust Wasm target if cargo build fails with E0463
  rustup target add wasm32v1-none
  ```

### 2.3 `snarkjs` (`0.7.x`) Path Resolution (`..\node_modules` vs `-Snarkjs`)

- **Error messages:**
  ```text
  snarkjs CLI not found. Run npm install in the workspace or pass -Snarkjs.
  ```
  ```text
  Error: Cannot find module 'C:\...\snarkjs.cmd'
  ```
- **How Setu resolves `snarkjs`:**
  - `scripts/live_testnet_e2e.ps1` executes **`node $Snarkjs groth16 prove ...`** (line 112). Because it passes `$Snarkjs` directly to `node`, `$Snarkjs` **must point to the JavaScript CLI file (`cli.cjs`)**, **not** `snarkjs.cmd` or `snarkjs.ps1`!
  - By default, `live_testnet_e2e.ps1` checks `<parent-workspace>\node_modules\snarkjs\build\cli.cjs` first, then `Get-Command snarkjs` (which on Windows can resolve to `snarkjs.ps1` or `snarkjs.cmd` and fail when passed to `node`).
  - `scripts/check-prereqs.cjs` checks `snarkjs --version` on `PATH`, `./node_modules/snarkjs/package.json`, and `../node_modules/snarkjs/package.json` for `0.7.x` (it deliberately avoids `npx` so it never triggers hidden network downloads).
- **Copyable fixes:**
  ```powershell
  # Option 1: Install in the parent workspace directory expected by README & live_testnet_e2e.ps1
  Push-Location ..
  npm.cmd install snarkjs@^0.7.0 circomlib
  Pop-Location

  # Option 2: Install inside the repo and pass cli.cjs explicitly via -Snarkjs
  npm.cmd install --no-save snarkjs@^0.7.0 circomlib
  powershell -ExecutionPolicy Bypass -File .\scripts\live_testnet_e2e.ps1 `
    -Snarkjs "$PWD\node_modules\snarkjs\build\cli.cjs" `
    -ContractId CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT
  ```

### 2.4 `circomlib` Lookup (`CIRCOMLIB`) & `circom` `2.2.x`

- **Error messages:**
  ```text
  FAIL circomlib  present (no version pinned)  not found
         Run `npm install` in the parent workspace (see README > Run Locally), or set $CIRCOMLIB to a circomlib checkout's `circuits` directory.
  ```
  ```text
  error[P1012]: File "/opt/homebrew/lib/node_modules/circomlib/circuits/poseidon.circom" not found
  ```
  ```text
  WARN circom  2.2.x  0.5.46
         This repo compiles circuits with `--prime bls12381`, which needs circom 2.2.x specifically.
  ```
- **How Setu resolves `circomlib` & `circom`:**
  - The root `Makefile` hardcodes `CIRCOMLIB=/opt/homebrew/lib/node_modules/circomlib/circuits` (macOS Homebrew), so bare `make` fails on Windows and Linux.
  - Prefer `bash scripts/build-circuits.sh` (in WSL2 or Git Bash), which searches `$CIRCOMLIB`, `../node_modules/circomlib/circuits`, `./node_modules/circomlib/circuits`, `$(npm root)/circomlib/circuits`, and `$(npm root -g)/circomlib/circuits`.
  - `CIRCOMLIB` must point to the **`circuits` subdirectory** inside `circomlib` (for example `.../node_modules/circomlib/circuits`), not the package root (`scripts/check-prereqs.cjs` reads `../package.json` relative to `$CIRCOMLIB`).
  - Do **not** install `circom` from npm (`npm install -g circom` installs the deprecated JS `0.5.x` compiler without `--prime bls12381`). Build Rust `circom` `2.2.x` from `https://github.com/iden3/circom`.
- **Copyable fixes:**
  ```powershell
  # PowerShell: point CIRCOMLIB at circomlib\circuits for check-prereqs.cjs
  $env:CIRCOMLIB = (Resolve-Path .\node_modules\circomlib\circuits).Path
  node scripts/check-prereqs.cjs
  ```
  ```bash
  # WSL2 / Git Bash: regenerate circuits without touching the macOS Makefile path
  export CIRCOMLIB="$PWD/node_modules/circomlib/circuits"
  bash scripts/build-circuits.sh --check
  bash scripts/build-circuits.sh
  ```

### 2.5 Common Secondary Pitfalls (`CRLF`, Port Conflicts & `file://` Freighter)

- **`$'\r': command not found` when running `scripts/build-circuits.sh` or `scripts/disclosure_e2e.sh`:**
  - *Cause:* Git checked out `.sh` files with Windows `CRLF` line endings.
  - *Fix:*
    ```bash
    git config core.autocrlf input
    sed -i 's/\r$//' scripts/*.sh
    ```
- **`EADDRINUSE` on port `4180` or `4174` during `site` smoke/dev server:**
  - *Cause:* Another local server is bound to the default port.
  - *Fix:*
    ```powershell
    $env:SMOKE_PORT = "4195"
    npm.cmd --prefix site run smoke
    ```
- **Freighter Wallet does not appear when opening `site\index.html` from Explorer:**
  - *Cause:* Browser wallet extensions do not inject `window.freighterApi` on `file://` URLs.
  - *Fix:* Serve `site/` over HTTP (`npm.cmd --prefix site run dev`) and open `http://127.0.0.1:4174/index.html#send`.

---

## 3. Prototype & Security Boundaries

- **No production keys or secrets:** Local circuit builds generate single-contributor development `zkey` artifacts (`not production-secure`, `trusted setup` ceremony not performed). Never pass mainnet secret keys or commit `.env` / `site/auth-config.js` files.
- **Testnet & stubbed off-ramp scope:** On-chain steps in `scripts/live_testnet_e2e.ps1` target the Stellar `testnet` only, selective-disclosure inputs include prover-asserted metadata, and off-ramp/anchor settlement flows remain `stub` prototypes. Read [Privacy & Compliance Limitations](privacy-compliance-limitations.md) before modifying public claims.
