use crate::{Dimension, Rules};
use std::{fmt::Debug, hash::Hash, num::TryFromIntError};

pub type FindResult<S> = std::result::Result<Option<S>, NoSocket>;

pub trait RuleFinder<V, D, S> {
  fn find(&self) -> Result<Rules<V, D, Option<S>>, Error<V, D>>
  where
    V: Debug + Eq + Hash + Ord + Clone,
    D: Dimension,
    S: Debug + Eq + Hash + Ord + Clone;
}

pub trait SocketProvider<V, D, S> {
  type WorkingType: Debug + Eq + Hash + Ord + Clone;

  fn find(
    &self,
    current: Option<Self::WorkingType>,
    dir: D,
    source: &V,
    target: &V,
  ) -> FindResult<Self::WorkingType>;

  fn finalize(&self, dir: D, socket: Self::WorkingType) -> S;
}

#[derive(Default, Debug)]
pub struct NoSocket;

#[derive(Debug, thiserror::Error)]
pub enum Error<V, D> {
  #[error("The provided dimensions could not be used")]
  DimensionTooLarge,
  #[error("Failed to find rules for: {0:#?}")]
  RuleNotFound(Vec<(D, V, V)>),
  #[error("Failed to generate socket {0:?} => {1:?} => {2:?}")]
  SocketGenerationFailure(V, D, V),
}

impl<V, D> From<TryFromIntError> for Error<V, D> {
  fn from(_value: TryFromIntError) -> Self {
    Self::DimensionTooLarge
  }
}
