use crate::util::IPos;
use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
pub enum Error<V, D, S, const DIM: usize> {
  #[error(
    "Contradiction found at {position:?} with {neighbor:?} to the {direction:?} with the set of possible neighbor variants {neighbor_variants:?} (sockets: {neighbor_sockets:?})"
  )]
  Contradiction {
    position: IPos<DIM>,
    neighbor: IPos<DIM>,
    direction: D,
    neighbor_variants: Vec<V>,
    neighbor_sockets: Vec<S>,
  },
  #[error("No rule available for variant {0:?}")]
  NoRule(V),
  #[error("No possibilities available due to setup misconfiguration")]
  NoPossibilities,
  #[error("Mismatch in dimensions, Mode DIM set to {0} and Dimension evaluated to {1}")]
  DimensionMismatch(usize, usize),
}

#[derive(Debug, thiserror::Error)]
pub enum ConversionError<const DIM: usize> {
  #[error("Could not convert {0:?} to UPos")]
  IPosToUPos(IPos<DIM>),
}
