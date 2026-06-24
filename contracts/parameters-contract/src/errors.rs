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
    Overflow = 6,
    Underflow = 7,
    MultisigAlreadyConfigured = 8,
    MultisigNotConfigured = 9,
    NotSigner = 10,
    InvalidThreshold = 11,
    DuplicateSigner = 12,
    ProposalNotFound = 13,
    ProposalExpired = 14,
    ProposalAlreadyExecuted = 15,
    DuplicateSignature = 16,
    ThresholdNotMet = 17,
}
