use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    VendorAlreadyRegistered = 3,
    VendorNotFound = 4,
    InvalidName = 5,
    Unauthorized = 6,
    Overflow = 7,
    ReentrancyDetected = 8,
}
