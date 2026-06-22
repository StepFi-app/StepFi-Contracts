#![no_std]

mod access;
mod errors;
mod events;
mod safe_math;
mod storage;
mod types;

#[cfg(test)]
mod tests;

use errors::Error;
use soroban_sdk::{contract, contractimpl, Address, Env, String};
use types::{VendorInfo, VendorStatus};

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
            status: VendorStatus::Pending,
            total_sales: 0,
        };

        storage::set_vendor(&env, &vendor, &info);
        storage::increment_vendor_count(&env)?;
        events::publish_vendor_registered(&env, vendor, name);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    /// Approves a pending vendor so they can receive loans
    pub fn approve_vendor(env: Env, admin: Address, vendor: Address) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        access::require_admin(&env, &admin)?;

        Self::check_non_reentrant(&env)?;

        let mut info = storage::get_vendor(&env, &vendor)?;
        if info.status != VendorStatus::Pending {
            return Err(Error::VendorNotPending);
        }
        info.status = VendorStatus::Approved;
        storage::set_vendor(&env, &vendor, &info);
        events::publish_vendor_status(&env, vendor, VendorStatus::Approved);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    /// Suspends an approved vendor, preventing new loans
    pub fn suspend_vendor(env: Env, admin: Address, vendor: Address) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        access::require_admin(&env, &admin)?;

        Self::check_non_reentrant(&env)?;

        let mut info = storage::get_vendor(&env, &vendor)?;
        info.status = VendorStatus::Suspended;
        storage::set_vendor(&env, &vendor, &info);
        events::publish_vendor_status(&env, vendor, VendorStatus::Suspended);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    /// Deactivates an existing vendor (legacy — maps to Suspended)
    pub fn deactivate_vendor(env: Env, admin: Address, vendor: Address) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        access::require_admin(&env, &admin)?;

        Self::check_non_reentrant(&env)?;

        let mut info = storage::get_vendor(&env, &vendor)?;
        info.status = VendorStatus::Suspended;
        storage::set_vendor(&env, &vendor, &info);
        events::publish_vendor_status(&env, vendor, VendorStatus::Suspended);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    /// Activates an existing vendor (legacy — maps to Approved)
    pub fn activate_vendor(env: Env, admin: Address, vendor: Address) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        access::require_admin(&env, &admin)?;

        Self::check_non_reentrant(&env)?;

        let mut info = storage::get_vendor(&env, &vendor)?;
        info.status = VendorStatus::Approved;
        storage::set_vendor(&env, &vendor, &info);
        events::publish_vendor_status(&env, vendor, VendorStatus::Approved);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    /// Sets a vendor's active status (admin only, legacy).
    /// Pass `active = true` to approve, `active = false` to suspend.
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
        let new_status = if active {
            VendorStatus::Approved
        } else {
            VendorStatus::Suspended
        };
        info.status = new_status.clone();
        storage::set_vendor(&env, &vendor, &info);
        events::publish_vendor_status(&env, vendor, new_status);

        Self::exit_non_reentrant(&env);

        Ok(())
    }

    pub fn is_active(env: Env, vendor: Address) -> bool {
        storage::get_vendor(&env, &vendor)
            .map(|info| info.status == VendorStatus::Approved)
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

    /// Get numeric contract version
    pub fn get_version(env: Env) -> u32 {
        storage::get_version(&env).unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err))
    }

    /// Upgrade the contract WASM — admin only
    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin = storage::get_admin(&env).unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err));
        admin.require_auth();

        let old = storage::get_version(&env).unwrap_or(1u32);
        let new = old.checked_add(1).unwrap_or(old);
        storage::set_version(&env, new);

        env.deployer().update_current_contract_wasm(new_wasm_hash);
        events::emit_contract_upgraded(&env, old, new);
    }
}
