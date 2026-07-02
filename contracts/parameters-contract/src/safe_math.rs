use crate::errors::ParametersError;

pub fn add_i128(a: i128, b: i128) -> Result<i128, ParametersError> {
    a.checked_add(b).ok_or(ParametersError::Overflow)
}

pub fn sub_i128(a: i128, b: i128) -> Result<i128, ParametersError> {
    a.checked_sub(b).ok_or(ParametersError::Underflow)
}

pub fn mul_i128(a: i128, b: i128) -> Result<i128, ParametersError> {
    a.checked_mul(b).ok_or(ParametersError::Overflow)
}

pub fn div_i128(a: i128, b: i128) -> Result<i128, ParametersError> {
    a.checked_div(b).ok_or(ParametersError::Overflow)
}
