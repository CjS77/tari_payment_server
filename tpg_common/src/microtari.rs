use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, Mul, Neg, Sub, SubAssign},
};

use serde::{Deserialize, Serialize};
use sqlx::Type;
use thiserror::Error;

use crate::op;

pub const TARI_CURRENCY_CODE: &str = "XTR";
pub const TARI_CURRENCY_CODE_LOWER: &str = "xtr";

//--------------------------------------     MicroTari       ---------------------------------------------------------
#[derive(Debug, Clone, Copy, Default, Type, Ord, PartialOrd, Serialize, Deserialize)]
#[sqlx(transparent)]
pub struct MicroTari(i64);

op!(binary MicroTari, Add, add);
op!(binary MicroTari, Sub, sub);
op!(inplace MicroTari, SubAssign, sub_assign);
op!(unary MicroTari, Neg, neg);

impl Mul<i64> for MicroTari {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self::Output {
        Self::from(self.value() * rhs)
    }
}

impl Sum for MicroTari {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), Add::add)
    }
}

#[derive(Debug, Clone, Error)]
#[error("Value cannot be represented in microTari: {0}")]
pub struct MicroTariConversionError(String);

impl From<i64> for MicroTari {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl PartialEq for MicroTari {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for MicroTari {}

impl TryFrom<u64> for MicroTari {
    type Error = MicroTariConversionError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > i64::MAX as u64 {
            Err(MicroTariConversionError(format!("Value {} is too large to convert to MicroTari", value)))
        } else {
            #[allow(clippy::cast_possible_wrap)]
            Ok(Self(value as i64))
        }
    }
}

impl Display for MicroTari {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 10_000 {
            write!(f, "{}μτ", self.0)
        } else {
            let tari = self.0 as f64 / 1_000_000.0;
            write!(f, "{tari:0.3}τ")
        }
    }
}

impl MicroTari {
    pub fn value(&self) -> i64 {
        self.0
    }

    pub fn from_tari(tari: i64) -> Self {
        Self(tari * 1_000_000)
    }
}
