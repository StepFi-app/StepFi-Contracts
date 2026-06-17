use soroban_sdk::contracterror;

// Error types for the reputation contract
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ReputationError {
    NotAdmin = 1,
    NotUpdater = 2,
    OutOfBounds = 3,
    Overflow = 4,
    Underflow = 5,
    NotInitialized = 6,
    ReentrancyDetected = 7,
}
