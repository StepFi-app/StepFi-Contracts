use soroban_sdk::{Address, Env, String, Symbol};

pub fn publish_vendor_registered(env: &Env, vendor: Address, name: String) {
    let topics = (Symbol::new(env, "MERCHTREG"), vendor);
    env.events().publish(topics, name);
}

pub fn publish_vendor_status(env: &Env, vendor: Address, active: bool) {
    let topics = (Symbol::new(env, "MERCHTSTATUS"), vendor);
    env.events().publish(topics, active);
}

pub fn emit_contract_upgraded(env: &Env, old_version: u32, new_version: u32) {
    env.events().publish(
        (Symbol::new(env, "CONTRACTUPGRADED"),),
        (old_version, new_version, env.ledger().timestamp()),
    );
}
