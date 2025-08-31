use crate::{err::ConversionError, DimensionId};
use derive_more::derive::{Deref, DerefMut};
use nalgebra::SVector;
use std::{
  borrow::Borrow,
  fmt::Debug,
  ops::{Add, Rem},
};
use strum::IntoEnumIterator;

#[macro_export]
macro_rules! here {
  () => {{
    use std::io::Write;
    println!("{}({})", file!(), line!());
    std::io::stdout().flush().ok();
  }};
  ($($arg:tt)+) => {{
    use std::io::Write;
    print!("{}({}): ", file!(), line!());
    println!($($arg)*);
    std::io::stdout().flush().ok();
  }};
}

#[derive(Debug, Clone, Copy, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Size<const DIM: usize>(SVector<usize, DIM>);

impl<const DIM: usize> Default for Size<DIM> {
  fn default() -> Self {
    Self(SVector::zeros())
  }
}

impl<const DIM: usize> Size<DIM> {
  pub fn new(inner: [usize; DIM]) -> Self {
    Self(SVector::from(inner))
  }

  pub fn len(&self) -> usize {
    self.0.iter().product()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn contains(&self, dim: &IPos<DIM>) -> bool {
    dim
      .iter()
      .enumerate()
      .all(|(i, d)| *d >= 0 && *d < self[i] as isize)
  }
}

impl<const DIM: usize> From<UPos<DIM>> for Size<DIM> {
  fn from(value: UPos<DIM>) -> Self {
    Self(value.map(|i| i + 1))
  }
}

impl<const DIM: usize> From<[usize; DIM]> for Size<DIM> {
  fn from(value: [usize; DIM]) -> Self {
    Self::new(value)
  }
}

impl<const DIM: usize> From<[u32; DIM]> for Size<DIM> {
  fn from(value: [u32; DIM]) -> Self {
    Self::new(value.map(|i| i as usize))
  }
}

impl<const DIM: usize> From<[i32; DIM]> for Size<DIM> {
  fn from(value: [i32; DIM]) -> Self {
    Self::new(value.map(|i| i as usize))
  }
}

#[derive(Debug, Clone, Copy, Deref, DerefMut, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct IPos<const DIM: usize>(pub SVector<isize, DIM>);

impl<const DIM: usize> Default for IPos<DIM> {
  fn default() -> Self {
    Self(SVector::zeros())
  }
}

impl<const DIM: usize> IPos<DIM> {
  pub fn new(inner: [isize; DIM]) -> Self {
    Self(SVector::from(inner))
  }

  /// Converts an index into an IPos
  pub fn from_index(index: usize, size: Size<DIM>) -> Self {
    let arr = from_index(index, size);
    Self(SVector::from(arr.map(|i| i as isize)))
  }

  pub fn index(&self, size: Size<DIM>) -> usize {
    to_index(self.iter().map(|i| *i as usize), size)
  }

  pub fn wrap(&self, size: Size<DIM>) -> Self {
    Self(SVector::from_iterator(
      self
        .iter()
        .zip(size.iter().map(|a| *a as isize))
        .map(|(i, s)| wrap(*i, s)),
    ))
  }

  pub fn index_in(&self, size: Size<DIM>) -> usize {
    self.wrap(size).index(size)
  }
}

impl<T, const DIM: usize> From<T> for IPos<DIM>
where
  T: Into<SVector<isize, DIM>>,
{
  fn from(value: T) -> Self {
    Self(value.into())
  }
}

#[derive(Debug, Clone, Copy, Deref, DerefMut, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct UPos<const DIM: usize>(pub SVector<usize, DIM>);

impl<const DIM: usize> Default for UPos<DIM> {
  fn default() -> Self {
    Self(SVector::zeros())
  }
}

impl<const DIM: usize> UPos<DIM> {
  pub fn new(inner: [usize; DIM]) -> Self {
    Self(SVector::from(inner))
  }

  pub fn from_index(index: usize, size: Size<DIM>) -> Self {
    let arr = from_index(index, size);
    Self(SVector::from(arr))
  }

  pub fn index(&self, size: Size<DIM>) -> usize {
    to_index(self.iter(), size)
  }
}

impl<T, const DIM: usize> From<T> for UPos<DIM>
where
  T: Into<SVector<usize, DIM>>,
{
  fn from(value: T) -> Self {
    Self(value.into())
  }
}

impl<const DIM: usize> TryFrom<IPos<DIM>> for UPos<DIM> {
  type Error = ConversionError<DIM>;

