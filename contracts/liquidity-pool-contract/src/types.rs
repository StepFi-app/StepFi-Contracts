use soroban_sdk::contracttype;

/// Pool statistics returned by get_pool_stats
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolStats {
    pub total_liquidity: i128,
    pub locked_liquidity: i128,
    pub available_liquidity: i128,
    pub total_shares: i128,
    /// Share price expressed in basis points (10000 = $1.00)
    pub share_price: i128,
}

// Fee split constants (basis points, sum = 10000)
pub const LP_FEE_BPS: i128 = 8500; // 85% to liquidity providers
pub const PROTOCOL_FEE_BPS: i128 = 1000; // 10% to protocol treasury
pub const MERCHANT_FEE_BPS: i128 = 500; // 5% to merchant incentive fund
pub const TOTAL_BPS: i128 = 10000;

/// Precision used for share price calculation (10000 = 1.0)
pub const SHARE_PRICE_PRECISION: i128 = 10_000;

/// Minimum deposit / withdrawal to prevent rounding exploits
pub const MIN_AMOUNT: i128 = 1;

/// Pending timelocked contract upgrade proposal
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: soroban_sdk::BytesN<32>,
    pub proposed_at: u64,
    pub unlock_at: u64,
}

/// Protocol parameters structure for cross-contract governance fetching
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolParameters {
    pub min_guarantee_percent: i128,
    pub min_reputation_threshold: u32,
    pub full_repayment_reward: u32,
    pub default_penalty: u32,
    pub large_loan_threshold: i128,
    pub large_loan_default_penalty: u32,
    pub base_interest_bps: u32,
    pub grace_period_seconds: u64,
    pub upgrade_delay_seconds: u64,
}
