use crate::{
    errors::Error,
    types::{DataKey, VendorInfo},
};
use soroban_sdk::{Address, Env};

pub const PERSISTENT_TTL_THRESHOLD: u32 = 1_036_800;
pub const PERSISTENT_TTL_EXTEND_TO: u32 = 2_073_600;

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn has_vendor(env: &Env, vendor: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Vendor(vendor.clone()))
}

pub fn get_vendor(env: &Env, vendor: &Address) -> Result<VendorInfo, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Vendor(vendor.clone()))
        .ok_or(Error::VendorNotFound)
}

pub fn set_vendor(env: &Env, vendor: &Address, info: &VendorInfo) {
    let key = DataKey::Vendor(vendor.clone());
    env.storage().persistent().set(&key, info);
    extend_persistent_ttl(env, &key);
}

pub fn get_vendor_count(env: &Env) -> Result<u64, Error> {
    Ok(env
        .storage()
        .persistent()
        .get(&DataKey::VendorCount)
        .unwrap_or(0))
}

pub fn increment_vendor_count(env: &Env) -> Result<(), Error> {
    let count = get_vendor_count(env)?;
    let next = count.checked_add(1).ok_or(Error::Overflow)?;
    let key = DataKey::VendorCount;
    env.storage().persistent().set(&key, &next);
    extend_persistent_ttl(env, &key);
    Ok(())
}

pub fn is_reentrancy_locked(env: &Env) -> Result<bool, Error> {
    Ok(env
        .storage()
        .instance()
        .get(&DataKey::Locked)
        .unwrap_or(false))
}

pub fn set_reentrancy_locked(env: &Env, locked: bool) {
    env.storage().instance().set(&DataKey::Locked, &locked);
}

fn extend_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}
