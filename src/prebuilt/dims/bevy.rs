use crate::Dimension;
use strum_macros::{EnumCount, EnumIter, VariantArray};

/// Bevy specific version of 2d that is to be used where Up is Y+
#[derive(
  PartialEq, Eq, Hash, PartialOrd, Ord, EnumCount, EnumIter, VariantArray, Clone, Copy, Debug,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub enum Dim2d {
  XNeg,
  XPos,
  YNeg,
  YPos,
}

impl Dimension for Dim2d {
  fn opposite(&self) -> Self {
    match self {
      Self::XNeg => Self::XPos,
      Self::XPos => Self::XNeg,
      Self::YNeg => Self::YPos,
      Self::YPos => Self::YNeg,
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
  XNeg,
  XPos,
  YNeg,
  YPos,
  ZNeg,
  ZPos,
}

impl Dimension for Dim3d {
  fn opposite(&self) -> Self {
    match self {
      Self::XNeg => Self::XPos,
      Self::XPos => Self::XNeg,
      Self::YNeg => Self::YPos,
      Self::YPos => Self::YNeg,
      Self::ZNeg => Self::ZPos,
      Self::ZPos => Self::ZNeg,
    }
  }
}
