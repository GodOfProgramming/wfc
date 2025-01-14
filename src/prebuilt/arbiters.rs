use crate::{cells::Cells, err, Adjuster, Arbiter, CellIndex, Dimension, Error, Shape, Variant};
use derive_more::derive::{Deref, DerefMut};
use rand::{
  seq::{IteratorRandom, SliceRandom},
  thread_rng, Rng, SeedableRng,
};
use rand_chacha::ChaCha20Rng;
use std::{collections::HashMap, iter::Iterator, marker::PhantomData};

#[derive(Debug)]
pub struct RandomArbiter {
  seed: u64,
  rng: ChaCha20Rng,
}

impl Default for RandomArbiter {
  fn default() -> Self {
    let seed = thread_rng().gen();
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Self { seed, rng }
  }
}

impl Clone for RandomArbiter {
  fn clone(&self) -> Self {
    Self {
      seed: self.seed,
      rng: self.rng.clone(),
    }
  }
}

impl RandomArbiter {
  pub fn new(seed: Option<u64>) -> Self {
    let (rng, seed) = seed
      .map(|seed| (ChaCha20Rng::seed_from_u64(seed), seed))
      .unwrap_or_else(|| {
        let seed = thread_rng().gen();
        (ChaCha20Rng::seed_from_u64(seed), seed)
      });

    Self { seed, rng }
  }

  pub fn seed(&self) -> u64 {
    self.seed
  }
}

impl<V: Variant> Arbiter<V> for RandomArbiter {
  #[profiling::function]
  fn designate<D: Dimension, const DIM: usize>(
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

impl<V: Variant> Adjuster<V> for RandomArbiter {
  type Chained<C: Adjuster<V>> = MultiPhaseArbitration<V, Self, C>;

  fn revise<D: Dimension, const DIM: usize>(
    &mut self,
    _variant: &V,
    _cells: &mut Cells<V, D, DIM>,
  ) {
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster<V>,
  {
    MultiPhaseArbitration::new(self, other)
  }
}

#[derive(Debug)]
pub struct WeightArbiter<S: Shape> {
  seed: u64,
  rng: ChaCha20Rng,
  shape: S,
}

impl<S: Shape> Default for WeightArbiter<S>
where
  S: Default,
{
  fn default() -> Self {
    let seed = thread_rng().gen();
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Self {
      seed,
      rng,
      shape: S::default(),
    }
  }
}

impl<S: Shape> Clone for WeightArbiter<S>
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

impl<S: Shape> WeightArbiter<S> {
  pub fn new(seed: Option<u64>, shape: S) -> Self {
    let (rng, seed) = seed
      .map(|seed| (ChaCha20Rng::seed_from_u64(seed), seed))
      .unwrap_or_else(|| {
        let seed = thread_rng().gen();
        (ChaCha20Rng::seed_from_u64(seed), seed)
      });

    Self { seed, rng, shape }
  }

  pub fn seed(&self) -> u64 {
    self.seed
  }
}

impl<S: Shape> Arbiter<S::Variant> for WeightArbiter<S> {
  #[profiling::function]
  fn designate<D: Dimension, const DIM: usize>(
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

impl<S: Shape> Adjuster<S::Variant> for WeightArbiter<S> {
  type Chained<C: Adjuster<S::Variant>> = MultiPhaseArbitration<S::Variant, Self, C>;

  fn revise<D: Dimension, const DIM: usize>(
    &mut self,
    _variant: &S::Variant,
    _cells: &mut Cells<S::Variant, D, DIM>,
  ) {
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster<S::Variant>,
  {
    MultiPhaseArbitration::new(self, other)
  }
}

#[derive(Debug, Deref, DerefMut)]
pub struct LimitAdjuster<V: Variant>(HashMap<V, usize>);

impl<V: Variant> Clone for LimitAdjuster<V> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<V: Variant> LimitAdjuster<V> {
  pub fn new(limits: impl Into<HashMap<V, usize>>) -> Self {
    Self(limits.into())
  }
}

impl<V: Variant> Adjuster<V> for LimitAdjuster<V> {
  type Chained<C: Adjuster<V>> = (Self, C);

  #[profiling::function]
  fn revise<D: Dimension, const DIM: usize>(&mut self, variant: &V, cells: &mut Cells<V, D, DIM>) {
    let Some(limit) = self.get_mut(&variant) else {
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
    C: Adjuster<V>,
  {
    (self, other)
  }
}

pub struct MultiPhaseArbitration<V, A, Adj>
where
  V: Variant,
  A: Arbiter<V>,
  Adj: Adjuster<V>,
{
  arbiter: A,
  adjuster: Adj,
  _pd: PhantomData<V>,
}

impl<V, A, Adj> Clone for MultiPhaseArbitration<V, A, Adj>
where
  V: Variant,
  A: Arbiter<V> + Clone,
  Adj: Adjuster<V> + Clone,
{
  fn clone(&self) -> Self {
    Self {
      arbiter: self.arbiter.clone(),
      adjuster: self.adjuster.clone(),
      _pd: PhantomData,
    }
  }
}

impl<V, A, Adj> MultiPhaseArbitration<V, A, Adj>
where
  V: Variant,
  A: Arbiter<V>,
  Adj: Adjuster<V>,
{
  pub fn new(arbiter: A, adjuster: Adj) -> Self {
    Self {
      arbiter,
      adjuster,
      _pd: PhantomData,
    }
  }
}

impl<V, A, Adj> Arbiter<V> for MultiPhaseArbitration<V, A, Adj>
where
  V: Variant,
  A: Arbiter<V>,
  Adj: Adjuster<V>,
{
  fn designate<D: Dimension, const DIM: usize>(
    &mut self,
    cells: &mut Cells<V, D, DIM>,
  ) -> Result<Option<usize>, err::Error<DIM>> {
    self.arbiter.designate(cells)
  }
}

impl<V, A, Adj> Adjuster<V> for MultiPhaseArbitration<V, A, Adj>
where
  V: Variant,
  A: Arbiter<V>,
  Adj: Adjuster<V>,
{
  type Chained<C: Adjuster<V>> = MultiPhaseArbitration<V, A, (Adj, C)>;

  fn revise<D: Dimension, const DIM: usize>(&mut self, variant: &V, cells: &mut Cells<V, D, DIM>) {
    self.adjuster.revise(variant, cells);
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster<V>,
  {
    MultiPhaseArbitration::new(self.arbiter, (self.adjuster, other))
  }
}

impl<V, A0, A1> Adjuster<V> for (A0, A1)
where
  V: Variant,
  A0: Adjuster<V>,
  A1: Adjuster<V>,
{
  type Chained<C: Adjuster<V>> = ((A0, A1), C);

  fn revise<D: Dimension, const DIM: usize>(&mut self, variant: &V, cells: &mut Cells<V, D, DIM>) {
    self.0.revise(variant, cells);
    self.1.revise(variant, cells);
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster<V>,
  {
    (self, other)
  }
}
