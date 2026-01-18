#![allow(dead_code)]
use std::ops::{Bound, RangeBounds};

pub struct Exclusive;
pub struct StartExclusive;
#[derive(Debug)]
pub struct EndExclusive;
pub struct Inclusive;
pub trait Bounded {
    const START: Bound<()>;
    const END: Bound<()>;
    fn get_bound(value: &usize, bound: Bound<()>) -> Bound<&usize> {
        match bound {
            Bound::Included(()) => Bound::Included(value),
            Bound::Excluded(()) => Bound::Excluded(value),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
    fn unbounded(value: usize, bound: Bound<()>, inc: isize) -> usize {
        match bound {
            Bound::Included(()) => value,
            Bound::Excluded(()) => value.saturating_add_signed(inc),
            Bound::Unbounded => *super::Row::MAX,
        }
    }
    // fn parse(expected: Bound<()>, bound: Bound<&usize>, unbounded: usize, one: isize) -> usize {
    //     use Bound::*;
    //     match (expected, bound) {
    //         (Included(()), Included(value)) => value.clone(),
    //         (Included(()), Excluded(value)) => value.saturating_add_signed(one),
    //         (Included(()), Unbounded) => unbounded,

    //         (Excluded(()), Included(value)) => value.saturating_sub_signed(one),
    //         (Excluded(()), Excluded(value)) => value.clone(),
    //         (Excluded(()), Unbounded) => unbounded,

    //         (Unbounded, Included(_value)) => unbounded,
    //         (Unbounded, Excluded(_value)) => unbounded,
    //         (Unbounded, Unbounded) => unbounded,
    //     }
    // }
    // fn bound_start(bound: Bound<&usize>) -> usize {
    //     Self::parse(Self::START, bound, usize::MIN, 1)
    // }
    // fn unbound_start(expected: Bound<()>, value: &usize) -> usize {
    //     Self::parse(expected, Self::get_bound(value, Self::START), usize::MIN, 1)
    // }
    // fn bound_end(bound: Bound<&usize>) -> usize {
    //     Self::parse(Self::END, bound, usize::MAX, -1)
    // }
    // fn unbound_end(expected: Bound<()>, value: &usize) -> usize {
    //     Self::parse(expected, Self::get_bound(value, Self::END), usize::MAX, -1)
    // }
}
impl Bounded for Exclusive {
    const START: Bound<()> = Bound::Excluded(());
    const END: Bound<()> = Bound::Excluded(());
}
impl Bounded for StartExclusive {
    const START: Bound<()> = Bound::Excluded(());
    const END: Bound<()> = Bound::Included(());
}
impl Bounded for EndExclusive {
    const START: Bound<()> = Bound::Included(());
    const END: Bound<()> = Bound::Excluded(());
}
impl Bounded for Inclusive {
    const START: Bound<()> = Bound::Included(());
    const END: Bound<()> = Bound::Included(());
}
#[derive(Debug)]
pub struct ZeroIndexed;
#[derive(Debug)]
pub struct OneIndexed;
pub trait Indexed {
    const INC: usize = 0;
    fn indexate(value: usize) -> usize {
        value.saturating_add(Self::INC)
    }
    fn unindexate(value: usize) -> usize {
        value.saturating_sub(Self::INC)
    }
}
impl Indexed for ZeroIndexed {}
impl Indexed for OneIndexed {
    const INC: usize = 1;
}

#[derive(Debug)]
pub struct NvimRange<B = Inclusive, I = ZeroIndexed>
where
    I: Indexed,
    B: Bounded,
{
    bound: std::marker::PhantomData<B>,
    index: std::marker::PhantomData<I>,
    pub(super) start: NvimBound<I>,
    pub(super) end: NvimBound<I>,
}

#[derive(Debug)]
pub struct NvimBound<I = ZeroIndexed>
where
    I: Indexed,
{
    index: std::marker::PhantomData<I>,
    pub(super) value: usize,
}

impl<B, I> NvimRange<B, I>
where
    B: Bounded,
    I: Indexed,
{
    pub fn start(&self) -> usize {
        self.start.value()
    }
    pub fn end(&self) -> usize {
        self.end.value()
    }
    pub fn start_unbounded(&self) -> usize {
        B::unbounded(self.start.value(), B::START, 1)
    }
    pub fn end_unbounded(&self) -> usize {
        B::unbounded(self.end.value(), B::END, -1)
    }
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            bound: std::marker::PhantomData,
            index: std::marker::PhantomData,
            start: NvimBound::new(start),
            end: NvimBound::new(end),
        }
    }
    // pub fn start(&self, expected: Bound<()>) -> usize {
    //     B::unbound_start(expected, &self.start.value())
    // }
    // pub fn end(&self, expected: Bound<()>) -> usize {
    //     B::unbound_end(expected, &self.end.value())
    // }
    // pub fn new(start: Bound<&usize>, end: Bound<&usize>) -> Self {
    //     Self {
    //         bound: std::marker::PhantomData,
    //         index: std::marker::PhantomData,
    //         start: NvimBound::new(B::bound_start(start)),
    //         end: NvimBound::new(B::bound_end(end)),
    //     }
    // }
}

