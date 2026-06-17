use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ParametersError {
    AlreadyInitialized = 1,
    NotAdmin = 2,
    InvalidParameters = 3,
    NotInitialized = 4,
    ReentrancyDetected = 5,
}
