use soroban_sdk::{Address, Env, Symbol};

pub fn emit_mentor_vouched(env: &Env, mentor: &Address, learner: &Address, boost_amount: u32) {
    env.events().publish(
        (Symbol::new(env, "MENTORVOUCHED"), mentor, learner),
        boost_amount,
    );
}

pub fn emit_vouch_revoked(env: &Env, mentor: &Address, learner: &Address, boost_amount: u32) {
    env.events().publish(
        (Symbol::new(env, "VOUCHREVOKED"), mentor, learner),
        boost_amount,
    );
}

pub fn emit_mentor_verified(env: &Env, mentor: &Address, verified: bool) {
    env.events()
        .publish((Symbol::new(env, "MENTORVERIFIED"), mentor), verified);
}
