use crate::{errors::Error, storage};
use soroban_sdk::{Address, Env};

pub fn require_admin(env: &Env, admin: &Address) -> Result<(), Error> {
    admin.require_auth();
    let stored_admin = storage::get_admin(env)?;
    if *admin != stored_admin {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
