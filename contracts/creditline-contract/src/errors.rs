use soroban_sdk::contracterror;

// Error types for the creditline contract
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum CreditLineError {
    NotAdmin = 1,
    InsufficientGuarantee = 2,
    VendorNotActive = 3,
    InsufficientReputation = 4,
    InsufficientLiquidity = 5,
    LoanNotFound = 6,
    LoanNotActive = 7,
    NotBorrower = 8,
    InvalidAmount = 9,
    Overflow = 10,
    Underflow = 11,
    LoanNotOverdue = 12,
    InvalidRepaymentAmount = 13,
    UnauthorizedRepayer = 14,
    TokenNotConfigured = 15,
    InvalidVendor = 16,
    VendorValidationFailed = 17,
    ExposureLimitExceeded = 18,
    LoanNotCancellable = 19,
    ParametersUnavailable = 20,
    ReentrancyDetected = 21,
    LoanInGracePeriod = 22,
    InvalidInstallmentIndex = 23,
    InstallmentAlreadyPaid = 24,
    InvalidLoanStatus = 25,
    NotInitialized = 26,
    InvalidDueDate = 27,
}
