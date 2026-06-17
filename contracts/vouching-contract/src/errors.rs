use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VouchingError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    NotAdmin = 3,
    MentorNotVerified = 4,
    VouchAlreadyActive = 5,
    VouchNotFound = 6,
    VouchNotActive = 7,
    InvalidBoost = 8,
    ReputationCallFailed = 9,
    ReentrancyDetected = 10,
}
