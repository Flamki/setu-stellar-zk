/// Configuration constants for the coinutils application
/// Default coin denomination, in the smallest unit of the pool's token.
///
/// This must equal the denomination the pool contract was deployed with
/// (`get_denomination`). It is a raw base-unit amount, not a currency amount:
/// for a 7-decimal Stellar asset such as the native XLM contract,
/// `1_000_000_000` is 100 units, not 1. Pass `--denomination` to `generate`
/// for pools deployed with a different value.
pub const DEFAULT_DENOMINATION: i128 = 1_000_000_000;
pub const TREE_DEPTH: u32 = 20;
pub const ASSOCIATION_TREE_DEPTH: u32 = 2;
pub const MAX_ASSOCIATION_LABELS: usize = 4;
