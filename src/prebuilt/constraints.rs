use crate::{Constraint, SocketId};
use std::{collections::HashSet, fmt::Debug};

#[derive(Default, Debug, Clone, Copy)]
pub struct UnaryConstraint;

impl Constraint for UnaryConstraint {
  #[profiling::function]
  fn check(&self, socket: SocketId, sockets: &HashSet<SocketId>) -> bool {
    sockets.contains(&socket)
  }
}
