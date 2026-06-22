use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    // Instance storage
    Admin,
    Locked,

    // Persistent storage
    Vendor(Address),
    VendorCount,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VendorStatus {
    Pending = 0,
    Approved = 1,
    Suspended = 2,
    Rejected = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VendorInfo {
    pub name: String,
    pub registration_date: u64,
    pub status: VendorStatus,
    pub total_sales: u64,
}
