# Setu Protocol & Application Architecture

**Status: prototype.** This document visualizes the end-to-end architecture of Setu on the Stellar **testnet**, explicitly separating **live implemented components (solid lines)** from **mocked UI stubs and design-only components (dashed lines)**. Read alongside [docs/privacy-compliance-limitations.md](./privacy-compliance-limitations.md).

---

## 1. System Topology (Live Testnet vs Stubbed Boundaries)

- **Solid arrows (`-->`)** represent implemented code paths tested in `contract/`, `circuits/`, `libs/zk/`, `cli/`, and `site/`.
- **Dashed arrows (`-.->`)** represent **mocked product-story stubs** (USDC on-ramp, INR off-ramp) or **design-only specifications** (signer-funded relayer).

```mermaid
flowchart TD
    subgraph StubRails["Mocked Fiat / Anchor Boundaries (UI Stubs Only - Not Live)"]
        OnRamp["USDC On-Ramp Stub
(No live SEP-24/SEP-31 anchor)"]
        OffRamp["Bob INR Off-Ramp Stub
(Illustrative UI only)"]
        Relayer["Signer-Funded Relayer
(Design-only: docs/relayer-withdrawal-design.md)"]
    end

    subgraph ClientLayer["Client & Proving Tooling (Local CLI + Static Web Workspace)"]
        SiteUI["Static Web App (site/)
Freighter Wallet + ZK Simulator + Receipt Desk"]
        CoinUtils["cli/coinutils
Poseidon255 Note & Commitment Generator"]
        MainCircuit["circuits/main.circom
Withdrawal + Association Membership Proof"]
        DiscCircuit["circuits/disclosure.circom
5-Signal v2 Selective-Disclosure Receipt"]
        Converter["cli/circom2soroban
snarkjs JSON -> Soroban BLS12-381 Bytes"]
    end

    subgraph SorobanTestnet["Stellar Soroban Testnet (PrivacyPoolsContract)"]
        PoolCore["contract/src/lib.rs
deposit() / withdraw() / set_association_root()
get_vk_commitment()"]
        LeanIMT["libs/lean-imt
Depth-20 Incremental Merkle Tree"]
        ZkVerifier["libs/zk
BLS12-381 Groth16Verifier + Canonical Fr Check"]
        DiscVerifier["contract/src/disclosure.rs
set_disclosure_vk() / verify_disclosure()
get_disclosure_vk_commitment()"]
    end

    subgraph AuditorLayer["Auditor Verification Path"]
        AuditorRecompute["circuits/auditor_recompute.circom
Off-Chain discloseHash & auditorTag Check"]
    end

    OnRamp -. "Stubbed corridor (testnet uses native XLM)" .-> SiteUI
    SiteUI -->|"Freighter testnet auth"| PoolCore
    CoinUtils -->|"commitment = Poseidon255(value, label, pre)"| PoolCore
    MainCircuit -->|"proof.json + public.json (4 signals)"| Converter
    DiscCircuit -->|"proof.json + public.json (5 signals)"| Converter
    Converter -->|"Canonical BLS12-381 wire bytes"| PoolCore
    Converter -->|"Canonical BLS12-381 wire bytes"| DiscVerifier
    MainCircuit -. "Future relayed tx (unimplemented)" .-> Relayer
    Relayer -. "Design-only envelope decoupling" .-> PoolCore
    PoolCore --> LeanIMT
    PoolCore --> ZkVerifier
    DiscVerifier --> ZkVerifier
    PoolCore -. "Illustrative payout display" .-> OffRamp
    DiscCircuit -->|"Private receipt + cleartext (recipientId, purpose, value, receiptNonce)"| AuditorRecompute
    AuditorRecompute -->|"Optional public check (publishes link on-chain)"| DiscVerifier
```

---

## 2. Deposit, Withdrawal & Selective-Disclosure Sequence

```mermaid
sequenceDiagram
    autonumber
    participant Alice as Depositor / Withdrawer
    participant Prover as Local Prover (Circom + circom2soroban)
    participant Contract as Soroban PrivacyPoolsContract (Testnet)
    participant Auditor as Regulator / Auditor

    Note over Alice,Contract: Phase 1: Fixed-Denomination Deposit (1 XLM Testnet)
    Alice->>Prover: Generate (nullifier, secret, label, value)
    Prover-->>Alice: commitment = Poseidon255(value, label, Poseidon255(nullifier, secret))
    Alice->>Contract: deposit(from, commitment) [Transfers FIXED_AMOUNT]
    Contract-->>Alice: leaf_index (stored in LeanIMT)

    Note over Alice,Contract: Phase 2: Private Withdrawal (Unlinkable on-chain)
    Alice->>Prover: Prove circuits/main.circom [nullifierHash, withdrawnValue, stateRoot, associationRoot]
    Prover-->>Alice: proof_bytes + pub_signals_bytes (4 signals)
    Alice->>Contract: withdraw(to, proof_bytes, pub_signals_bytes)
    Contract->>Contract: Check associationRoot != 0, nullifier unused, stateRoot matches, Groth16 valid
    Contract-->>Alice: Transfer FIXED_AMOUNT to `to` + emit ("withdraw") event

    Note over Alice,Auditor: Phase 3: Selective-Disclosure Receipt (v2 Nonce-Separated)
    Alice->>Prover: Prove circuits/disclosure.circom with fresh receiptNonce
    Prover-->>Alice: Receipt [nullifierHash, commitment, discloseHash, auditorTag, receiptNonce]
    Alice->>Auditor: Send Receipt + Cleartext (recipientId, purpose, value, receiptNonce)
    Auditor->>Auditor: Recompute discloseHash & auditorTag (circuits/auditor_recompute.circom)
    Auditor->>Contract: Check get_vk_commitment() & get_disclosure_vk_commitment()
    opt Public On-Chain Verification (Publishes nullifierHash <-> commitment link)
        Auditor->>Contract: verify_disclosure(proof_bytes, pub_signals_bytes)
        Contract-->>Auditor: true (nullifier spent && commitment in leaves && Groth16 valid)
    end
```

---

## 3. Architectural Invariants & Prototype Limits

1. **Cross-Layer Signal Ordering:** `circuits/main.circom` (4 signals) and `circuits/disclosure.circom` (5 signals) must match `cli/circom2soroban`, `libs/zk`, and `contract/src/` byte-for-byte (pinned by `libs/zk/testdata/public_signals/`).
2. **Honest Boundary Separation:** The Soroban contract and BLS12-381 Groth16 verifier execute real cryptographic verification on Stellar testnet, whereas fiat/USDC/INR anchor rails in `site/` are mock UI stubs and the Groth16 ceremony is a local single-contributor setup (`not production-secure`).
