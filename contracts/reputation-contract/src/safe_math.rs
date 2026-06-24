#![allow(dead_code)]
use crate::errors::ReputationError;

pub fn add_u32(a: u32, b: u32) -> Result<u32, ReputationError> {
    a.checked_add(b).ok_or(ReputationError::Overflow)
}

pub fn sub_u32(a: u32, b: u32) -> Result<u32, ReputationError> {
    a.checked_sub(b).ok_or(ReputationError::Underflow)
}
