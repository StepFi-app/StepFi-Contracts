#![no_std]

mod access;
mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod tests;

use errors::Error;
use soroban_sdk::{contract, contractimpl, Address, Env, String};
use types::VendorInfo;

// Export Error type for external use
pub use errors::Error as VendorRegistryError;

#[contract]
pub struct VendorRegistryContract;

#[contractimpl]
impl VendorRegistryContract {
    /// Initializes the contract with an admin
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        storage::set_admin(&env, &admin);

        Ok(())
    }

    fn check_non_reentrant(env: &Env) -> Result<(), Error> {
        if storage::is_reentrancy_locked(env)? {
            return Err(Error::ReentrancyDetected);
        }
        storage::set_reentrancy_locked(env, true);
        Ok(())
    }

    fn exit_non_reentrant(env: &Env) {
        storage::set_reentrancy_locked(env, false);
    }

    /// Registers a new vendor
    pub fn register_vendor(
        env: Env,
        admin: Address,
        vendor: Address,
        name: String,
    ) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        access::require_admin(&env, &admin)?;

        if storage::has_vendor(&env, &vendor) {
            return Err(Error::VendorAlreadyRegistered);
        }

        if name.is_empty() || name.len() > 64 {
            return Err(Error::InvalidName);
        }

        Self::check_non_reentrant(&env)?;

        let info = VendorInfo {
            name: name.clone(),
            registration_date: env.ledger().timestamp(),
            active: true,
            total_sales: 0,
        };

        storage::set_vendor(&env, &vendor, &info);
        storage::increment_vendor_count(&env)?;
        events::publish_vendor_registered(&env, vendor, name);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    /// Deactivates an existing vendor
    pub fn deactivate_vendor(env: Env, admin: Address, vendor: Address) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        access::require_admin(&env, &admin)?;

        Self::check_non_reentrant(&env)?;

        let mut info = storage::get_vendor(&env, &vendor)?;
        info.active = false;
        storage::set_vendor(&env, &vendor, &info);
        events::publish_vendor_status(&env, vendor, false);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    /// Activates an existing vendor
    pub fn activate_vendor(env: Env, admin: Address, vendor: Address) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        access::require_admin(&env, &admin)?;

        Self::check_non_reentrant(&env)?;

        let mut info = storage::get_vendor(&env, &vendor)?;
        info.active = true;
        storage::set_vendor(&env, &vendor, &info);
        events::publish_vendor_status(&env, vendor, true);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    /// Sets a vendor's active status (admin only).
    /// Pass `active = true` to activate, `active = false` to deactivate.
    pub fn set_vendor_status(
        env: Env,
        admin: Address,
        vendor: Address,
        active: bool,
    ) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        access::require_admin(&env, &admin)?;

        Self::check_non_reentrant(&env)?;

        let mut info = storage::get_vendor(&env, &vendor)?;
        info.active = active;
        storage::set_vendor(&env, &vendor, &info);
        events::publish_vendor_status(&env, vendor, active);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    pub fn is_active(env: Env, vendor: Address) -> bool {
        storage::get_vendor(&env, &vendor)
            .map(|info| info.active)
            .unwrap_or(false)
    }

    pub fn get_vendor_info(env: Env, vendor: Address) -> Result<VendorInfo, Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        storage::get_vendor(&env, &vendor)
    }

    pub fn get_vendor_count(env: Env) -> Result<u64, Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        storage::get_vendor_count(&env)
    }
}
