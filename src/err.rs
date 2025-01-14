use crate::util::IPos;
use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
pub enum Error<const DIM: usize> {
  #[error("Contradiction found at {position:?} with {neighbor:?} ")]
  Contradiction {
    position: IPos<DIM>,
    neighbor: IPos<DIM>,
  },
  #[error("No rule available for variant {variant:?}")]
  NoRule { variant: usize },
  #[error("No possibilities available due to setup misconfiguration")]
  NoPossibilities,
  #[error(
    "Mismatch in dimensions, DIM set to {const_value} and Dimension evaluated to {dimension_count}"
  )]
  DimensionMismatch {
    const_value: usize,
    dimension_count: usize,
  },
}

#[derive(Debug, thiserror::Error)]
pub enum ConversionError<const DIM: usize> {
  #[error("Could not convert {0:?} to UPos")]
  IPosToUPos(IPos<DIM>),
}
