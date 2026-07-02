use crate::errors::VouchingError;

pub fn add_u32(a: u32, b: u32) -> Result<u32, VouchingError> {
    a.checked_add(b).ok_or(VouchingError::Overflow)
}

pub fn sub_u32(a: u32, b: u32) -> Result<u32, VouchingError> {
    a.checked_sub(b).ok_or(VouchingError::Underflow)
}
