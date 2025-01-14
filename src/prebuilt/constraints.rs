use crate::{Constraint, Socket};
use std::{
  collections::{BTreeSet, HashSet},
  fmt::Debug,
  marker::PhantomData,
};

#[derive(Default, Debug, Clone, Copy)]
pub struct UnaryConstraint<S: Socket>(PhantomData<S>);

impl<S: Socket> Constraint for UnaryConstraint<S> {
  type Socket = S;

  #[profiling::function]
  fn check(&self, socket: &Self::Socket, sockets: &HashSet<Self::Socket>) -> bool {
    sockets.contains(socket)
  }
}

#[derive(Debug)]
pub struct SetConstraint<S: Socket>(PhantomData<S>);

impl<S: Socket> Default for SetConstraint<S> {
  fn default() -> Self {
    Self(PhantomData)
  }
}

impl<S: Socket> Clone for SetConstraint<S> {
  fn clone(&self) -> Self {
    Self(PhantomData)
  }
}

impl<S: Socket> Constraint for SetConstraint<S> {
  type Socket = BTreeSet<S>;

  #[profiling::function]
  fn check(&self, socket: &Self::Socket, all_connecting_sockets: &HashSet<Self::Socket>) -> bool {
    all_connecting_sockets
      .iter()
      .any(|connecting_sockets| !connecting_sockets.is_disjoint(socket))
  }
}
