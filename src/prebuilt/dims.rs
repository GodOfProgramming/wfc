//! Dimensions are interpreted to be in pairs of tow, and go from - to +
//!
//! So the first two entries refer to the x axis, - to +, left to right and so on

pub mod bevy;

use crate::Dimension;
use strum_macros::{EnumCount, EnumIter, VariantArray};

#[derive(
  PartialEq, Eq, Hash, PartialOrd, Ord, EnumCount, EnumIter, VariantArray, Clone, Copy, Debug,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub enum Dim1d {
  Left,
  Right,
}

impl Dimension for Dim1d {
  fn opposite(&self) -> Self {
    match self {
      Self::Left => Self::Right,
      Self::Right => Self::Left,
    }
  }
}

#[derive(
  PartialEq, Eq, Hash, PartialOrd, Ord, EnumCount, EnumIter, VariantArray, Clone, Copy, Debug,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub enum Dim2d {
  Left,
  Right,
  Up,
  Down,
}

impl Dimension for Dim2d {
  fn opposite(&self) -> Self {
    match self {
      Self::Left => Self::Right,
      Self::Right => Self::Left,
      Self::Up => Self::Down,
      Self::Down => Self::Up,
    }
  }
}

#[derive(
  PartialEq, Eq, Hash, PartialOrd, Ord, EnumCount, EnumIter, VariantArray, Clone, Copy, Debug,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub enum Dim3d {
  Left,
  Right,
  Up,
  Down,
  Forward,
  Backward,
}

impl Dimension for Dim3d {
  fn opposite(&self) -> Self {
    match self {
      Self::Left => Self::Right,
      Self::Right => Self::Left,
      Self::Up => Self::Down,
      Self::Down => Self::Up,
      Self::Forward => Self::Backward,
      Self::Backward => Self::Forward,
    }
  }
}
