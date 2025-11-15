use crate::{CellIndex, Dimension, Error, Modifier, Observer, Shape, Variant, cells::Cells, err};
use derive_more::derive::{Deref, DerefMut};
use rand::{
  RngCore, SeedableRng,
  seq::{IndexedRandom, IteratorRandom},
};
use rand_chacha::ChaCha20Rng;
use std::{collections::HashMap, iter::Iterator, marker::PhantomData};

/// Randomly selects from a set of variants for collapsing
#[derive(Debug)]
pub struct RandomObserver {
  seed: u64,
  rng: ChaCha20Rng,
}

impl Default for RandomObserver {
  fn default() -> Self {
    let seed = rand::rng().next_u64();
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Self { seed, rng }
  }
}

impl Clone for RandomObserver {
  fn clone(&self) -> Self {
    Self {
      seed: self.seed,
      rng: self.rng.clone(),
    }
  }
}

impl RandomObserver {
  pub fn new(seed: Option<u64>) -> Self {
    let (rng, seed) = seed
      .map(|seed| (ChaCha20Rng::seed_from_u64(seed), seed))
      .unwrap_or_else(|| {
        let seed = rand::rng().next_u64();
        (ChaCha20Rng::seed_from_u64(seed), seed)
      });

    Self { seed, rng }
  }

  pub fn seed(&self) -> u64 {
    self.seed
  }
}

impl<V: Variant> Observer<V> for RandomObserver {
  #[profiling::function]
  fn observe<D: Dimension, const DIM: usize>(
    &mut self,
    cells: &mut Cells<V, D, DIM>,
  ) -> Result<Option<CellIndex>, err::Error<DIM>> {
    let Some(indexes) = cells.lowest_entropy_indexes() else {
      return Ok(None);
    };

    let Some(index) = indexes.iter().choose(&mut self.rng).cloned() else {
      // should be unreachable
      return Ok(None);
    };

    cells.collapse(index, |_cells, variants| {
      variants
        .iter()
        .choose(&mut self.rng)
        .cloned()
        .ok_or(Error::NoPossibilities)
    })?;

    Ok(Some(index))
  }
}

impl<V: Variant> Modifier<V> for RandomObserver {
  type Chained<C: Modifier<V>> = Chain<V, Self, C>;

  fn modify<D: Dimension, const DIM: usize>(
    &mut self,
    _variant: &V,
    _cells: &mut Cells<V, D, DIM>,
  ) {
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Modifier<V>,
  {
    Chain::new(self, other)
  }
}

/// Applies weights when selecting a variant
#[derive(Debug)]
pub struct WeightedObserver<S: Shape> {
  seed: u64,
  rng: ChaCha20Rng,
  shape: S,
}

impl<S: Shape> Default for WeightedObserver<S>
where
  S: Default,
{
  fn default() -> Self {
    let seed = rand::rng().next_u64();
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Self {
      seed,
      rng,
      shape: S::default(),
    }
  }
}

impl<S: Shape> Clone for WeightedObserver<S>
where
  S: Clone,
{
  fn clone(&self) -> Self {
    Self {
      seed: self.seed,
      rng: self.rng.clone(),
      shape: self.shape.clone(),
    }
  }
}

impl<S: Shape> WeightedObserver<S> {
  pub fn new(seed: Option<u64>, shape: S) -> Self {
    let (rng, seed) = seed
      .map(|seed| (ChaCha20Rng::seed_from_u64(seed), seed))
      .unwrap_or_else(|| {
        let seed = rand::rng().next_u64();
        (ChaCha20Rng::seed_from_u64(seed), seed)
      });

    Self { seed, rng, shape }
  }

  pub fn seed(&self) -> u64 {
    self.seed
  }
}

impl<S: Shape> Observer<S::Variant> for WeightedObserver<S> {
  #[profiling::function]
  fn observe<D: Dimension, const DIM: usize>(
    &mut self,
    cells: &mut Cells<S::Variant, D, DIM>,
  ) -> Result<Option<usize>, err::Error<DIM>> {
    let Some(indexes) = cells.lowest_entropy_indexes() else {
      return Ok(None);
    };

    let Some(index) = indexes.iter().choose(&mut self.rng).cloned() else {
      // should be unreachable
      return Ok(None);
    };

    cells.collapse(index, |cells, variants| {
      variants
        .iter()
        .collect::<Vec<_>>()
        .choose_weighted(&mut self.rng, |variant| {
          self.shape.weight(*variant, index, cells)
        })
        .cloned()
        .cloned()
        .map_err(|_| Error::NoPossibilities)
    })?;

    Ok(Some(index))
  }
}

impl<S: Shape> Modifier<S::Variant> for WeightedObserver<S> {
  type Chained<C: Modifier<S::Variant>> = Chain<S::Variant, Self, C>;

