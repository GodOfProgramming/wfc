use prebuilt::{arbiters::RandomArbiter, constraints::UnaryConstraint};
use std::collections::BTreeSet;
use wfc::{prebuilt::Dim3d, prelude::*, StateBuilder};

fn main() {
  #[cfg(feature = "profiling")]
  let _guards = wfc::perf::enable_profiling();

  let rules: Rules<i32, Dim3d, BTreeSet<i32>> = RuleBuilder::default()
    .with_rule(0, |_| BTreeSet::from_iter([0, 1]))
    .with_rule(1, |_| BTreeSet::from_iter([0, 1, 2]))
    .with_rule(2, |_| BTreeSet::from_iter([1, 2, 3]))
    .with_rule(3, |_| BTreeSet::from_iter([2, 3]))
    .into();

  let builder = StateBuilder::new(
    [50, 50, 50],
    RandomArbiter::default(),
    UnaryConstraint,
    rules,
  );

  let mut state = builder.build().expect("Failed to build state");

  wfc::collapse(&mut state).expect("Failed to collapse");
}
