use crate::{cells::Cells, Adjuster, Arbiter, Error, Shape, TResult, TypeAtlas};
use derive_more::derive::{Deref, DerefMut};
use rand::{
  seq::{IteratorRandom, SliceRandom},
  thread_rng, Rng, SeedableRng,
};
use rand_chacha::ChaCha20Rng;
use std::{collections::HashMap, iter::Iterator, marker::PhantomData};

#[derive(Debug)]
pub struct RandomArbiter<T: TypeAtlas<DIM>, const DIM: usize> {
  seed: u64,
  rng: ChaCha20Rng,
  _pd: PhantomData<T>,
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Default for RandomArbiter<T, DIM> {
  fn default() -> Self {
    let seed = thread_rng().gen();
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Self {
      seed,
      rng,
      _pd: PhantomData,
    }
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Clone for RandomArbiter<T, DIM> {
  fn clone(&self) -> Self {
    Self {
      seed: self.seed,
      rng: self.rng.clone(),
      _pd: PhantomData,
    }
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> RandomArbiter<T, DIM> {
  pub fn new(seed: Option<u64>) -> Self {
    let (rng, seed) = seed
      .map(|seed| (ChaCha20Rng::seed_from_u64(seed), seed))
      .unwrap_or_else(|| {
        let seed = thread_rng().gen();
        (ChaCha20Rng::seed_from_u64(seed), seed)
      });

    Self {
      seed,
      rng,
      _pd: PhantomData,
    }
  }

  pub fn seed(&self) -> u64 {
    self.seed
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Arbiter<T, DIM> for RandomArbiter<T, DIM> {
  #[profiling::function]
  fn designate(&mut self, cells: &mut Cells<T, DIM>) -> TResult<Option<usize>, T, DIM> {
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

impl<T: TypeAtlas<DIM>, const DIM: usize> Adjuster<T, DIM> for RandomArbiter<T, DIM> {
  type Chained<C: Adjuster<T, DIM>> = MultiPhaseArbitration<Self, C, T, DIM>;

  fn revise(&mut self, _variant: &T::Variant, _cells: &mut Cells<T, DIM>) {}

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster<T, DIM>,
  {
    MultiPhaseArbitration::new(self, other)
  }
}

#[derive(Debug)]
pub struct WeightArbiter<S: Shape<T, DIM>, T: TypeAtlas<DIM>, const DIM: usize> {
  seed: u64,
  rng: ChaCha20Rng,
  shape: S,
  _pd: PhantomData<T>,
}

impl<S, T: TypeAtlas<DIM>, const DIM: usize> Default for WeightArbiter<S, T, DIM>
where
  S: Shape<T, DIM> + Default,
{
  fn default() -> Self {
    let seed = thread_rng().gen();
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Self {
      seed,
      rng,
      shape: S::default(),
      _pd: PhantomData,
    }
  }
}

impl<S, T: TypeAtlas<DIM>, const DIM: usize> Clone for WeightArbiter<S, T, DIM>
where
  S: Shape<T, DIM> + Clone,
{
  fn clone(&self) -> Self {
    Self {
      seed: self.seed,
      rng: self.rng.clone(),
      shape: self.shape.clone(),
      _pd: PhantomData,
    }
  }
}

impl<S: Shape<T, DIM>, T: TypeAtlas<DIM>, const DIM: usize> WeightArbiter<S, T, DIM> {
  pub fn new(seed: Option<u64>, shape: S) -> Self {
    let (rng, seed) = seed
      .map(|seed| (ChaCha20Rng::seed_from_u64(seed), seed))
      .unwrap_or_else(|| {
        let seed = thread_rng().gen();
        (ChaCha20Rng::seed_from_u64(seed), seed)
      });

    Self {
      seed,
      rng,
      shape,
      _pd: PhantomData,
    }
  }

  pub fn seed(&self) -> u64 {
    self.seed
  }
}

impl<S: Shape<T, DIM>, T: TypeAtlas<DIM>, const DIM: usize> Arbiter<T, DIM>
  for WeightArbiter<S, T, DIM>
{
  #[profiling::function]
  fn designate(&mut self, cells: &mut Cells<T, DIM>) -> TResult<Option<usize>, T, DIM> {
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
          self.shape.weight(variant, index, cells)
        })
        .cloned()
        .cloned()
        .map_err(|_| Error::NoPossibilities)
    })?;

    Ok(Some(index))
  }
}

impl<S: Shape<T, DIM>, T: TypeAtlas<DIM>, const DIM: usize> Adjuster<T, DIM>
  for WeightArbiter<S, T, DIM>
{
  type Chained<C: Adjuster<T, DIM>> = MultiPhaseArbitration<Self, C, T, DIM>;

  fn revise(&mut self, _variant: &T::Variant, _cells: &mut Cells<T, DIM>) {}

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster<T, DIM>,
  {
    MultiPhaseArbitration::new(self, other)
  }
}

#[derive(Debug, Deref, DerefMut)]
pub struct LimitAdjuster<T: TypeAtlas<DIM>, const DIM: usize>(HashMap<T::Variant, usize>);

impl<T: TypeAtlas<DIM>, const DIM: usize> Clone for LimitAdjuster<T, DIM> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> LimitAdjuster<T, DIM> {
  pub fn new(limits: impl Into<HashMap<T::Variant, usize>>) -> Self {
    Self(limits.into())
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Adjuster<T, DIM> for LimitAdjuster<T, DIM> {
  type Chained<C: Adjuster<T, DIM>> = (Self, C);

  #[profiling::function]
  fn revise(&mut self, variant: &T::Variant, cells: &mut Cells<T, DIM>) {
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
    C: Adjuster<T, DIM>,
  {
    (self, other)
  }
}

pub struct MultiPhaseArbitration<A, Adj, T: TypeAtlas<DIM>, const DIM: usize>
where
  A: Arbiter<T, DIM>,
  Adj: Adjuster<T, DIM>,
{
  arbiter: A,
  adjuster: Adj,
  _pd: PhantomData<T>,
}

impl<A, Adj, T: TypeAtlas<DIM>, const DIM: usize> Clone for MultiPhaseArbitration<A, Adj, T, DIM>
where
  A: Arbiter<T, DIM> + Clone,
  Adj: Adjuster<T, DIM> + Clone,
{
  fn clone(&self) -> Self {
    Self {
      arbiter: self.arbiter.clone(),
      adjuster: self.adjuster.clone(),
      _pd: PhantomData,
    }
  }
}

impl<A, Adj, T: TypeAtlas<DIM>, const DIM: usize> MultiPhaseArbitration<A, Adj, T, DIM>
where
  A: Arbiter<T, DIM>,
  Adj: Adjuster<T, DIM>,
{
  pub fn new(arbiter: A, adjuster: Adj) -> Self {
    Self {
      arbiter,
      adjuster,
      _pd: PhantomData,
    }
  }
}

impl<A, Adj, T: TypeAtlas<DIM>, const DIM: usize> Arbiter<T, DIM>
  for MultiPhaseArbitration<A, Adj, T, DIM>
where
  A: Arbiter<T, DIM>,
  Adj: Adjuster<T, DIM>,
{
  fn designate(&mut self, cells: &mut Cells<T, DIM>) -> TResult<Option<usize>, T, DIM> {
    self.arbiter.designate(cells)
  }
}

impl<A, Adj, T: TypeAtlas<DIM>, const DIM: usize> Adjuster<T, DIM>
  for MultiPhaseArbitration<A, Adj, T, DIM>
where
  A: Arbiter<T, DIM>,
  Adj: Adjuster<T, DIM>,
{
  type Chained<C: Adjuster<T, DIM>> = MultiPhaseArbitration<A, (Adj, C), T, DIM>;

  fn revise(&mut self, variant: &T::Variant, cells: &mut Cells<T, DIM>) {
    self.adjuster.revise(variant, cells);
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster<T, DIM>,
  {
    MultiPhaseArbitration::new(self.arbiter, (self.adjuster, other))
  }
}

impl<A0, A1, T: TypeAtlas<DIM>, const DIM: usize> Adjuster<T, DIM> for (A0, A1)
where
  A0: Adjuster<T, DIM>,
  A1: Adjuster<T, DIM>,
{
  type Chained<C: Adjuster<T, DIM>> = ((A0, A1), C);

  fn revise(&mut self, variant: &T::Variant, cells: &mut Cells<T, DIM>) {
    self.0.revise(variant, cells);
    self.1.revise(variant, cells);
  }

  fn chain<C>(self, other: C) -> Self::Chained<C>
  where
    C: Adjuster<T, DIM>,
  {
    (self, other)
  }
}
