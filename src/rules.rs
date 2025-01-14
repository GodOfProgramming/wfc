use crate::{Dimension, DimensionId, VariantId};
use crate::{Socket, Variant};
use bimap::BiHashMap;
use derive_more::derive::{Deref, DerefMut, From, IntoIterator};
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::Debug;
use std::iter::FromIterator;

#[derive(Deref, DerefMut)]
pub struct RuleBuilder<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
{
  pub table: HashMap<V, Rule<D, S>>,
}

impl<V, D, S> Default for RuleBuilder<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
{
  fn default() -> Self {
    Self {
      table: Default::default(),
    }
  }
}

impl<V, D, S> RuleBuilder<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
{
  pub fn add_rule(&mut self, variant: V, rule: impl Into<Rule<D, S>>) -> &mut Self {
    self.table.insert(variant, rule.into());
    self
  }

  pub fn with_rule(mut self, variant: V, rule: impl Into<Rule<D, S>>) -> Self {
    self.add_rule(variant, rule);
    self
  }

  pub fn abstract_rules(&self) -> (AbstractRules<S>, Legend<V, D>) {
    let legend = Legend::from(&self.table);
    let abstract_rules = self
      .table
      .iter()
      .map(|(variant, rule)| {
        let vi = legend.variant_id(variant);
        (
          vi,
          rule
            .table
            .iter()
            .map(|(dim, socket)| {
              let di = legend.dimension_id(dim);
              (di, socket.clone())
            })
            .collect::<AbstractRule<S>>(),
        )
      })
      .collect();

    (abstract_rules, legend)
  }
}

impl<V, D, S, IntoRule> From<HashMap<V, IntoRule>> for RuleBuilder<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
  IntoRule: Into<Rule<D, S>>,
{
  fn from(value: HashMap<V, IntoRule>) -> Self {
    Self {
      table: value
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect::<HashMap<V, Rule<D, S>>>(),
    }
  }
}

impl<V, D, S> FromIterator<(V, Rule<D, S>)> for RuleBuilder<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
{
  fn from_iter<I: IntoIterator<Item = (V, Rule<D, S>)>>(iter: I) -> Self {
    Self {
      table: HashMap::from_iter(iter),
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Rules<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
{
  table: HashMap<V, Rule<D, S>>,
  abstract_rules: AbstractRules<S>,
  legend: Legend<V, D>,
}

impl<V, D, S> Clone for Rules<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
{
  fn clone(&self) -> Self {
    Self {
      table: self.table.clone(),
      abstract_rules: self.abstract_rules.clone(),
      legend: self.legend.clone(),
    }
  }
}

impl<V, D, S> Rules<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
{
  pub fn variants(&self) -> impl Iterator<Item = &V> {
    self.table.keys()
  }

  pub fn abstract_rules(&self) -> &AbstractRules<S> {
    &self.abstract_rules
  }

  pub fn legend(&self) -> &Legend<V, D> {
    &self.legend
  }
}

impl<V, D, S> From<RuleBuilder<V, D, S>> for Rules<V, D, S>
where
  V: Variant,
  D: Dimension,
  S: Socket,
{
  fn from(builder: RuleBuilder<V, D, S>) -> Self {
    let (abstract_rules, legend) = builder.abstract_rules();
    Self {
      table: builder.table,
      abstract_rules,
      legend,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Deref, DerefMut, IntoIterator, From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Rule<D, S>
where
  D: Dimension,
  S: Socket,
{
  table: HashMap<D, S>,
}

impl<D, S> Default for Rule<D, S>
where
  D: Dimension,
  S: Socket,
{
  fn default() -> Self {
    Self {
      table: Default::default(),
    }
  }
}

impl<D, S> Clone for Rule<D, S>
where
  D: Dimension,
  S: Socket,
{
  fn clone(&self) -> Self {
    Self {
      table: self.table.clone(),
    }
  }
}

impl<D, S> Rule<D, S>
where
  D: Dimension,
  S: Socket,
{
  pub fn new(sockets: impl Into<HashMap<D, S>>) -> Self {
    Self {
      table: sockets.into(),
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
    Self { table: map }
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
  S: Socket,
{
  fn from_iter<I: IntoIterator<Item = (D, S)>>(iter: I) -> Self {
    Self {
      table: HashMap::from_iter(iter),
    }
  }
}

impl<D, S, F, IntoS> From<F> for Rule<D, S>
where
  D: Dimension,
  S: Socket,
  F: FnMut(D) -> IntoS,
  IntoS: Into<S>,
{
  fn from(value: F) -> Self {
    Self::from_fn(value)
  }
}

#[derive(PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Legend<V: Variant, D: Dimension> {
  variants: BiHashMap<VariantId, V>,
  dimensions: BiHashMap<DimensionId, D>,
}

impl<V: Variant, D: Dimension> Clone for Legend<V, D> {
  fn clone(&self) -> Self {
    Self {
      variants: self.variants.clone(),
      dimensions: self.dimensions.clone(),
    }
  }
}

impl<V: Variant, D: Dimension> Legend<V, D> {
  pub fn variant_id(&self, variant: &V) -> VariantId {
    *self.variants.get_by_right(variant).unwrap()
  }

  pub fn variant(&self, variant_id: VariantId) -> V {
    self.variants.get_by_left(&variant_id).cloned().unwrap()
  }

  pub fn dimension_id(&self, dimension: &D) -> DimensionId {
    *self.dimensions.get_by_right(dimension).unwrap()
  }
}

impl<V: Variant, D: Dimension, S: Socket> From<&HashMap<V, Rule<D, S>>> for Legend<V, D> {
  fn from(table: &HashMap<V, Rule<D, S>>) -> Self {
    let variants = table.keys().sorted().cloned().enumerate().collect();
    let dimensions = D::iter()
      .enumerate()
      .map(|(i, d)| (DimensionId::new(i), d))
      .collect();

    Self {
      variants,
      dimensions,
    }
  }
}

#[derive(PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct AbstractRules<S: Socket>(HashMap<usize, AbstractRule<S>>);

impl<S: Socket> Clone for AbstractRules<S> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<S: Socket> AbstractRules<S> {
  pub fn rule_for(&self, variant: VariantId) -> Option<&AbstractRule<S>> {
    self.0.get(&variant)
  }

  pub fn variants(&self) -> impl Iterator<Item = &VariantId> {
    self.0.keys()
  }
}

impl<S: Socket> FromIterator<(VariantId, AbstractRule<S>)> for AbstractRules<S> {
  fn from_iter<T: IntoIterator<Item = (usize, AbstractRule<S>)>>(iter: T) -> Self {
    Self(HashMap::from_iter(iter))
  }
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct AbstractRule<S: Socket>(HashMap<DimensionId, S>);

impl<S: Socket> AbstractRule<S> {
  pub fn socket_for(&self, dir: DimensionId) -> Option<&S> {
    self.0.get(&dir)
  }
}

impl<S: Socket> FromIterator<(DimensionId, S)> for AbstractRule<S> {
  fn from_iter<T: IntoIterator<Item = (DimensionId, S)>>(iter: T) -> Self {
    Self(HashMap::from_iter(iter))
  }
}