impl<I> NvimBound<I>
where
    I: Indexed,
{
    pub fn value(&self) -> usize {
        I::unindexate(self.value)
    }
    pub fn new(value: usize) -> Self {
        Self {
            index: std::marker::PhantomData,
            value: I::indexate(value),
        }
    }
}

impl<B, I> RangeBounds<usize> for NvimRange<B, I>
where
    I: Indexed,
    B: Bounded,
{
    fn start_bound(&self) -> Bound<&usize> {
        B::get_bound(&self.start.value, B::START)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        B::get_bound(&self.end.value, B::END)
    }
}

pub trait FromNvimRange<B = Inclusive, I = ZeroIndexed>
where
    I: Indexed,
    B: Bounded,
{
    fn from_nvim(value: NvimRange<B, I>) -> Self;
    fn into_nvim(self) -> NvimRange<B, I>;
}

pub trait FromNvimBound<I = ZeroIndexed>
where
    I: Indexed,
{
    fn from_nvim(value: NvimBound<I>) -> Self;
    fn into_nvim(self) -> NvimBound<I>;
    fn into_from<T>(self) -> T
    where
        T: FromNvimBound<I>,
        Self: Sized,
    {
        T::from_nvim(self.into_nvim())
    }
}
impl<I> FromNvimBound<I> for NvimBound<I>
where
    I: Indexed,
{
    fn from_nvim(value: NvimBound<I>) -> Self {
        value
    }
    fn into_nvim(self) -> NvimBound<I> {
        self
    }
}

impl<I> From<usize> for NvimBound<I>
where
    I: Indexed,
{
    fn from(value: usize) -> Self {
        Self {
            index: std::marker::PhantomData,
            value,
        }
    }
}
impl<B, I> From<std::ops::Range<usize>> for NvimRange<B, I>
where
    B: Bounded,
    I: Indexed,
{
    fn from(value: std::ops::Range<usize>) -> Self {
        // Self::new(value.start_bound(), value.end_bound())
        // FIXME : 0..0 would not behave correctly.
        Self::new(value.start, value.end.saturating_sub(1))
    }
}

