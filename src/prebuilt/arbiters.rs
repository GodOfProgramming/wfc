use crate::{
  cells::Cells, err, Adjuster, Arbiter, CellIndex, Dimension, Error, Rules, Shape, Socket, Variant,
  VariantId,
};
use derive_more::derive::{Deref, DerefMut};
use rand::{
  seq::{IteratorRandom, SliceRandom},
  thread_rng, Rng, SeedableRng,
};
use rand_chacha::ChaCha20Rng;
use std::{borrow::Borrow, collections::HashMap, iter::Iterator};

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

impl Arbiter for RandomArbiter {
  #[profiling::function]
  fn designate<const DIM: usize>(
    &mut self,
    cells: &mut Cells<DIM>,
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

impl Adjuster for RandomArbiter {
  type Chained<C: Adjuster> = MultiPhaseArbitration<Self, C>;

  fn revise<const DIM: usize>(&mut self, _variant: VariantId, _cells: &mut Cells<DIM>) {}

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster,
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

impl<S: Shape> Arbiter for WeightArbiter<S> {
  #[profiling::function]
  fn designate<const DIM: usize>(
    &mut self,
    cells: &mut Cells<DIM>,
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
          self.shape.weight(**variant, index, cells)
        })
        .cloned()
        .cloned()
        .map_err(|_| Error::NoPossibilities)
    })?;

    Ok(Some(index))
  }
}

impl<S: Shape> Adjuster for WeightArbiter<S> {
  type Chained<C: Adjuster> = MultiPhaseArbitration<Self, C>;

  fn revise<const DIM: usize>(&mut self, _variant: VariantId, _cells: &mut Cells<DIM>) {}

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster,
  {
    MultiPhaseArbitration::new(self, other)
  }
}

#[derive(Debug, Deref, DerefMut)]
pub struct LimitAdjuster(HashMap<VariantId, usize>);

impl Clone for LimitAdjuster {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl LimitAdjuster {
  pub fn new<V: Variant, D: Dimension, S: Socket>(
    limits: impl IntoIterator<Item = (V, usize)>,
    rules: impl Borrow<Rules<V, D, S>>,
  ) -> Self {
    Self(
      limits
        .into_iter()
        .map(|(v, count)| (rules.borrow().legend().variant_id(&v), count))
        .collect(),
    )
  }
}

impl Adjuster for LimitAdjuster {
  type Chained<C: Adjuster> = (Self, C);

  #[profiling::function]
  fn revise<const DIM: usize>(&mut self, variant: VariantId, cells: &mut Cells<DIM>) {
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
    C: Adjuster,
  {
    (self, other)
  }
}

pub struct MultiPhaseArbitration<A, Adj>
where
  A: Arbiter,
  Adj: Adjuster,
{
  arbiter: A,
  adjuster: Adj,
}

impl<A, Adj> Clone for MultiPhaseArbitration<A, Adj>
where
  A: Arbiter + Clone,
  Adj: Adjuster + Clone,
{
  fn clone(&self) -> Self {
    Self {
      arbiter: self.arbiter.clone(),
      adjuster: self.adjuster.clone(),
    }
  }
}

impl<A, Adj> MultiPhaseArbitration<A, Adj>
where
  A: Arbiter,
  Adj: Adjuster,
{
  pub fn new(arbiter: A, adjuster: Adj) -> Self {
    Self { arbiter, adjuster }
  }
}

impl<A, Adj> Arbiter for MultiPhaseArbitration<A, Adj>
where
  A: Arbiter,
  Adj: Adjuster,
{
  fn designate<const DIM: usize>(
    &mut self,
    cells: &mut Cells<DIM>,
  ) -> Result<Option<usize>, err::Error<DIM>> {
    self.arbiter.designate(cells)
  }
}

impl<A, Adj> Adjuster for MultiPhaseArbitration<A, Adj>
where
  A: Arbiter,
  Adj: Adjuster,
{
  type Chained<C: Adjuster> = MultiPhaseArbitration<A, (Adj, C)>;

  fn revise<const DIM: usize>(&mut self, variant: VariantId, cells: &mut Cells<DIM>) {
    self.adjuster.revise(variant, cells);
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster,
  {
    MultiPhaseArbitration::new(self.arbiter, (self.adjuster, other))
  }
}

impl<A0, A1> Adjuster for (A0, A1)
where
  A0: Adjuster,
  A1: Adjuster,
{
  type Chained<C: Adjuster> = ((A0, A1), C);

  fn revise<const DIM: usize>(&mut self, variant: VariantId, cells: &mut Cells<DIM>) {
    self.0.revise(variant, cells);
    self.1.revise(variant, cells);
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster,
  {
    (self, other)
  }
}
