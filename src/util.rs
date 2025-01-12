use crate::err::ConversionError;
use derive_more::derive::{Deref, DerefMut};
use nalgebra::SVector;
use std::{
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

  pub fn from_index(mut index: usize, size: Size<DIM>) -> Self {
    let mut parts: [usize; DIM] = std::array::from_fn(|i| {
      let rev_i = DIM - 1 - i;
      let product_of_parts = (rev_i > 0)
        .then(|| (0..rev_i).map(|ri| size[ri]).product::<usize>())
        .unwrap_or(1);
      let entry = index / product_of_parts;
      index -= entry * product_of_parts;
      entry
    });

    parts.reverse();

    Self(SVector::from(parts.map(|i| i as isize)))
  }

  pub fn index(&self, size: Size<DIM>) -> usize {
    self
      .iter()
      .enumerate()
      .map(|(i, p)| {
        (p * (i > 0)
          .then(|| (0..i).map(|i| size[i]).product::<usize>() as isize)
          .unwrap_or(1)) as usize
      })
      .sum()
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

  pub fn from_index(mut index: usize, size: Size<DIM>) -> Self {
    let mut parts: [usize; DIM] = std::array::from_fn(|i| {
      let rev_i = DIM - 1 - i;
      let product_of_parts = (rev_i > 0)
        .then(|| (0..rev_i).map(|ri| size[ri]).product::<usize>())
        .unwrap_or(1);
      let entry = index / product_of_parts;
      index -= entry * product_of_parts;
      entry
    });

    parts.reverse();

    Self(SVector::from(parts))
  }

  pub fn index(&self, size: Size<DIM>) -> usize {
    self
      .iter()
      .enumerate()
      .map(|(i, p)| {
        p * (i > 0)
          .then(|| (0..i).map(|i| size[i]).product::<usize>())
          .unwrap_or(1)
      })
      .sum()
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

impl<D, const DIM: usize> Add<D> for IPos<DIM>
where
  D: IntoEnumIterator + PartialEq<D>,
{
  type Output = Self;

  fn add(mut self, rhs: D) -> Self::Output {
    let index = D::iter().position(|d| d == rhs).unwrap();
    let even = index & 1 == 0;
    let offset = if even { -1 } else { 1 };

    let arr_index = index / 2;

    self[arr_index] += offset;

    self
  }
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
