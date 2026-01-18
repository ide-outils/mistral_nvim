#![allow(dead_code)]
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Bound, Deref, DerefMut, Range, RangeBounds, Sub},
};

use nvim_oxi::api;

use crate::notify::NotifyExt as _;

#[derive(Clone, Copy, Eq, PartialEq, Debug, PartialOrd, Ord, Default)]
pub struct Row(usize);
#[derive(Clone, Copy, Eq, PartialEq, Debug, PartialOrd, Ord, Default)]
pub struct Col(usize);

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Row1Indexed(usize);
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Col1Indexed(usize);

// --- RowRange ---
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RowRange {
    pub start: Row,
    pub end: Row,
}
impl From<Range<usize>> for RowRange {
    fn from(value: Range<usize>) -> RowRange {
        RowRange {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}
impl From<usize> for RowRange {
    fn from(value: usize) -> RowRange {
        let start = Row(value);
        RowRange { start, end: start }
    }
}
impl RowRange {
    pub fn exclusive(self) -> RowRangeExclusive {
        self.into()
    }
    pub fn inclusive(self) -> RowRangeInclusive {
        self.into()
    }
    pub fn contains<U>(&self, item: &U) -> bool
    where
        usize: PartialOrd<U>,
        U: ?Sized + PartialOrd<usize>,
    {
        &*self.start <= item && item <= &*self.end
    }
    pub fn len(&self) -> usize {
        (*self.end as isize)
            .saturating_sub_unsigned(*self.start)
            .abs() as usize
    }
}

// --- ColRange ---
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ColRange {
    pub start: Col,
    pub end: Col,
}
// impl RangeBounds<usize> for ColRange {
//     fn start_bound(&self) -> Bound<&usize> {
//         Bound::Included(&*self.start)
//     }

//     fn end_bound(&self) -> std::ops::Bound<&usize> {
//         Bound::Excluded(&*self.end)
//     }

//     fn contains<U>(&self, item: &U) -> bool
//     where
//         usize: PartialOrd<U>,
//         U: ?Sized + PartialOrd<usize>,
//     {
//         &*self.start <= item && item <= &*self.end
//     }
// }
impl From<Range<usize>> for ColRange {
    fn from(value: Range<usize>) -> ColRange {
        ColRange {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}
impl From<usize> for ColRange {
    fn from(value: usize) -> ColRange {
        let start = Col(value);
        ColRange { start, end: start }
    }
}
impl ColRange {
    pub fn exclusive(self) -> ColRangeExclusive {
        self.into()
    }
    pub fn inclusive(self) -> ColRangeInclusive {
        self.into()
    }
    pub fn contains<U>(&self, item: &U) -> bool
    where
        usize: PartialOrd<U>,
        U: ?Sized + PartialOrd<usize>,
    {
        &*self.start <= item && item <= &*self.end
    }
    pub fn len(&self) -> usize {
        (*self.end as isize)
            .saturating_sub_unsigned(*self.start)
            .abs() as usize
    }
}

// --- Row ---
impl Into<usize> for Row {
    fn into(self) -> usize {
        self.0
    }
}
impl Into<isize> for Row {
    fn into(self) -> isize {
        self.0 as isize
    }
}
impl From<usize> for Row {
    fn from(value: usize) -> Row {
        Row(value)
    }
}
impl Deref for Row {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Row {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Add<usize> for Row {
    type Output = Row;

    fn add(mut self, rhs: usize) -> Self::Output {
        *self += rhs;
        self
    }
}
impl AddAssign<usize> for Row {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs
    }
}
impl Sub<usize> for Row {
    type Output = Row;

    fn sub(mut self, rhs: usize) -> Self::Output {
        *self -= rhs;
        self
    }
}
impl Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Column ---
impl Into<usize> for Col {
    fn into(self) -> usize {
        self.0
    }
}
impl From<usize> for Col {
    fn from(value: usize) -> Col {
        Col(value)
    }
}
impl Deref for Col {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Col {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Add<usize> for Col {
    type Output = Col;

    fn add(mut self, rhs: usize) -> Self::Output {
        *self += rhs;
        self
    }
}
impl AddAssign<usize> for Col {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs
    }
}
impl Sub<usize> for Col {
    type Output = Self;

    fn sub(mut self, rhs: usize) -> Self::Output {
        *self -= rhs;
        self
    }
}
impl Display for Col {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- RowIndexed ---
impl Into<Row> for Row1Indexed {
    fn into(self) -> Row {
        Row(self.0.saturating_sub(1))
    }
}
impl From<Row> for Row1Indexed {
    fn from(value: Row) -> Self {
        Self(value.saturating_add(1))
    }
}

// --- ColumnIndexed ---
impl Into<Col> for Col1Indexed {
    fn into(self) -> Col {
        Col(self.0.saturating_sub(1))
    }
}
impl From<Col> for Col1Indexed {
    fn from(value: Col) -> Self {
        Self(value.saturating_add(1))
    }
}

// --- RowRangeInclusive ---
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RowRangeInclusive {
    pub start: Row,
    pub end: Row,
}
impl RangeBounds<usize> for RowRangeInclusive {
    fn start_bound(&self) -> Bound<&usize> {
        Bound::Included(&*self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        Bound::Included(&*self.end)
    }
}
impl From<RowRange> for RowRangeInclusive {
    fn from(value: RowRange) -> Self {
        let RowRange { start, end } = value;
        Self { start, end }
    }
}
// --- ColRangeInclusive ---
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ColRangeInclusive {
    pub start: Col,
    pub end: Col,
}
impl RangeBounds<usize> for ColRangeInclusive {
    fn start_bound(&self) -> Bound<&usize> {
        Bound::Included(&*self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        Bound::Included(&*self.end)
    }
}
impl From<ColRange> for ColRangeInclusive {
    fn from(value: ColRange) -> Self {
        let ColRange { start, end } = value;
        Self { start, end }
    }
}

// --- RowRangeInclusive ---
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RowRangeExclusive {
    pub start: Row,
    pub end: Row,
}
impl RangeBounds<usize> for RowRangeExclusive {
    fn start_bound(&self) -> Bound<&usize> {
        Bound::Included(&*self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        Bound::Excluded(&*self.end)
    }
}
impl From<RowRange> for RowRangeExclusive {
    fn from(value: RowRange) -> Self {
        let RowRange { start, end } = value;
        Self { start, end }
    }
}

// --- ColRangeExclusive ---
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ColRangeExclusive {
    pub start: Col,
    pub end: Col,
}
impl RangeBounds<usize> for ColRangeExclusive {
    fn start_bound(&self) -> Bound<&usize> {
        Bound::Included(&*self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        Bound::Excluded(&*self.end)
    }
}
impl From<ColRange> for ColRangeExclusive {
    fn from(value: ColRange) -> Self {
        let ColRange { start, end } = value;
        Self { start, end }
    }
}

// --- Methods ---
pub fn get_cursor(win: &api::Window) -> Option<(Row, Col)> {
    get_cursor_1_indexed(win).map(|(row, col)| (row.into(), col))
}
pub fn get_cursor_1_indexed(win: &api::Window) -> Option<(Row1Indexed, Col)> {
    // get_cursor is Row 1-indexed
    match win.get_cursor() {
        Ok((row, col)) => Some((Row1Indexed(row), Col(col))),
        err => {
            err.notify_error();
            None
        }
    }
}
pub fn set_cursor(win: &mut api::Window, row: impl Into<Row1Indexed>, col: Col) -> Option<()> {
    // set_cursor is Row 1-indexed
    let row = row.into();
    match win.set_cursor(row.0, *col) {
        Ok(()) => Some(()),
        err => {
            err.notify_error();
            None
        }
    }
}

pub struct RangeIterExclusive<T>
where
    T: Copy + PartialOrd + std::ops::AddAssign<usize>,
{
    current: T,
    end: T,
    step: T,
}
impl<T> Iterator for RangeIterExclusive<T>
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

impl IntoIterator for RowRange {
    type Item = Row;

    type IntoIter = RangeIterExclusive<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            current: self.start,
            end: self.end,
            step: self.start,
        }
    }
}

impl IntoIterator for ColRange {
    type Item = Col;

    type IntoIter = RangeIterExclusive<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            current: self.start,
            end: self.end,
            step: self.start,
        }
    }
}

// pub struct RangeIterInclusive<T>
// where
//     T: Copy + PartialOrd + std::ops::AddAssign<usize>,
// {
//     current: T,
//     end: T,
//     step: T,
// }
// impl<T> Iterator for RangeIterInclusive<T>
// where
//     T: Copy + PartialOrd + std::ops::AddAssign<usize>,
// {
//     type Item = T;

//     fn next(&mut self) -> Option<Self::Item> {
//         // Bounds are inclusive but we want to iterate over the item as it will happen nvim side.
//         if self.current < self.end {
//             let result = Some(self.current);
//             self.current += 1;
//             result
//         } else {
//             None
//         }
//     }
// }

// impl IntoIterator for RowRangeInclusive {
//     type Item = Row;

//     type IntoIter = RangeIterInclusive<Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         Self::IntoIter {
//             current: self.0.start,
//             end: self.0.end,
//             step: self.0.start,
//         }
//     }
// }

// impl IntoIterator for ColRangeInclusive {
//     type Item = Col;

//     type IntoIter = RangeIterInclusive<Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         Self::IntoIter {
//             current: self.0.start,
//             end: self.0.end,
//             step: self.0.start,
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    // #[test]
    // fn row_range_length() {
    //     let r1 = 1..1;
    //     let r2: RowRange = 1.into();
    //     let r3: RowRangeInclusive = r2.clone().into();
    //     assert_eq!(r1.start, *r2.start);
    //     assert_eq!(r1.start, *r3.0.start);

    //     assert_eq!(r1.end, *r2.end);
    //     assert_eq!(r1.end, *r3.0.end - 1);
    // }
    #[test]
    fn row_contains() {
        let row: RowRange = (1..2).into();
        let row_range: RowRange = (1..2).into();
        assert!(row_range.contains(&1));
        assert!(row_range.contains(&Row(1)));
        assert!(row_range.contains(&Row(2)));
        assert!(!row_range.contains(&Row(3)));
        let row_range: RowRangeInclusive = row_range.into();
        assert!(row_range.contains(&Row(1)));
        assert!(row_range.contains(&Row(2)));
        assert!(!row_range.contains(&Row(3)));
        let row_range: RowRangeExclusive = row.into();
        assert!(row_range.contains(&Row(1)));
        assert!(!row_range.contains(&Row(2)));
    }
    #[test]
    fn row_start() {
        let row_range: RowRange = 1.into();
        assert_eq!(row_range.start, Row(1));
        let row_range: RowRangeInclusive = row_range.into();
        assert_eq!(row_range.start, Row(1));
    }
    #[test]
    fn row_end() {
        let row_range: RowRange = 1.into();
        assert_eq!(row_range.end, Row(1));
        let row_range: RowRangeInclusive = row_range.into();
        assert_eq!(row_range.end, Row(1));
    }
}
