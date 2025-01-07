use crate::Constraint;
use std::{
  collections::{BTreeSet, HashSet},
  fmt::Debug,
  hash::Hash,
  marker::PhantomData,
};

pub type DefaultConstrainer = UnaryConstrainer;

#[derive(Default, Debug, Clone, Copy)]
pub struct UnaryConstrainer;

impl<S> Constraint<S> for UnaryConstrainer
where
  S: Eq + Hash,
{
  #[profiling::function]
  fn check(&self, socket: &S, sockets: &HashSet<S>) -> bool {
    sockets.contains(socket)
  }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct SetConstrainer;

impl<S> Constraint<BTreeSet<S>> for SetConstrainer
where
  S: Ord,
{
  #[profiling::function]
  fn check(&self, possible_socket: &BTreeSet<S>, sockets: &HashSet<BTreeSet<S>>) -> bool {
    sockets
      .iter()
      .any(|sockets| sockets.intersection(possible_socket).next().is_some())
  }
}

#[derive(Debug)]
pub struct SequentialConstrainer<C1, C2, S>
where
  C1: Constraint<S>,
  C2: Constraint<S>,
  S: Debug,
{
  constrainer1: C1,
  constrainer2: C2,
  _pd: PhantomData<S>,
}

impl<C1, C2, S> Constraint<S> for SequentialConstrainer<C1, C2, S>
where
  C1: Constraint<S>,
  C2: Constraint<S>,
  S: Debug,
{
  #[profiling::function]
  fn check(&self, variant_socket: &S, sockets: &HashSet<S>) -> bool {
    self.constrainer1.check(variant_socket, sockets)
      && self.constrainer2.check(variant_socket, sockets)
  }
}
