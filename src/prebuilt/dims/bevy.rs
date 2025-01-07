use crate::Dimension;
use strum_macros::{EnumCount, EnumIter, VariantArray};

/// Bevy specific version of 2d that is to be used where Up is Y+
#[derive(
  PartialEq, Eq, Hash, PartialOrd, Ord, EnumCount, EnumIter, VariantArray, Clone, Copy, Debug,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub enum Dim2d {
  Left,
  Right,
  Down,
  Up,
}

impl Dimension for Dim2d {
  fn opposite(&self) -> Self {
    match self {
      Self::Left => Self::Right,
      Self::Right => Self::Left,
      Self::Down => Self::Up,
      Self::Up => Self::Down,
    }
  }
}

/// Bevy specific version of 3d that is to be used where Up is Y+
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
