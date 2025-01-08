use crate::{
  util::{self, IPos, Size},
  Dimension, Rules, TResult, TypeAtlas, UPos,
};
use derive_more::derive::Deref;
use ordermap::OrderSet;
use std::{
  fmt::Debug,
  hash::Hash,
  ops::{Index, IndexMut},
};
use strum::{IntoEnumIterator, VariantArray};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Cells<T: TypeAtlas<DIM>, const DIM: usize> {
  pub size: Size<DIM>,
  pub list: Vec<Cell<T::Variant, T::Dimension, DIM>>,

  #[cfg_attr(feature = "bevy", reflect(ignore))]
  pub entropy_cache: EntropyCache,
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Cells<T, DIM> {
  #[profiling::function]
  pub fn new(
    size: Size<DIM>,
    input: Vec<Option<T::Variant>>,
    rules: &Rules<T::Variant, T::Dimension, T::Socket>,
  ) -> Self {
    let all_possibilities = OrderSet::from_iter(rules.keys().cloned());
    let mut entropy_cache = EntropyCache::new(all_possibilities.len());
    let max_entropy = entropy_cache.len();

    let list = input
      .into_iter()
      .enumerate()
      .map(|(i, input)| {
        let position = IPos::from_index(i, size);
        input
          .map(|variant| Cell::new_collapsed(position, variant, size))
          .unwrap_or_else(|| {
            entropy_cache[max_entropy].insert(i);
            Cell::new(position, all_possibilities.clone(), size)
          })
      })
      .collect::<Vec<Cell<T::Variant, T::Dimension, DIM>>>();

    Self {
      size,
      list,
      entropy_cache,
    }
  }

  pub fn at_pos(&self, pos: &IPos<DIM>) -> Option<&Cell<T::Variant, T::Dimension, DIM>> {
    self.list.get(pos.index(self.size))
  }

  pub fn at(&self, index: usize) -> &Cell<T::Variant, T::Dimension, DIM> {
    &self.list[index]
  }

  pub fn at_mut(&mut self, index: usize) -> &mut Cell<T::Variant, T::Dimension, DIM> {
    &mut self.list[index]
  }

  pub fn set_entropy(&mut self, starting_entropy: usize, index: usize, new_entropy: usize) {
    self.entropy_cache.set(starting_entropy, index, new_entropy);
  }

  pub fn uncollapsed_indexes_along_dir(&self, dir: T::Dimension) -> Vec<usize> {
    T::Dimension::VARIANTS[0];

    let mut cells = Vec::new();

    let index = T::Dimension::iter().position(|d| d == dir).unwrap();

    let dindex = index / 2;
    let even = index & 1 == 0;
    let mut pos = UPos::<DIM>::default();

    if even {
      // increase along other even axis starting from this axis min size value
      pos[dindex] = 0;
    } else {
      // increase along other odd axis starting from this axis max size value
      pos[dindex] = self.size[dindex] - 1;
    }

    self.find_cells(1, dindex, &mut pos, &mut cells);

    cells
  }

  #[profiling::function]
  pub fn collapse<'v, F>(&mut self, index: usize, collapse_fn: F) -> TResult<(), T, DIM>
  where
    F: FnOnce(&Self, &OrderSet<T::Variant>) -> TResult<T::Variant, T, DIM>,
  {
    let cell = &self.at(index);

    if let CellState::Unstable(variants) = &cell.state {
      let variant = collapse_fn(self, variants)?.clone();
      self.entropy_cache.clear_entry(variants.len(), index);

      let cell = &mut self.list[index];
      cell.collapse(variant);
    }

    Ok(())
  }
  pub fn lowest_entropy_indexes(&self) -> Option<&OrderSet<usize>> {
    self.entropy_cache.lowest()
  }

  fn find_cells(
    &self,
    dimension: usize,
    dindex: usize,
    pos: &mut UPos<DIM>,
    cells: &mut Vec<usize>,
  ) {
    let index = pos.index(self.size);
    let cell = self.at(index);
    if !cell.collapsed() {
      cells.push(pos.index(self.size));
    }
    if dimension < DIM {
      let d = util::wrap(dindex + dimension, DIM);
      for i in 1..self.size[d] {
        pos[d] = i;
        self.find_cells(dimension + 1, dindex, pos, cells);
      }
    }
  }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Cell<V, D, const DIM: usize>
where
  V: Ord + Hash + Eq,
{
  state: CellState<V>,
  neighbors: Vec<(usize, D)>,

  pos: IPos<DIM>,
}

impl<V, D, const DIM: usize> Cell<V, D, DIM>
where
  V: Ord + Hash + Eq,
  D: Dimension,
{
  fn new(position: IPos<DIM>, possibilities: impl Into<OrderSet<V>>, size: Size<DIM>) -> Self {
    Self {
      state: CellState::Unstable(possibilities.into()),
      neighbors: Self::neighbors_of(position, size),
      pos: position,
    }
  }

  pub fn new_collapsed(pos: IPos<DIM>, collapsed_variant: V, size: Size<DIM>) -> Self
  where
    V: Ord,
  {
    Self {
      state: CellState::Collapsed(collapsed_variant),
      neighbors: Self::neighbors_of(pos, size),
      pos,
    }
  }

  pub fn selected_variant(&self) -> Option<&V> {
    match &self.state {
      CellState::Collapsed(variant) => Some(variant),
      CellState::Unstable(_) => None,
    }
  }

  pub fn collapse(&mut self, variant: V)
  where
    V: Ord,
  {
    self.state = CellState::Collapsed(variant);
  }

  pub fn remove_variant(&mut self, variant: &V)
  where
    V: Ord,
  {
    match &mut self.state {
      CellState::Collapsed(_) => (),
      CellState::Unstable(variants) => {
        variants.swap_remove(variant);
      }
    }
  }

  pub fn collapsed(&self) -> bool {
    matches!(self.state, CellState::Collapsed(_))
  }

  pub fn pos(&self) -> &IPos<DIM> {
    &self.pos
  }

  pub fn neighbors(&self) -> &Vec<(usize, D)> {
    &self.neighbors
  }

  pub fn state(&self) -> &CellState<V> {
    &self.state
  }

  pub fn state_mut(&mut self) -> &mut CellState<V> {
    &mut self.state
  }

  pub fn entropy(&self) -> Option<usize> {
    match &self.state {
      CellState::Collapsed(_) => None,
      CellState::Unstable(variants) => Some(variants.len()),
    }
  }

  fn neighbors_of(position: IPos<DIM>, size: Size<DIM>) -> Vec<(usize, D)> {
    D::iter()
      .filter_map(|dir| {
        let npos = position + dir;
        size.contains(&npos).then(|| (npos.index(size), dir))
      })
      .collect()
  }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub enum CellState<V>
where
  V: Eq + Hash,
{
  Collapsed(V),
  Unstable(OrderSet<V>),
}

#[derive(Default, Debug, Deref)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntropyCache(Vec<OrderSet<usize>>);

impl EntropyCache {
  fn new(max_entropy: usize) -> Self {
    Self(vec![OrderSet::new(); max_entropy])
  }

  #[profiling::function]
  pub fn lowest(&self) -> Option<&OrderSet<usize>> {
    self.iter().find(|level| !level.is_empty())
  }

  pub fn set(&mut self, starting_entropy: usize, index: usize, new_entropy: usize) {
    self[starting_entropy].swap_remove(&index);
    self[new_entropy].insert(index);
  }

  pub fn clear_entry(&mut self, entropy: usize, index: usize) {
    self[entropy].swap_remove(&index);
  }
}

impl Index<usize> for EntropyCache {
  type Output = OrderSet<usize>;

  fn index(&self, index: usize) -> &Self::Output {
    &self.0[index - 1]
  }
}

impl IndexMut<usize> for EntropyCache {
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    &mut self.0[index - 1]
  }
}
