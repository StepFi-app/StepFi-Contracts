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
pub struct VendorInfo {
    pub name: String,
    pub registration_date: u64,
    pub active: bool,
    pub total_sales: u64,
}
