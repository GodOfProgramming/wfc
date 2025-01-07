use crate::Dimension;
use derive_more::derive::{Deref, DerefMut, From, IntoIterator};
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, PartialEq, Eq, Deref, DerefMut, IntoIterator)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Rules<V, D, S>
where
  V: Debug + Eq + Hash + Ord + Clone,
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  #[deref]
  #[deref_mut]
  #[into_iterator]
  table: HashMap<V, Rule<D, S>>,
}

impl<V, D, S> Default for Rules<V, D, S>
where
  V: Debug + Eq + Hash + Ord + Clone,
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  fn default() -> Self {
    Self {
      table: Default::default(),
    }
  }
}

impl<V, D, S> Clone for Rules<V, D, S>
where
  V: Debug + Eq + Hash + Ord + Clone,
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  fn clone(&self) -> Self {
    Self::new(self.table.clone())
  }
}

impl<'self_lt, V, D, S> Rules<V, D, S>
where
  V: Debug + Eq + Hash + Ord + Clone,
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  pub fn new(rules: impl Into<HashMap<V, Rule<D, S>>>) -> Self {
    let rules: HashMap<V, Rule<D, S>> = rules.into();
    Self {
      table: rules,
      ..Default::default()
    }
  }

  pub fn sockets(&self) -> BTreeSet<S> {
    self
      .values()
      .map(|rule: &Rule<D, S>| rule.sockets.values())
      .flatten()
      .cloned()
      .collect()
  }
}

impl<V, D, S, IntoRule> From<HashMap<V, IntoRule>> for Rules<V, D, S>
where
  V: Debug + Eq + Hash + Ord + Clone,
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
  IntoRule: Into<Rule<D, S>>,
{
  fn from(value: HashMap<V, IntoRule>) -> Self {
    Self::new(
      value
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect::<HashMap<V, Rule<D, S>>>(),
    )
  }
}

#[derive(Debug, PartialEq, Eq, Deref, DerefMut, IntoIterator, From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Rule<D, S>
where
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  sockets: HashMap<D, S>,
}

impl<D, S> Default for Rule<D, S>
where
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  fn default() -> Self {
    Self {
      sockets: Default::default(),
    }
  }
}

impl<D, S> Clone for Rule<D, S>
where
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  fn clone(&self) -> Self {
    Self {
      sockets: self.sockets.clone(),
    }
  }
}

impl<D, S> Rule<D, S>
where
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  pub fn new(sockets: impl Into<HashMap<D, S>>) -> Self {
    Self {
      sockets: sockets.into(),
    }
  }

  pub fn splat<IntoS: Into<S> + Clone>(value: IntoS) -> Self {
    Self::from_fn(|_| value.clone().into())
  }

  pub fn from_fn<IntoS>(mut f: impl FnMut(D) -> IntoS) -> Self
  where
    IntoS: Into<S>,
  {
    let mut map = HashMap::new();
    for d in D::iter() {
      map.insert(d, f(d).into());
    }
    Self { sockets: map }
  }

  pub fn from_default() -> Self
  where
    S: Default,
  {
    Self::from_fn(|_| S::default())
  }
}

impl<D, S> FromIterator<(D, S)> for Rule<D, S>
where
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
{
  fn from_iter<I: IntoIterator<Item = (D, S)>>(iter: I) -> Self {
    Self {
      sockets: HashMap::from_iter(iter),
    }
  }
}

impl<D, S, F, IntoS> From<F> for Rule<D, S>
where
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
  F: FnMut(D) -> IntoS,
  IntoS: Into<S>,
{
  fn from(value: F) -> Self {
    Self::from_fn(value)
  }
}
