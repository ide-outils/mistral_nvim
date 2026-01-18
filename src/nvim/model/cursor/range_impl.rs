#![allow(dead_code)]
use std::ops::{Add, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive, Sub};

use super::{Bounded, Col, FromNvimRange, Indexed, NvimRange, Row};

pub struct RangeIterInclusive<T>
where
    T: Copy + PartialOrd + std::ops::AddAssign<usize>,
{
    current: T,
    end: T,
}
impl<T> Iterator for RangeIterInclusive<T>
where
    T: Copy + PartialOrd + std::ops::AddAssign<usize>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current <= self.end {
            let result = Some(self.current);
            self.current += 1;
            result
        } else {
            None
        }
    }
}
// the trait `nvim_range::FromNvimRange` is not implemented for `std::ops::RangeFrom<Row>`

macro_rules! fake_range_impl {
    ($BoundType:ident => $RangeType:ident) => {
        #[derive(Clone, Eq, PartialEq, Debug, PartialOrd, Ord, Default, Hash)]
        pub struct $RangeType {
            pub start: $BoundType,
            pub end: $BoundType,
        }

        // --- impl all usize helpers ---
        impl From<RangeTo<usize>> for $RangeType {
            fn from(value: RangeTo<usize>) -> $RangeType {
                (0..value.end).into()
            }
        }
        impl From<RangeFrom<usize>> for $RangeType {
            fn from(value: RangeFrom<usize>) -> $RangeType {
                (value.start..=$BoundType::MAX.0).into()
            }
        }
        impl From<RangeFull> for $RangeType {
            fn from(_value: RangeFull) -> $RangeType {
                ($BoundType::MIN.0..=$BoundType::MAX.0).into()
            }
        }

        impl From<std::ops::Range<usize>> for $RangeType {
            fn from(value: std::ops::Range<usize>) -> Self {
                let start = value.start;
                let mut end = value.end;
                if end != $BoundType::MAX.0 {
                    end = end.saturating_sub(1)
                }
                Self {
                    start: start.into(),
                    end: end.into(),
                }
            }
        }
        impl From<RangeToInclusive<usize>> for $RangeType {
            fn from(value: RangeToInclusive<usize>) -> Self {
                (0..=value.end).into()
            }
        }
        impl From<RangeInclusive<usize>> for $RangeType {
            fn from(value: RangeInclusive<usize>) -> Self {
                let (start, end) = value.into_inner();
                Self {
                    start: start.into(),
                    end: end.into(),
                }
            }
        }
        impl From<usize> for $RangeType {
            fn from(value: usize) -> Self {
                let start = $BoundType(value);
                Self { start, end: start }
            }
        }
        impl Sub<usize> for $RangeType {
            type Output = Self;

            fn sub(mut self, rhs: usize) -> Self::Output {
                self.start -= rhs;
                self.end -= rhs;
                self
            }
        }
        impl Add<usize> for $RangeType {
            type Output = Self;

            fn add(mut self, rhs: usize) -> Self::Output {
                self.start += rhs;
                self.end += rhs;
                self
            }
        }

        // --- impl all $BoundType helpers ---
        impl From<RangeTo<$BoundType>> for $RangeType {
            fn from(value: RangeTo<$BoundType>) -> $RangeType {
                ($BoundType::MIN..value.end).into()
            }
        }
        impl From<RangeFrom<$BoundType>> for $RangeType {
            fn from(value: RangeFrom<$BoundType>) -> $RangeType {
                (value.start..=$BoundType::MAX).into()
            }
        }
        impl From<std::ops::Range<$BoundType>> for $RangeType {
            fn from(value: std::ops::Range<$BoundType>) -> Self {
                let start = value.start;
                let mut end = value.end.0;
                if end != $BoundType::MAX.0 {
                    end = end.saturating_sub(1)
                }
                Self {
                    start: start.into(),
                    end: end.into(),
                }
            }
        }
        impl From<RangeToInclusive<$BoundType>> for $RangeType {
            fn from(value: RangeToInclusive<$BoundType>) -> Self {
                ($BoundType::MIN..=value.end).into()
            }
        }
        impl From<RangeInclusive<$BoundType>> for $RangeType {
            fn from(value: RangeInclusive<$BoundType>) -> Self {
                let (start, end) = value.into_inner();
                Self { start, end }
            }
        }

        impl $RangeType {
            pub const FULL: Self = Self {
                start: $BoundType::MIN,
                end: $BoundType::MAX,
            };
            pub fn contains<U>(&self, item: &U) -> bool
            where
                usize: PartialOrd<U>,
                U: PartialOrd<usize>,
            {
                &*self.start <= item && item <= &*self.end
            }
            pub fn len_abs(&self) -> usize {
                (*self.end as isize)
                    .saturating_sub_unsigned(*self.start)
                    .abs() as usize
            }
            pub fn tuple(&self) -> ($BoundType, $BoundType) {
                (self.start, self.end)
            }
        }

        impl<B, I> FromNvimRange<B, I> for $RangeType
        where
            I: Indexed,
            B: Bounded,
        {
            fn from_nvim(range: NvimRange<B, I>) -> Self {
                Self {
                    // start: $BoundType(range.start(Bound::Included(()))),
                    // end: $BoundType(range.end(Bound::Included(()))),
                    start: $BoundType(range.start()),
                    end: $BoundType(range.end()),
                }
            }
            fn into_nvim(self) -> NvimRange<B, I> {
                NvimRange::new(*self.start, *self.end)
            }
        }
        impl<B, I> FromNvimRange<B, I> for RangeFrom<$BoundType>
        where
            I: Indexed,
            B: Bounded,
        {
            fn from_nvim(range: NvimRange<B, I>) -> Self {
                $BoundType(range.start())..
            }
            fn into_nvim(self) -> NvimRange<B, I> {
                NvimRange::new(*self.start, *$BoundType::MAX)
            }
        }
        impl<B, I> FromNvimRange<B, I> for std::ops::Range<$BoundType>
        where
            I: Indexed,
            B: Bounded,
        {
            fn from_nvim(range: NvimRange<B, I>) -> Self {
                $BoundType(range.start())..$BoundType(range.end())
            }
            fn into_nvim(self) -> NvimRange<B, I> {
                NvimRange::new(*self.start, *self.end)
            }
        }
        impl<B, I> FromNvimRange<B, I> for std::ops::RangeInclusive<$BoundType>
        where
            I: Indexed,
            B: Bounded,
        {
            fn from_nvim(range: NvimRange<B, I>) -> Self {
                $BoundType(range.start())..=$BoundType(range.end())
            }
            fn into_nvim(self) -> NvimRange<B, I> {
                let (start, end) = self.into_inner();
                NvimRange::new(*start, *end)
            }
        }
        impl IntoIterator for $RangeType {
            type Item = $BoundType;

            type IntoIter = RangeIterInclusive<Self::Item>;

            fn into_iter(self) -> Self::IntoIter {
                Self::IntoIter {
                    current: self.start,
                    end: self.end,
                }
            }
        }
    };
}

fake_range_impl!(Row => RowRange);
fake_range_impl!(Col => ColRange);
