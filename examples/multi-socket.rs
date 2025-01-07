use maplit::hashmap;
use prebuilt::{
  arbiters::WeightArbiter, constraints::SetConstrainer, shapes::NoShape, weights::DirectWeight,
};
use std::collections::BTreeSet;
use wfc::{prebuilt::Dim3d, prelude::*, Rule, StateBuilder};

#[derive(Debug)]
struct Bench;

impl TypeAtlas<3> for Bench {
  type Variant = usize;
  type Dimension = Dim3d;
  type Socket = BTreeSet<usize>;
  type Arbiter = WeightArbiter<Self, 3>;
  type Constraint = SetConstrainer;
  type Weight = DirectWeight;
  type Shape = NoShape;
}

fn main() {
  #[cfg(feature = "profiling")]
  let _guards = wfc::perf::enable_profiling();

  let mut builder = StateBuilder::<Bench, 3>::new(
    [50, 50, 50],
    WeightArbiter::new(None, NoShape),
    SetConstrainer,
  );

  builder.with_rules(hashmap! {
    0 => Rule::from_fn(|_| BTreeSet::from_iter([0, 1])),
    1 => Rule::from_fn(|_| BTreeSet::from_iter([0, 1, 2])),
    2 => Rule::from_fn(|_| BTreeSet::from_iter([1, 2, 3])),
    3 => Rule::from_fn(|_| BTreeSet::from_iter([2, 3])),
  });

  let mut state = builder.build().expect("Failed to build state");

  wfc::collapse(&mut state).expect("Failed to collapse");
}