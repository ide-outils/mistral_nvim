#![allow(dead_code)]
use std::ops::{Bound, RangeBounds};

pub enum Direction {
    UP,
    DOWN,
}
use Direction::*;

pub struct Exclusive;
pub struct StartExclusive;
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
    fn parse(expected: Bound<()>, bound: Bound<&usize>, unbounded: usize, one: isize) -> usize {
        // let one_sided = match dir {
        //     UP => 1,
        //     DOWN => -1,
        // };
        use Bound::*;
        match (expected, bound) {
            (Included(()), Included(value)) => value.clone(),
            (Included(()), Excluded(value)) => value.saturating_add_signed(one),
            (Included(()), Unbounded) => unbounded,

            (Excluded(()), Included(value)) => value.saturating_sub_signed(one),
            (Excluded(()), Excluded(value)) => value.clone(),
            (Excluded(()), Unbounded) => unbounded,

            (Unbounded, Included(_value)) => unbounded,
            (Unbounded, Excluded(_value)) => unbounded,
            (Unbounded, Unbounded) => unbounded,
        }
    }
    fn bound_start(bound: Bound<&usize>) -> usize {
        Self::parse(Self::START, bound, usize::MIN, 1)
    }
    fn unbound_start(expected: Bound<()>, value: &usize) -> usize {
        Self::parse(expected, Self::get_bound(value, Self::START), usize::MIN, 1)
    }
    fn bound_end(bound: Bound<&usize>) -> usize {
        Self::parse(Self::END, bound, usize::MAX, -1)
    }
    fn unbound_end(expected: Bound<()>, value: &usize) -> usize {
        Self::parse(expected, Self::get_bound(value, Self::END), usize::MAX, -1)
    }
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
pub struct ZeroIndexed;
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

pub struct NvimRange<B = Inclusive, I = ZeroIndexed>
where
    I: Indexed,
    B: Bounded,
{
    bound: std::marker::PhantomData<B>,
    index: std::marker::PhantomData<I>,
    start: NvimBound<I>,
    end: NvimBound<I>,
}

pub struct NvimBound<I = ZeroIndexed>
where
    I: Indexed,
{
    index: std::marker::PhantomData<I>,
    value: usize,
}

impl<B, I> NvimRange<B, I>
where
    B: Bounded,
    I: Indexed,
{
    pub fn start(&self, expected: Bound<()>) -> usize {
        B::unbound_start(expected, &self.start.value())
    }
    pub fn end(&self, expected: Bound<()>) -> usize {
        B::unbound_end(expected, &self.end.value())
    }
    pub fn new(start: Bound<&usize>, end: Bound<&usize>) -> Self {
        Self {
            bound: std::marker::PhantomData,
            index: std::marker::PhantomData,
            start: NvimBound::new(B::bound_start(start)),
            end: NvimBound::new(B::bound_end(end)),
        }
    }
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
        Self::new(value.start_bound(), value.end_bound())
    }
}

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

    // #[test]
    // fn bound_usize_into() {
    //     let num: usize = 2;
    //     let bound: NvimBound = num.into_nvim();
    //     assert_eq!(bound.value, 2);
    //     let bound: NvimBound<OneIndexed> = num.into_nvim();
    //     assert_eq!(bound.value, 3);
    //     let num: usize = bound_one_indexed(bound);
    //     assert_eq!(num, 2);
    // }

    // #[test]
    // fn range_usize_into() {
    //     let range = 2..5;
    //     let nvim_range: NvimRange = range.into_nvim();
    //     assert_eq!(nvim_range.start.value, 2);
    //     assert_eq!(nvim_range.end.value, 5);
    //     let range = 2..5;
    //     let nvim_range: NvimRange<Inclusive, OneIndexed> = range.into_nvim();
    //     assert_eq!(nvim_range.start.value, 3);
    //     assert_eq!(nvim_range.end.value, 6);
    //     assert_eq!(nvim_range.range, 2..5);
    //     let num: std::ops::Range<usize> = range_one_indexed(nvim_range);
    //     assert_eq!(num, 2..5);
    //     let nvim_range = into_range_end_exclusive(2..5);
    //     assert_eq!(nvim_range.start.value, 2);
    //     assert_eq!(nvim_range.end.value, 5);
    //     assert!(matches!(nvim_range.end_bound(), Bound::Excluded(5)));
    //     assert_eq!(nvim_range.range, 2..5);
    // }

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
        assert_eq!(nvim_range.end.value, 6);
        assert_eq!(nvim_range.start.value(), 2);
        assert_eq!(nvim_range.end.value(), 6);
        assert!(matches!(nvim_range.start_bound(), Bound::Included(2)));
        assert!(matches!(nvim_range.end_bound(), Bound::Excluded(6)));
        let r: RowRange = RowRange::from_nvim(nvim_range);
        assert_eq!(r.start.0, 2);
        assert_eq!(r.end.0, 5);
    }
}
