use soroban_sdk::{Address, Env, Vec};

use crate::{
    errors::VouchingError,
    types::{DataKey, VouchRecord},
};

pub const PERSISTENT_TTL_THRESHOLD: u32 = 1_036_800;
pub const PERSISTENT_TTL_EXTEND_TO: u32 = 2_073_600;

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn get_admin(env: &Env) -> Result<Address, VouchingError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(VouchingError::NotInitialized)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_reputation_contract(env: &Env) -> Result<Address, VouchingError> {
    env.storage()
        .instance()
        .get(&DataKey::ReputationContract)
        .ok_or(VouchingError::NotInitialized)
}

pub fn set_reputation_contract(env: &Env, reputation_contract: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::ReputationContract, reputation_contract);
}

pub fn get_vouch_boost(env: &Env) -> Result<u32, VouchingError> {
    env.storage()
        .instance()
        .get(&DataKey::VouchBoost)
        .ok_or(VouchingError::NotInitialized)
}

pub fn set_vouch_boost(env: &Env, boost_amount: u32) {
    env.storage()
        .instance()
        .set(&DataKey::VouchBoost, &boost_amount);
}

pub fn is_mentor(env: &Env, mentor: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Mentor(mentor.clone()))
        .unwrap_or(false)
}

pub fn set_mentor(env: &Env, mentor: &Address, verified: bool) {
    let key = DataKey::Mentor(mentor.clone());
    env.storage().persistent().set(&key, &verified);
    extend_persistent_ttl(env, &key);
}

pub fn get_vouch(
    env: &Env,
    mentor: &Address,
    learner: &Address,
) -> Result<VouchRecord, VouchingError> {
    env.storage()
        .persistent()
        .get(&DataKey::Vouch(mentor.clone(), learner.clone()))
        .ok_or(VouchingError::VouchNotFound)
}

pub fn set_vouch(env: &Env, record: &VouchRecord) {
    let key = DataKey::Vouch(record.mentor.clone(), record.learner.clone());
    env.storage().persistent().set(&key, record);
    extend_persistent_ttl(env, &key);
}

pub fn get_learner_mentors(env: &Env, learner: &Address) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::LearnerVouches(learner.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn add_learner_mentor(env: &Env, learner: &Address, mentor: &Address) {
    let mut mentors = get_learner_mentors(env, learner);
    if !mentors.contains(mentor) {
        mentors.push_back(mentor.clone());
        let key = DataKey::LearnerVouches(learner.clone());
        env.storage().persistent().set(&key, &mentors);
        extend_persistent_ttl(env, &key);
    }
}

pub fn is_reentrancy_locked(env: &Env) -> Result<bool, VouchingError> {
    Ok(env
        .storage()
        .instance()
        .get(&DataKey::Locked)
        .unwrap_or(false))
}

pub fn set_reentrancy_locked(env: &Env, locked: bool) {
    env.storage().instance().set(&DataKey::Locked, &locked);
}

pub fn extend_vouch_ttl(env: &Env, mentor: &Address, learner: &Address) {
    extend_persistent_ttl(env, &DataKey::Vouch(mentor.clone(), learner.clone()));
}

fn extend_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}
