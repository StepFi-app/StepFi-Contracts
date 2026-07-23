use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LiquidityPoolError {
    NotAdmin = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    InvalidAmount = 4,
    InsufficientShares = 5,
    InsufficientLiquidity = 6,
    Overflow = 7,
    Underflow = 8,
    NotCreditLine = 9,
    ZeroTotalShares = 10,
    ReentrancyDetected = 11,
    Paused = 12,
}