  fn modify<D: Dimension, const DIM: usize>(
    &mut self,
    _variant: &S::Variant,
    _cells: &mut Cells<S::Variant, D, DIM>,
  ) {
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Modifier<S::Variant>,
  {
    Chain::new(self, other)
  }
}

/// Applies limits to variant selection.
/// Only stops the wave function from selecting any more than the amount, does not enforce that number to be reached
#[derive(Debug, Deref, DerefMut)]
pub struct LimitMod<V: Variant>(HashMap<V, usize>);

impl<V: Variant> Clone for LimitMod<V> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<V: Variant> LimitMod<V> {
  pub fn new(limits: impl Into<HashMap<V, usize>>) -> Self {
    Self(limits.into())
  }
}

impl<V: Variant> Modifier<V> for LimitMod<V> {
  type Chained<C: Modifier<V>> = (Self, C);

  #[profiling::function]
  fn modify<D: Dimension, const DIM: usize>(&mut self, variant: &V, cells: &mut Cells<V, D, DIM>) {
    let Some(limit) = self.get_mut(variant) else {
      return;
    };

    *limit = limit.saturating_sub(1);

    if *limit > 0 {
      return;
    }

    for (i, cell) in cells
      .list
      .iter_mut()
      .enumerate()
      .filter(|(_, cell)| !cell.collapsed())
    {
      let starting_entropy = cell.entropy;
      cell.remove_variant(variant);
      cells.entropy_cache.set(starting_entropy, i, cell.entropy);
    }
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Modifier<V>,
  {
    (self, other)
  }
}

/// Allows for chaining an Arbiter to a number of Adjusters for customization
pub struct Chain<V, A, Adj>
where
  V: Variant,
  A: Observer<V>,
  Adj: Modifier<V>,
{
  arbiter: A,
  adjuster: Adj,
  _pd: PhantomData<V>,
}

impl<V, A, Adj> Clone for Chain<V, A, Adj>
where
  V: Variant,
  A: Observer<V> + Clone,
  Adj: Modifier<V> + Clone,
{
  fn clone(&self) -> Self {
    Self {
      arbiter: self.arbiter.clone(),
      adjuster: self.adjuster.clone(),
      _pd: PhantomData,
    }
  }
}

impl<V, A, Adj> Chain<V, A, Adj>
where
  V: Variant,
  A: Observer<V>,
  Adj: Modifier<V>,
{
  pub fn new(arbiter: A, adjuster: Adj) -> Self {
    Self {
      arbiter,
      adjuster,
      _pd: PhantomData,
    }
  }
}

impl<V, A, Adj> Observer<V> for Chain<V, A, Adj>
where
  V: Variant,
  A: Observer<V>,
  Adj: Modifier<V>,
{
  fn observe<D: Dimension, const DIM: usize>(
    &mut self,
    cells: &mut Cells<V, D, DIM>,
  ) -> Result<Option<usize>, err::Error<DIM>> {
    self.arbiter.observe(cells)
  }
}

impl<V, A, Adj> Modifier<V> for Chain<V, A, Adj>
where
  V: Variant,
  A: Observer<V>,
  Adj: Modifier<V>,
{
  type Chained<C: Modifier<V>> = Chain<V, A, (Adj, C)>;

  fn modify<D: Dimension, const DIM: usize>(&mut self, variant: &V, cells: &mut Cells<V, D, DIM>) {
    self.adjuster.modify(variant, cells);
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Modifier<V>,
  {
    Chain::new(self.arbiter, (self.adjuster, other))
  }
}

impl<V, A0, A1> Modifier<V> for (A0, A1)
where
  V: Variant,
  A0: Modifier<V>,
  A1: Modifier<V>,
{
  type Chained<C: Modifier<V>> = ((A0, A1), C);

  fn modify<D: Dimension, const DIM: usize>(&mut self, variant: &V, cells: &mut Cells<V, D, DIM>) {
    self.0.modify(variant, cells);
    self.1.modify(variant, cells);
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Modifier<V>,
  {
    (self, other)
  }
}