impl<B, I> FromNvimRange<B, I> for std::ops::RangeFull
where
    I: Indexed,
    B: Bounded,
{
    fn from_nvim(_range: NvimRange<B, I>) -> Self {
        ..
    }
    fn into_nvim(self) -> NvimRange<B, I> {
        NvimRange::new(*super::Row::MIN, *super::Row::MAX)
    }
}
impl<B, I> FromNvimRange<B, I> for std::ops::RangeFrom<usize>
where
    I: Indexed,
    B: Bounded,
{
    fn from_nvim(range: NvimRange<B, I>) -> Self {
        (range.start())..
    }
    fn into_nvim(self) -> NvimRange<B, I> {
        NvimRange::new(self.start, *super::Row::MAX)
    }
}
impl<B, I> FromNvimRange<B, I> for std::ops::RangeTo<usize>
where
    I: Indexed,
    B: Bounded,
{
    fn from_nvim(range: NvimRange<B, I>) -> Self {
        ..range.end()
    }
    fn into_nvim(self) -> NvimRange<B, I> {
        NvimRange::new(*super::Row::MIN, self.end.saturating_sub(1))
    }
}
impl<B, I> FromNvimRange<B, I> for std::ops::RangeToInclusive<usize>
where
    I: Indexed,
    B: Bounded,
{
    fn from_nvim(range: NvimRange<B, I>) -> Self {
        ..=range.end()
    }
    fn into_nvim(self) -> NvimRange<B, I> {
        NvimRange::new(*super::Row::MIN, self.end)
    }
}
impl<B, I> FromNvimRange<B, I> for std::ops::Range<usize>
where
    I: Indexed,
    B: Bounded,
{
    fn from_nvim(range: NvimRange<B, I>) -> Self {
        range.start()..range.end()
    }
    fn into_nvim(self) -> NvimRange<B, I> {
        NvimRange::new(self.start, self.end.saturating_sub(1))
    }
}
impl<B, I> FromNvimRange<B, I> for std::ops::RangeInclusive<usize>
where
    I: Indexed,
    B: Bounded,
{
    fn from_nvim(range: NvimRange<B, I>) -> Self {
        range.start()..=range.end()
    }
    fn into_nvim(self) -> NvimRange<B, I> {
        let (start, end) = self.into_inner();
        NvimRange::new(start, end)
    }
}
// impl<B, I> FromNvimRange<B, I> for RangeInclusive<usize>
// where
//     I: Indexed,
//     B: Bounded,
// {
//     fn from_nvim(range: NvimRange<B, I>) -> Self {
//         range.start.value..=range.end.value
//         todo!()
//         // let (start, end) = value.into_inner();
//         // Self {
//         //     start: start.into(),
//         //     end: end.into(),
//         // }
//     }
//     fn into_nvim(self) -> NvimRange<B, I> {
//         let (start, end) = self.into_inner();
//         NvimRange::new(start, end)
//         // NvimRange::new(Bound::Included(&self), Bound::Included(&self))
//         // NvimRange::new(self, self)
//     }
// }
// impl From<RangeInclusive<usize>> for $RangeType {
//     fn from(value: RangeInclusive<usize>) -> Self {
//         let (start, end) = value.into_inner();
//         Self {
//             start: start.into(),
//             end: end.into(),
//         }
//     }
// }
impl<B, I> FromNvimRange<B, I> for usize
where
    I: Indexed,
    B: Bounded,
{
    fn from_nvim(range: NvimRange<B, I>) -> Self {
        // range.start(std::ops::Bound::Included(()))
        range.start()
    }
    fn into_nvim(self) -> NvimRange<B, I> {
        // NvimRange::new(Bound::Included(&self), Bound::Included(&self))
        NvimRange::new(self, self)
    }
}
impl<I> FromNvimBound<I> for usize
where
    I: Indexed,
{
    fn from_nvim(range: NvimBound<I>) -> Self {
        range.value()
    }
    fn into_nvim(self) -> NvimBound<I> {
        // NvimBound::new(self)
        NvimBound {
            index: std::marker::PhantomData,
            // This is an entry point from non parsed data,
            // So we don't want shift the value with Indexed::INC
            value: self,
        }
    }
}

impl From<NvimBound<OneIndexed>> for usize {
    fn from(bound: NvimBound<OneIndexed>) -> Self {
        bound.value()
    }
}
impl From<NvimBound<ZeroIndexed>> for usize {
    fn from(bound: NvimBound<ZeroIndexed>) -> Self {
        bound.value()
    }
}

// impl From<NvimBound<OneIndexed>> for NvimBound {
//     fn from(bound: NvimBound<OneIndexed>) -> Self {
//         Self::new(bound.value())
//     }
// }

// impl From<NvimRange<Exclusive>> for NvimRange<Inclusive> {
//     fn from(range: NvimRange<Exclusive>) -> Self {
//         Self::new_inner(range.start(Bound::Included(())), range.end(Bound::Included(())))
//     }
// }

