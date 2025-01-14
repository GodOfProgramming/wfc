pub(crate) mod auto;
pub(crate) mod cells;
pub(crate) mod err;
pub mod ext;
#[cfg(feature = "profiling")]
pub mod perf;
pub mod prebuilt;
pub(crate) mod rules;
pub(crate) mod state;
pub(crate) mod util;

use derive_more::derive::{Deref, DerefMut};
use derive_new::new;
pub use strum;

use cells::Cells;
use rand::distributions::uniform::SampleUniform;
use std::{
  cmp::PartialOrd,
  collections::{BTreeSet, HashSet},
  fmt::Debug,
  hash::Hash,
  iter::Sum,
  ops::{Add, AddAssign, Mul},
};
use strum::{EnumCount, IntoEnumIterator, VariantArray};

pub mod prelude {
  pub use super::{
    auto::{FindResult, NoSocket, RuleFinder, SocketProvider},
    collapse,
    err::Error,
    prebuilt,
    rules::{AbstractRule, AbstractRules, Legend, Rule, RuleBuilder, Rules},
    state::{State, StateBuilder},
    util::{IPos, Size, UPos},
    Observation,
  };
}

pub use prelude::*;

#[profiling::function]
pub fn collapse<A, C, V, D, S, const DIM: usize>(
  state: &mut State<A, C, V, D, S, DIM>,
) -> Result<(), err::Error<DIM>>
where
  A: Arbiter<V>,
  C: Constraint<S>,
  V: Variant,
  D: Dimension,
  S: Socket,
{
  loop {
    if state.collapse()?.complete() {
      break;
    }
  }
  Ok(())
}

pub type CellIndex = usize;

#[derive(Deref, DerefMut, PartialEq, Eq, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct VariantId(usize);

#[derive(Deref, DerefMut, PartialEq, Eq, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct SocketId(usize);

#[derive(new, Deref, DerefMut, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct DimensionId(usize);
impl DimensionId {
  fn opposite(self) -> Self {
    if *self & 1 == 0 {
      Self(*self + 1)
    } else {
      Self(*self - 1)
    }
  }
}

pub trait Variant: Debug + Eq + Hash + Ord + Clone {}

impl<T> Variant for T where T: Debug + Eq + Hash + Ord + Clone {}

pub trait Socket: Debug + Eq + Hash + Ord + Clone {
  fn to_set(sockets: impl Into<BTreeSet<Self>>) -> BTreeSet<Self> {
    sockets.into()
  }
}

impl<T> Socket for T where T: Debug + Eq + Hash + Ord + Clone {}

pub trait Dimension:
  PartialEq<Self>
  + Eq
  + Hash
  + Ord
  + Clone
  + Copy
  + EnumCount
  + IntoEnumIterator
  + Debug
  + VariantArray
{
  fn opposite(&self) -> Self;
}

#[derive(PartialEq, Eq)]
pub enum Observation {
  Incomplete(usize),
  Complete,
}

impl Observation {
  pub fn complete(&self) -> bool {
    *self == Self::Complete
  }

  pub fn last_observation(&self) -> Option<usize> {
    match self {
      Observation::Incomplete(index) => Some(*index),
      Observation::Complete => None,
    }
  }
}

pub trait Arbiter<V: Variant>: Adjuster<V> {
  fn designate<D: Dimension, const DIM: usize>(
    &mut self,
    cells: &mut Cells<V, D, DIM>,
  ) -> Result<Option<usize>, err::Error<DIM>>;
}

pub trait Adjuster<V: Variant> {
  type Chained<C: Adjuster<V>>: Adjuster<V>;

  /// Perform any mutations to the Cells upon a variant being selected
  fn revise<D: Dimension, const DIM: usize>(&mut self, variant: &V, cells: &mut Cells<V, D, DIM>);

  fn chain<A>(self, other: A) -> Self::Chained<A>
  where
    A: Adjuster<V>;
}

pub trait Constraint<S: Socket>: Debug {
  fn check(&self, socket: &S, all_connecting_sockets: &HashSet<S>) -> bool;
}

pub trait Weight:
  SampleUniform
  + Default
  + Clone
  + Copy
  + PartialOrd<Self>
  + for<'a> AddAssign<&'a Self>
  + Add<Self, Output = Self>
  + Mul<Self, Output = Self>
  + Sum<Self>
  + Debug
{
}

impl<T> Weight for T where
  T: SampleUniform
    + Default
    + Clone
    + Copy
    + PartialOrd<Self>
    + for<'a> AddAssign<&'a Self>
    + Add<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Sum<Self>
    + Debug
{
}

pub trait Shape: Debug {
  type Variant: Variant;
  type Weight: Weight;
  fn weight<D: Dimension, const DIM: usize>(
    &self,
    variant: &Self::Variant,
    index: CellIndex,
    cells: &Cells<Self::Variant, D, DIM>,
  ) -> Self::Weight;
}

#[cfg(test)]
mod tests {
  use crate::{prelude::*, rules::RuleBuilder};
  use maplit::hashmap;
  use prebuilt::{
    arbiters::WeightArbiter, constraints::UnaryConstraint, shapes::WeightedShape, Dim2d,
  };

  const SEED: u64 = 123;

  #[derive(Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
  #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
  #[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
  enum Tiles {
    #[default]
    Empty,
    TileA,
    TileB,
    TileC,
  }

  #[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
  #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
  #[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
  enum Sockets {
    Any,
  }

  #[test]
  fn same_seed_produces_same_gen() {
    let rules = RuleBuilder::default()
      .with_rule(
        Tiles::TileA,
        hashmap! {
          Dim2d::Up    => Some(Sockets::Any),
          Dim2d::Down  =>Some(Sockets::Any),
          Dim2d::Left  =>Some(Sockets::Any),
          Dim2d::Right =>Some(Sockets::Any),
        },
      )
      .with_rule(
        Tiles::TileB,
        hashmap! {
         Dim2d::Up    => Some(Sockets::Any),
         Dim2d::Down  => Some(Sockets::Any),
         Dim2d::Left  => Some(Sockets::Any),
         Dim2d::Right => Some(Sockets::Any),
        },
      )
      .with_rule(
        Tiles::TileC,
        hashmap! {
         Dim2d::Up    => Some(Sockets::Any),
         Dim2d::Down  => Some(Sockets::Any),
         Dim2d::Left  => Some(Sockets::Any),
         Dim2d::Right => Some(Sockets::Any),
        },
      )
      .into();

    let weights = hashmap! {
      Tiles::TileA => 3,
      Tiles::TileB => 2,
    };

    let a_builder = StateBuilder::new(
      [5, 5],
      WeightArbiter::new(Some(SEED), WeightedShape::new(weights.clone())),
      UnaryConstraint::default(),
      rules.clone(),
    );

    let mut a = a_builder.build().unwrap();

    let b_builder = StateBuilder::new(
      [5, 5],
      WeightArbiter::new(Some(SEED), WeightedShape::new(weights)),
      UnaryConstraint::default(),
      rules,
    );

    let mut b = b_builder.build().unwrap();

    crate::collapse(&mut a).unwrap();
    crate::collapse(&mut b).unwrap();

    let a_data: Vec<_> = a.into();
    let b_data: Vec<_> = b.into();

    assert_eq!(a_data, b_data);
  }
}
