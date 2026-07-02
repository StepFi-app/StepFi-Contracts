use crate::errors::Error;

pub fn add_u64(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::Overflow)
}

pub fn sub_u64(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_sub(b).ok_or(Error::Underflow)
}