  fn try_from(value: IPos<DIM>) -> Result<Self, Self::Error> {
    if value.iter().all(|i| *i >= 0) {
      Ok(UPos(value.map(|i| i as usize)))
    } else {
      Err(ConversionError::IPosToUPos(value))
    }
  }
}

impl<const DIM: usize> Add<DimensionId> for IPos<DIM> {
  type Output = Self;

  fn add(mut self, rhs: DimensionId) -> Self::Output {
    let even = *rhs & 1 == 0;
    let offset = if even { -1 } else { 1 };
    let arr_index = *rhs / 2;
    self[arr_index] += offset;
    self
  }
}

impl<D, const DIM: usize> Add<D> for IPos<DIM>
where
  D: IntoEnumIterator + PartialEq<D>,
{
  type Output = Self;

  /// Adds the direction to the IPos to shift it appropriately
  /// Relies on the dimension being in order from - to + sides
  fn add(mut self, rhs: D) -> Self::Output {
    let index = D::iter().position(|d| d == rhs).unwrap();
    let even = index & 1 == 0;
    let offset = if even { -1 } else { 1 };
    let arr_index = index / 2;
    self[arr_index] += offset;
    self
  }
}

/// Converts an iterator of usize's that should match the length of DIM to a single dimensional index
pub fn to_index<const DIM: usize>(
  iter: impl Iterator<Item = impl Borrow<usize>>,
  size: Size<DIM>,
) -> usize {
  iter
    .enumerate()
    .map(|(i, p)| {
      *p.borrow()
        * if i > 0 {
          (0..i).map(|i| size[i]).product::<usize>()
        } else {
          1
        }
    })
    .sum()
}

/// Converts an index into an position in a dimension specified by DIM
pub fn from_index<const DIM: usize>(mut index: usize, size: Size<DIM>) -> [usize; DIM] {
  // this is the reversed parts of the position, so in 3d it'd be z,y,x
  let mut parts: [usize; DIM] = std::array::from_fn(|i| {
    // this algorithm *must* iterate backwards to work, so take i from above and invert it based on the dimension
    let rev_i = DIM - 1 - i;

    // then compute the product of the dimensions from 0 to the reverse of i
    // in 3d with rev_i == 2 this would be x & y dimensions
    let product_of_parts = if rev_i > 0 {
      (0..rev_i).map(|ri| size[ri]).product::<usize>()
    } else {
      1
    };

    // divide by the above value, which is the amount of times the index fits into the dimension
    let entry = index / product_of_parts;
    // then subtract that away from the index, allowing the next dimension down to test the same
    index -= entry * product_of_parts;

    entry
  });

  // then reverse the output here, zyx -> xyz
  parts.reverse();

  parts
}

/// only to be used when both i and j are different
pub unsafe fn index_twice_mut<T>(slice: &mut [T], i: usize, j: usize) -> [&mut T; 2] {
  debug_assert_ne!(i, j);
  let ptr = slice.as_mut_ptr();
  let ar = &mut *ptr.add(i);
  let br = &mut *ptr.add(j);
  [ar, br]
}

pub fn wrap<T>(i: T, s: T) -> T
where
  T: Clone + Copy + Add<T, Output = T> + Rem<T, Output = T>,
{
  ((i % s) + s) % s
}

#[cfg(test)]
mod tests {
  use super::{IPos, Size, UPos};

  #[test]
  fn ipos_indexes() {
    let size = Size::new([5, 5, 5]);
    let pos = IPos::new([2, 3, 4]);
    let index = pos.index(size);
    assert_eq!(index, 117);
    let orig = IPos::from_index(index, size);
    assert_eq!(pos, orig);
  }

  #[test]
  fn upos_indexes() {
    let size = Size::new([5, 5, 5]);
    let pos = UPos::new([2, 3, 4]);
    let index = pos.index(size);
    assert_eq!(index, 117);
    let orig = UPos::from_index(index, size);
    assert_eq!(pos, orig);
  }

  #[test]
  fn wrapped_ipos() {
    let size = Size::new([4]);
    let pos = IPos::new([-1]);
    let wrapped = pos.wrap(size);

    assert_eq!(wrapped, IPos::new([3]));

    let size = Size::new([10, 10]);
    let pos = IPos::new([-1, -1]);
    let wrapped = pos.wrap(size);

    assert_eq!(wrapped, IPos::new([9, 9]));

    let pos = IPos::new([10, 10]);
    let wrapped = pos.wrap(size);

    assert_eq!(wrapped, IPos::new([0, 0]));
  }
}
