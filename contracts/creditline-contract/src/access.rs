use soroban_sdk::{panic_with_error, Address, Env};

use crate::errors::CreditLineError;
use crate::storage;

/// Require that the given address is the admin, otherwise panic with NotAdmin error
pub fn require_admin(env: &Env, caller: &Address) {
    let admin = storage::get_admin(env).unwrap_or_else(|err| panic_with_error!(env, err));

    if caller != &admin {
        panic_with_error!(env, CreditLineError::NotAdmin);
    }
}
