use crate::{auto::FindResult, ext, prelude::*, Constraint, Dimension, TypeAtlas};
use derive_new::new;
use prebuilt::arbiters::WeightArbiter;
use std::{
  collections::{BTreeSet, HashSet},
  fmt::Debug,
  hash::Hash,
  marker::PhantomData,
};

#[derive(new, Debug)]
pub struct ChaosMode<V, D, S, const DIM: usize, const INFLUENCE: usize>(
  #[new(default)] PhantomData<(V, D, S)>,
);

impl<V, D, S, const DIM: usize, const INFLUENCE: usize> TypeAtlas<DIM>
  for ChaosMode<V, D, S, DIM, INFLUENCE>
where
  Self: Constraint<V>,
  V: Eq + Hash + Ord + Clone + Debug + ext::MaybeSerde,
  D: Dimension + ext::MaybeSerde,
  S: crate::Shape<Self, DIM>,
{
  type Variant = V;
  type Dimension = D;
  type Socket = V;
  type Constraint = Self;
  type Arbiter = WeightArbiter<S, Self, DIM>;
}

impl<V, D, S, const DIM: usize, const INFLUENCE: usize> SocketProvider<V, D, BTreeSet<V>>
  for ChaosMode<V, D, S, DIM, INFLUENCE>
where
  Self: Constraint<V>,
  V: Eq + Hash + Ord + Clone + Debug + ext::MaybeSerde,
  D: Dimension + ext::MaybeSerde,
  S: crate::Shape<Self, DIM>,
{
  type WorkingType = BTreeSet<V>;

  fn find(
    &self,
    current: Option<Self::WorkingType>,
    _dir: D,
    source: &V,
    target: &V,
  ) -> FindResult<Self::WorkingType> {
    Ok(Some(
      current
        .map(|current| {
          current
            .union(&BTreeSet::from_iter([source, target].into_iter().cloned()))
            .cloned()
            .collect::<BTreeSet<V>>()
        })
        .unwrap_or_else(|| BTreeSet::from_iter([source, target].into_iter().cloned())),
    ))
  }

  fn finalize(&self, _dir: D, socket: Self::WorkingType) -> BTreeSet<V> {
    socket
  }
}

impl<V, D, const DIM: usize, const INFLUENCE: usize> Constraint<Option<BTreeSet<V>>>
  for ChaosMode<V, V, D, DIM, INFLUENCE>
where
  Self: SocketProvider<V, BTreeSet<V>, D>,
  V: Debug + Ord,
  D: Dimension,
{
  fn check(
    &self,
    possible_socket: &Option<BTreeSet<V>>,
    sockets: &HashSet<Option<BTreeSet<V>>>,
  ) -> bool {
    sockets
      .iter()
      .any(|sockets| match (sockets, possible_socket) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(_), None) => false,
        (Some(sockets), Some(possible_socket)) => {
          sockets.intersection(possible_socket).next().is_some()
        }
      })
  }
}
