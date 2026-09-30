# Ownership notes

Design note for the relayer withdrawal flow lives in
[relayer-withdrawal.md](./relayer-withdrawal.md). This file continues to
document ownership by area because Setu does not yet publish a

.github/CODEOWNERS file.

GitHub CODEoWNERS entries require **real, write-permissioned** user or team
handles. Inventing `@handles` would route review requests incorrectly and fail
CODEOWNERS validation, so this repository documents ownership by **area**
instead until maintainers publish official handles.

## Area ownership (logical)

| Area | Paths | Notes |
| --- | --- | --- |
| Soroban contract | `contract/` | On-chain privacy pool + disclosure verifier |
| Circuits / proofs | `circuits/`, `libs/zk/`, `libs/lean-imt/` | Circom circuits and supporting crypto libs |
| CLI tooling | `cli/` | `coinutils`, `circom2soroban` |
| Demo site | `site/` | Static lab / smoke UI |
| Docs & limitations | `docs/`, top-level `README.md` | Honest prototype / compliance caveats |
| Relayer design | `docs/relayer-withdrawal.md` | Withdrawal gas privacy design only; no implementation claim until tested |

## Review routing (interim)

Until CODEOWNERS is added:

1. Prefer reviewers who recently touched the same paths (GitHub suggested reviewers).
2. For contract or circuit changes, call out both `contract/` and `circuits/` in the PR body.
3. Do not add placeholder @user entries to CODEOWNERS.

## Adding CODEOWNERS later

When official maintainer handles or a GitHub team exist:

1. Create `.github/CODEOWNERS` with path → `@org/team` (or known users).
2. Confirm each handle has write access to this repo.
3. Delete or shrink this note so CODEOWNERS becomes the source of truth.

## Prototype honesty

This project is a testnet-oriented prototype. Ownership metadata here is for
maintainer routing only and does **not** imply production support, audits, or
guaranteed response SLAs. The relayer withdrawal flow is a design document;
it is not an implementation claim and must not be treated as tested or
production-ready. See
[privacy-compliance-limitations.md](./privacy-compliance-limitations.md).
