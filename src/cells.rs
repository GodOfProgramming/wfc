use crate::{
  CellIndex, Dimension, Rules, Socket, UPos, Variant, err,
  util::{self, IPos, Size},
};
use derive_more::derive::Deref;
use ordermap::OrderSet;
use std::{
  collections::BTreeSet,
  fmt::Debug,
  ops::{Index, IndexMut},
};

/// Struct representing a collection of cells in some dimensional space
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Cells<V: Variant, D: Dimension, const DIM: usize> {
  #[cfg_attr(feature = "bevy", reflect(ignore))]
  pub size: Size<DIM>,
  pub list: Vec<Cell<V, D, DIM>>,

  #[cfg_attr(feature = "bevy", reflect(ignore))]
  pub entropy_cache: EntropyCache,
}

impl<V: Variant, D: Dimension, const DIM: usize> Cells<V, D, DIM> {
  #[profiling::function]
  pub fn new<S: Socket>(size: Size<DIM>, input: Vec<Option<V>>, rules: &Rules<V, D, S>) -> Self {
    let all_possibilities = BTreeSet::from_iter(rules.variants().cloned());
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
      .collect();

    Self {
      size,
      list,
      entropy_cache,
    }
  }

  pub fn at_pos(&self, pos: &IPos<DIM>) -> Option<&Cell<V, D, DIM>> {
    self.list.get(pos.index(self.size))
  }

  pub fn at(&self, index: usize) -> &Cell<V, D, DIM> {
    &self.list[index]
  }

  pub fn at_mut(&mut self, index: usize) -> &mut Cell<V, D, DIM> {
    &mut self.list[index]
  }

  pub fn set_entropy(&mut self, starting_entropy: usize, index: usize, new_entropy: usize) {
    self.entropy_cache.set(starting_entropy, index, new_entropy);
  }

  /// Acquires a list of uncollapsed cell indexes along a side of this group of cells
  /// Relies on the dimension being in order of - to + axis values
  pub fn uncollapsed_indexes_along_dir(&self, dir: D) -> Vec<usize> {
    let mut cells = Vec::new();

    // First get the index of this dimension value
    let dim_val_index = D::iter().position(|d| d == dir).unwrap();

    // Then get the dimensional number (0, 1 => first dimension => 0 value)
    let dim_num = dim_val_index / 2;
    // Also check if it's even, rules for even/odd explained below
    let even = dim_val_index & 1 == 0;

    let mut pos = UPos::<DIM>::default();

    if even {
      // increase along other even axis starting from this axis min size value
      pos[dim_num] = 0;
    } else {
      // increase along other odd axis starting from this axis max size value
      pos[dim_num] = self.size[dim_num] - 1;
    }

    // Now acquire all cells along the neighboring axis
    self.find_cells(1, dim_num, &mut pos, &mut cells);

    cells
  }

  /// Collapse the cell at the specified index
  #[profiling::function]
  pub fn collapse<'v, F>(&mut self, index: usize, collapse_fn: F) -> Result<(), err::Error<DIM>>
  where
    F: FnOnce(&Self, &BTreeSet<V>) -> Result<V, err::Error<DIM>>,
  {
    let cell = &self.list[index];

    // run the collapse function which returns the variant that should be used on this cell
    let variant = collapse_fn(self, &cell.possibilities)?;

    let cell = &mut self.list[index];

    // this cell will be collapsed, so clear its entropy from the cache
    self.entropy_cache.clear_entry(cell.entropy, index);
    // and collapse it to the selected variant
    cell.collapse(variant);

    Ok(())
  }
  pub fn lowest_entropy_indexes(&self) -> Option<&OrderSet<usize>> {
    self.entropy_cache.lowest()
  }

  /// recursively finds cells along a side of this collection of cells
  fn find_cells(
    &self,
    dimension: usize,
    dim_num: usize,
    pos: &mut UPos<DIM>,
    cells: &mut Vec<usize>,
  ) {
    let index = pos.index(self.size);
    let cell = self.at(index);
    if !cell.collapsed() {
      cells.push(pos.index(self.size));
    }
    if dimension < DIM {
      let d = util::wrap(dim_num + dimension, DIM);
      for i in 1..self.size[d] {
        pos[d] = i;
        self.find_cells(dimension + 1, dim_num, pos, cells);
      }
    }
  }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Cell<V: Variant, D: Dimension, const DIM: usize> {
  pub possibilities: BTreeSet<V>,
  pub neighbors: Vec<(CellIndex, D)>,
  pub entropy: usize,

  #[cfg_attr(feature = "bevy", reflect(ignore))]
  pub position: IPos<DIM>,
}

impl<V: Variant, D: Dimension, const DIM: usize> Cell<V, D, DIM> {
  fn new(position: IPos<DIM>, possibilities: impl Into<BTreeSet<V>>, size: Size<DIM>) -> Self {
    let possibilities = possibilities.into();
    let entropy = possibilities.len();
    Self {
      possibilities,
      entropy,
      neighbors: Self::neighbors(position, size).collect(),
      position,
    }
  }

  pub fn new_collapsed(position: IPos<DIM>, collapsed_variant: V, size: Size<DIM>) -> Self {
    Self {
      possibilities: BTreeSet::from_iter([collapsed_variant]),
      entropy: 0,
      neighbors: Self::neighbors(position, size).collect(),
      position,
    }
  }

  pub fn selected_variant(&self) -> Option<&V> {
    self
      .collapsed()
      .then(|| self.possibilities.iter().next())
      .flatten()
  }

  pub fn collapse(&mut self, variant: V) {
    self.possibilities = BTreeSet::from([variant]);
    self.entropy = 0;
  }

  pub fn remove_variant(&mut self, variant: &V) {
    self.possibilities.remove(variant);
    self.entropy = self.possibilities.len();
  }

  pub fn collapsed(&self) -> bool {
    self.entropy == 0
  }

  fn neighbors(position: IPos<DIM>, size: Size<DIM>) -> impl Iterator<Item = (CellIndex, D)> {
    D::iter().filter_map(move |dir| {
      let npos = position + dir;
      size.contains(&npos).then(|| (npos.index(size), dir))
    })
  }
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
