#![allow(dead_code)]
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Deref, DerefMut, Sub, SubAssign},
};

use super::{FromNvimBound, Indexed, NvimBound};

#[derive(Clone, Copy, Eq, PartialEq, Debug, PartialOrd, Ord, Default, Hash)]
pub struct Row(pub(crate) usize);

#[derive(Clone, Copy, Eq, PartialEq, Debug, PartialOrd, Ord, Default, Hash)]
pub struct Col(pub(crate) usize);

macro_rules! usize_impl {
    ($Bound:ident,  $max:expr) => {
        impl Into<usize> for $Bound {
            fn into(self) -> usize {
                self.0
            }
        }
        impl Into<isize> for $Bound {
            fn into(self) -> isize {
                self.0 as isize
            }
        }
        impl From<usize> for $Bound {
            fn from(value: usize) -> Self {
                $Bound(value)
            }
        }
        impl Deref for $Bound {
            type Target = usize;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl DerefMut for $Bound {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
        impl Add<usize> for $Bound {
            type Output = Self;

            fn add(mut self, rhs: usize) -> Self::Output {
                self.0 = self.0.saturating_add(rhs);
                self
            }
        }
        impl AddAssign<usize> for $Bound {
            fn add_assign(&mut self, rhs: usize) {
                self.0 = self.0.saturating_add(rhs);
            }
        }
        impl Add<$Bound> for $Bound {
            type Output = Self;

            fn add(mut self, rhs: $Bound) -> Self::Output {
                self.0 = self.0.saturating_add(*rhs);
                self
            }
        }
        impl AddAssign<$Bound> for $Bound {
            fn add_assign(&mut self, rhs: Self) {
                self.0 = self.0.saturating_add(*rhs);
            }
        }
        impl Sub<usize> for $Bound {
            type Output = Self;

            fn sub(mut self, rhs: usize) -> Self::Output {
                self.0 = self.0.saturating_sub(rhs);
                self
            }
        }
        impl SubAssign<usize> for $Bound {
            fn sub_assign(&mut self, rhs: usize) {
                self.0 = self.0.saturating_sub(rhs);
            }
        }
        impl Sub<$Bound> for $Bound {
            type Output = Self;

            fn sub(mut self, rhs: $Bound) -> Self::Output {
                self.0 = self.0.saturating_sub(*rhs);
                self
            }
        }
        impl SubAssign<$Bound> for $Bound {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 = self.0.saturating_sub(*rhs);
            }
        }
        impl Display for $Bound {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl $Bound {
            pub const MIN: Self = Self(0);
            pub const MAX: Self = Self($max);
            pub fn to_usize(&self) -> usize {
                self.0
            }
        }

        impl<I> FromNvimBound<I> for $Bound
        where
            I: Indexed,
        {
            fn from_nvim(value: NvimBound<I>) -> Self {
                Self(value.value())
            }
            fn into_nvim(self) -> NvimBound<I> {
                NvimBound::new(self.0)
            }
        }
    };
}

// usize_impl!(Row, usize::MAX - 1);
usize_impl!(Row, 2usize.pow(31) - 1);
usize_impl!(Col, 2usize.pow(31) - 1);
