#![allow(dead_code)]
use crate::errors::LiquidityPoolError;

pub fn add_i128(a: i128, b: i128) -> Result<i128, LiquidityPoolError> {
    a.checked_add(b).ok_or(LiquidityPoolError::Overflow)
}

pub fn sub_i128(a: i128, b: i128) -> Result<i128, LiquidityPoolError> {
    a.checked_sub(b).ok_or(LiquidityPoolError::Underflow)
}

pub fn mul_i128(a: i128, b: i128) -> Result<i128, LiquidityPoolError> {
    a.checked_mul(b).ok_or(LiquidityPoolError::Overflow)
}

pub fn div_i128(a: i128, b: i128) -> Result<i128, LiquidityPoolError> {
    a.checked_div(b).ok_or(LiquidityPoolError::Overflow)
}
