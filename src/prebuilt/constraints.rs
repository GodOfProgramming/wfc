use crate::{Constraint, Socket};
use std::{
  collections::{BTreeSet, HashSet},
  fmt::Debug,
};

/// Tests socket connections by directly checking if the socket is in the set
#[derive(Default, Debug, Clone, Copy)]
pub struct UnaryConstraint;

impl<S: Socket> Constraint<S> for UnaryConstraint {
  #[profiling::function]
  fn check(&self, socket: &S, sockets: &HashSet<S>) -> bool {
    sockets.contains(socket)
  }
}

/// Accounts for sockets that have multiple inner values
/// If the socket set intersects with any set of connecting sockets, the test passes
#[derive(Default, Debug, Clone, Copy)]
pub struct SetConstraint;

impl<S: Socket> Constraint<BTreeSet<S>> for SetConstraint {
  #[profiling::function]
  fn check(&self, socket: &BTreeSet<S>, all_connecting_sockets: &HashSet<BTreeSet<S>>) -> bool {
    all_connecting_sockets
      .iter()
      .any(|connecting_sockets| !connecting_sockets.is_disjoint(socket))
  }
}