// impl From<NvimRange<Exclusive>> for NvimRange<EndExclusive> {
//     fn from(range: NvimRange<Exclusive>) -> Self {
//         Self::new_inner(range.start(Bound::Included(())), range.end(Bound::Included(())))
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nvim::model::cursor::{Row, RowRange};
    fn bound_one_indexed<B>(bound: NvimBound<OneIndexed>) -> B
    where
        B: FromNvimBound<OneIndexed>,
    {
        B::from_nvim(bound)
    }
    fn range_one_indexed<B, R>(range: NvimRange<B, OneIndexed>) -> R
    where
        B: Bounded,
        R: FromNvimRange<B, OneIndexed>,
    {
        R::from_nvim(range)
    }
    fn into_bound_one_indexed<B>(bound: impl FromNvimBound<OneIndexed>) -> NvimBound<OneIndexed> {
        bound.into_nvim()
    }
    fn into_range_end_exclusive(range: impl FromNvimRange<EndExclusive>) -> NvimRange<EndExclusive> {
        range.into_nvim()
    }

    #[test]
    fn bound_row_into() {
        let num: Row = 2.into();
        let bound: NvimBound = num.into_nvim();
        assert_eq!(bound.value, 2);
        let bound: NvimBound<OneIndexed> = num.into_nvim();
        assert_eq!(bound.value, 3);
        let num: Row = bound_one_indexed(bound);
        assert_eq!(num, 2.into());
    }

    #[test]
    fn bound_one_indexed_usize() {
        fn plop<ROW>() -> ROW
        where
            ROW: FromNvimBound<OneIndexed>,
        {
            FromNvimBound::into_from(1)
        }
        let bound: NvimBound<OneIndexed> = FromNvimBound::into_nvim(1);
        assert_eq!(bound.value, 1);
        assert_eq!(bound.value(), 0);
        let bound: Row = plop();
        assert_eq!(*bound, 0);
    }
    #[test]
    fn bound_zero_indexed_usize() {
        fn plop<ROW>() -> ROW
        where
            ROW: FromNvimBound,
        {
            FromNvimBound::into_from(1)
        }
        let bound: NvimBound = FromNvimBound::into_nvim(1);
        assert_eq!(bound.value, 1);
        assert_eq!(bound.value(), 1);
        let bound: Row = plop();
        assert_eq!(*bound, 1);
    }

    #[test]
    fn range_rowrange_into() {
        let range: RowRange = (2..=5).into();
        let nvim_range: NvimRange = range.into_nvim();
        assert_eq!(nvim_range.start.value, 2);
        assert_eq!(nvim_range.end.value, 5);
        let range: RowRange = (2..=5).into();
        let nvim_range: NvimRange<Inclusive, OneIndexed> = range.into_nvim();
        assert_eq!(nvim_range.start.value, 3);
        assert_eq!(nvim_range.end.value, 6);
        assert_eq!(nvim_range.start.value(), 2);
        assert_eq!(nvim_range.end.value(), 5);
        let nvim_range: NvimRange<Inclusive, OneIndexed> = (2..6).into();
        assert_eq!(nvim_range.start.value, 3);
        assert_eq!(nvim_range.end.value, 6);
        assert_eq!(nvim_range.start.value(), 2);
        assert_eq!(nvim_range.end.value(), 5);
        let r: RowRange = range_one_indexed(nvim_range);
        assert_eq!(r.start.0, 2);
        assert_eq!(r.end.0, 5);
        let nvim_range = into_range_end_exclusive(r);
        assert_eq!(nvim_range.start.value, 2);
        assert_eq!(nvim_range.end.value, 5);
        assert_eq!(nvim_range.start.value(), 2);
        assert_eq!(nvim_range.end.value(), 5);
        assert!(matches!(nvim_range.start_bound(), Bound::Included(2)));
        assert!(matches!(nvim_range.end_bound(), Bound::Excluded(5)));
        let r: RowRange = RowRange::from_nvim(nvim_range);
        assert_eq!(r.start.0, 2);
        assert_eq!(r.end.0, 5);
        let nvim_range: NvimRange<Inclusive> = (2..6).into();
        assert_eq!(nvim_range.start.value, 2);
        assert_eq!(nvim_range.end.value, 5);
        let nvim_range: NvimRange<EndExclusive> = (2..6).into();
        assert_eq!(nvim_range.start.value, 2);
        assert_eq!(nvim_range.end.value, 5);
        let rows_into: RowRange = (Row(0)..=Row(1)).into();
        let rows = RowRange {
            start: Row(0),
            end: Row(1),
        };
        assert_eq!(rows_into, rows);
        let nvim_range: NvimRange<Inclusive> = rows.clone().into_nvim();
        // assert_eq!(nvim_range.start.value, 0);
        // assert_eq!(nvim_range.end.value, 1);
        assert_eq!(nvim_range.start_unbounded(), 0);
        assert_eq!(nvim_range.end_unbounded(), 1);
        let nvim_range: NvimRange<EndExclusive> = rows.into_nvim();
        assert_eq!(nvim_range.start_unbounded(), 0);
        assert_eq!(nvim_range.end_unbounded(), 0);
    }
}
