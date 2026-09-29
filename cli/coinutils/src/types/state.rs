use serde::{Deserialize, Serialize};

/// State file for the coinutils CLI.
///
/// This type is part of the withdrawal relayer design documented in
/// `docs/relayer-withdrawal-privacy.md`. The current withdrawal flow has the
/// recipient sign and pay the Soroban transaction fee directly, which leaks
/// metadata outside the ZK proof. The relayer flow introduces an optional
/// relayer address and fee declaration so the recipient address does not have
/// to be the fee-paying account. This is a prototype data model only; no
/// relayer network is implemented or tested by this file.
#[derive(Serialize, Deserialize)]
pub struct StateFile {
    pub commitments: Vec<String>,
    pub scope: String,
    pub association_set: Option<Vec<String>>, // Optional association set labels
    /// Optional relayer address that submits the withdrawal transaction.
    /// When `Some`, the recipient address is not required to pay the Soroban
    /// fee directly. This is a design field for the relayer flow and does not
    /// imply a relayer is operational or tested.
    pub relayer: Option<String>,
    /// Optional fee amount (in the native Sororan asset) to be paid to the
    /// relayer. This is declared in the state file for the design and must be
    /// enforced on-chain before any privacy claim is made.
    pub relayer_fee: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct AssociationSetFile {
    pub labels: Vec<String>,
    pub scope: String,
    pub root: Option<String>, // Merkle tree root of the association set
}
