use crate::errors::CreditLineError;

pub fn add_i128(a: i128, b: i128) -> Result<i128, CreditLineError> {
    a.checked_add(b).ok_or(CreditLineError::Overflow)
}

pub fn sub_i128(a: i128, b: i128) -> Result<i128, CreditLineError> {
    a.checked_sub(b).ok_or(CreditLineError::Underflow)
}

pub fn mul_i128(a: i128, b: i128) -> Result<i128, CreditLineError> {
    a.checked_mul(b).ok_or(CreditLineError::Overflow)
}

pub fn div_i128(a: i128, b: i128) -> Result<i128, CreditLineError> {
    a.checked_div(b).ok_or(CreditLineError::Overflow)
}

pub fn add_u64(a: u64, b: u64) -> Result<u64, CreditLineError> {
    a.checked_add(b).ok_or(CreditLineError::Overflow)
}

pub fn sub_u64(a: u64, b: u64) -> Result<u64, CreditLineError> {
    a.checked_sub(b).ok_or(CreditLineError::Underflow)
}

pub fn mul_u64(a: u64, b: u64) -> Result<u64, CreditLineError> {
    a.checked_mul(b).ok_or(CreditLineError::Overflow)
}

pub fn div_u64(a: u64, b: u64) -> Result<u64, CreditLineError> {
    a.checked_div(b).ok_or(CreditLineError::Overflow)
}
