use self::auto::Error;
use crate::{
  auto,
  util::{IPos, Size},
  Dimension, FindResult, Rule, RuleFinder, Rules, SocketProvider,
};
use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData};

pub struct GenericFinder<V, D, S, P, const DIM: usize>
where
  P: SocketProvider<V, D, S>,
{
  source: Vec<V>,
  size: Size<DIM>,
  provider: P,
  _pd: PhantomData<(S, D)>,
}

impl<V, D, S, P, const DIM: usize> GenericFinder<V, D, S, P, DIM>
where
  D: Dimension,
  P: SocketProvider<V, D, S>,
{
  pub fn new(provider: P, source: impl Into<Vec<V>>, size: impl Into<Size<DIM>>) -> Self {
    Self {
      source: source.into(),
      size: size.into(),
      provider,
      _pd: PhantomData,
    }
  }

  #[profiling::function]
  fn get_socket(
    &self,
    current: Option<P::WorkingType>,
    dir: D,
    variant: &V,
    neighbor_variant: &V,
    failures: &mut Vec<(D, V, V)>,
  ) -> FindResult<P::WorkingType>
  where
    V: Clone,
  {
    self
      .provider
      .find(current, dir.clone(), variant, neighbor_variant)
      .map_err(|e| {
        failures.push((dir, variant.clone(), neighbor_variant.clone()));
        e
      })
  }
}

impl<V, D, S, P, const DIM: usize> RuleFinder<V, D, S> for GenericFinder<V, D, S, P, DIM>
where
  V: Debug + Eq + Hash + Ord + Clone,
  D: Dimension,
  S: Debug + Eq + Hash + Ord + Clone,
  P: SocketProvider<V, D, S>,
{
  #[profiling::function]
  fn find(&self) -> Result<Rules<V, D, Option<S>>, Error<V, D>> {
    let mut failures = Vec::new();
    let mut rules = Rules::<V, D, Option<P::WorkingType>>::default();

    for (i, source) in self.source.iter().enumerate() {
      let pos = IPos::from_index(i, self.size);
      let entry = rules.entry(source.clone());
      let rule = entry.or_insert_with(|| Rule::default());

      for dir in D::iter() {
        let neighbor = pos + dir;

        if !self.size.contains(&neighbor) {
          continue;
        }

        let neighbor_index = neighbor.index(self.size);
        let target = &self.source[neighbor_index];

        let current_socket = rule.remove(&dir).unwrap_or(None);
        let new_socket = self
          .get_socket(current_socket, dir, source, target, &mut failures)
          .map_err(|_| Error::SocketGenerationFailure(source.clone(), dir, target.clone()))?;

        rule.insert(dir, new_socket);
      }
    }

    if failures.is_empty() {
      Ok(Rules::new(
        rules
          .into_iter()
          .map(|(i, rule)| {
            (
              i,
              rule
                .into_iter()
                .map(|(dir, socket)| {
                  (
                    dir,
                    socket.map(|socket| self.provider.finalize(dir, socket)),
                  )
                })
                .collect(),
            )
          })
          .collect::<HashMap<V, Rule<D, Option<S>>>>(),
      ))
    } else {
      Err(Error::RuleNotFound(failures))
    }
  }
}