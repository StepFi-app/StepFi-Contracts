use soroban_sdk::{panic_with_error, Address, Env};

use crate::{errors::ParametersError, storage};

pub fn require_admin(env: &Env, caller: &Address) {
    let admin = storage::get_admin(env).unwrap_or_else(|err| panic_with_error!(env, err));
    if admin != *caller {
        panic_with_error!(env, ParametersError::NotAdmin);
    }
}
