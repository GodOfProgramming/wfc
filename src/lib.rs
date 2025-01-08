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

pub use strum;

use cells::Cells;
use rand::distributions::uniform::SampleUniform;
use std::{
  cmp::PartialOrd,
  collections::HashSet,
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
    rules::{Rule, Rules},
    state::{State, StateBuilder},
    util::{IPos, Size, UPos},
    Observation, TypeAtlas,
  };
}

pub use prelude::*;

#[profiling::function]
pub fn collapse<T: TypeAtlas<DIM>, const DIM: usize>(
  state: &mut State<T, DIM>,
) -> TResult<(), T, DIM> {
  loop {
    if state.collapse()?.complete() {
      break;
    }
  }
  Ok(())
}

pub trait TypeAtlas<const DIM: usize>
where
  Self: Debug + Sized,
{
  type Variant: Debug + Eq + Hash + Ord + Clone + ext::MaybeSerde;
  type Socket: Debug + Eq + Hash + Ord + Clone + ext::MaybeSerde;
  type Dimension: Dimension + ext::MaybeSerde;

  type Arbiter: Arbiter<Self, DIM>;
  type Constraint: Constraint<Self::Socket>;

  type Weight: Weight;
  type Shape: Shape<Self, DIM>;
}

pub type TResult<Ok, T, const DIM: usize> = Result<Ok, TError<T, DIM>>;
pub type TError<T, const DIM: usize> = Error<
  <T as TypeAtlas<DIM>>::Variant,
  <T as TypeAtlas<DIM>>::Dimension,
  <T as TypeAtlas<DIM>>::Socket,
  DIM,
>;

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

pub trait Arbiter<T: TypeAtlas<DIM>, const DIM: usize>: Adjuster<T, DIM> {
  fn designate(&mut self, cells: &mut Cells<T, DIM>) -> TResult<Option<usize>, T, DIM>;
}

pub trait Adjuster<T: TypeAtlas<DIM>, const DIM: usize> {
  type Chained<C: Adjuster<T, DIM>>: Adjuster<T, DIM>;

  fn revise(&mut self, variant: &T::Variant, cells: &mut Cells<T, DIM>);

  fn chain<A>(self, other: A) -> Self::Chained<A>
  where
    A: Adjuster<T, DIM>;
}

pub trait Constraint<S>: Debug {
  fn check(&self, variant_socket: &S, sockets: &HashSet<S>) -> bool;
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

pub trait Shape<T: TypeAtlas<DIM>, const DIM: usize>: Debug {
  fn weight(&self, variant: &T::Variant, index: usize, cells: &Cells<T, DIM>) -> T::Weight;
}

#[cfg(test)]
mod tests {
  use crate::{ext::TypeAtlasExt, prelude::*};
  use maplit::hashmap;
  use prebuilt::{
    arbiters::WeightArbiter, constraints::UnaryConstrainer, shapes::WeightedShape, Dim2d,
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

  #[derive(Debug)]
  struct TestMode;

  impl super::TypeAtlas<2> for TestMode {
    type Dimension = Dim2d;
    type Variant = Tiles;
    type Socket = Option<Sockets>;
    type Arbiter = WeightArbiter<Self, 2>;
    type Constraint = UnaryConstrainer;
    type Weight = u8;
    type Shape = WeightedShape<Self, 2>;
  }

  #[test]
  fn same_seed_produces_same_gen() {
    let rules = hashmap! {
      Tiles::TileA => Rule::new(hashmap! {
        Dim2d::Up    => Some(Sockets::Any),
        Dim2d::Down  =>Some(Sockets::Any),
        Dim2d::Left  =>Some(Sockets::Any),
        Dim2d::Right =>Some(Sockets::Any),
      }),
      Tiles::TileB => Rule::new(hashmap!{
        Dim2d::Up    => Some(Sockets::Any),
        Dim2d::Down  => Some(Sockets::Any),
        Dim2d::Left  => Some(Sockets::Any),
        Dim2d::Right => Some(Sockets::Any),
       }),
      Tiles::TileC => Rule::new(hashmap! {
        Dim2d::Up    => Some(Sockets::Any),
        Dim2d::Down  => Some(Sockets::Any),
        Dim2d::Left  => Some(Sockets::Any),
        Dim2d::Right => Some(Sockets::Any),
       }),
    };

    let weights = hashmap! {
      Tiles::TileA => 3,
      Tiles::TileB => 2,
    };

    let mut a_builder = StateBuilder::<TestMode, { TestMode::DIM }>::new(
      [5, 5],
      WeightArbiter::new(Some(SEED), WeightedShape::new(weights.clone())),
      UnaryConstrainer,
    );

    a_builder.with_rules(rules.clone());

    let mut a = a_builder.build().unwrap();

    let mut b_builder = StateBuilder::<TestMode, { TestMode::DIM }>::new(
      [5, 5],
      WeightArbiter::new(Some(SEED), WeightedShape::new(weights)),
      UnaryConstrainer,
    );

    b_builder.with_rules(rules);

    let mut b = b_builder.build().unwrap();

    crate::collapse(&mut a).unwrap();
    crate::collapse(&mut b).unwrap();

    let a_data: Vec<_> = a.into();
    let b_data: Vec<_> = b.into();

    assert_eq!(a_data, b_data);
  }
}
