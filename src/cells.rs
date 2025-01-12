use crate::{
  err,
  rules::{AbstractRules, Legend},
  util::{self, IPos, Size},
  CellIndex, Dimension, DimensionId, Socket, UPos, Variant, VariantId,
};
use derive_more::derive::Deref;
use ordermap::OrderSet;
use std::{
  collections::BTreeSet,
  fmt::Debug,
  ops::{Index, IndexMut},
};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct Cells<const DIM: usize> {
  pub size: Size<DIM>,
  pub list: Vec<Cell<DIM>>,

  #[cfg_attr(feature = "bevy", reflect(ignore))]
  pub entropy_cache: EntropyCache,
}

impl<const DIM: usize> Cells<DIM> {
  #[profiling::function]
  pub fn new<V: Variant, D: Dimension, S: Socket>(
    size: Size<DIM>,
    input: Vec<Option<V>>,
    rules: &AbstractRules,
    legend: &Legend<V, D, S>,
  ) -> Self {
    let all_possibilities = BTreeSet::from_iter(rules.variants().cloned());
    let mut entropy_cache = EntropyCache::new(all_possibilities.len());
    let max_entropy = entropy_cache.len();

    let list = input
      .into_iter()
      .enumerate()
      .map(|(i, input)| {
        let position = IPos::from_index(i, size);
        input
          .map(|variant| Cell::new_collapsed(position, &variant, size, legend))
          .unwrap_or_else(|| {
            entropy_cache[max_entropy].insert(i);
            Cell::new(position, all_possibilities.clone(), size, legend)
          })
      })
      .collect::<Vec<Cell<DIM>>>();

    Self {
      size,
      list,
      entropy_cache,
    }
  }

  pub fn at_pos(&self, pos: &IPos<DIM>) -> Option<&Cell<DIM>> {
    self.list.get(pos.index(self.size))
  }

  pub fn at(&self, index: usize) -> &Cell<DIM> {
    &self.list[index]
  }

  pub fn at_mut(&mut self, index: usize) -> &mut Cell<DIM> {
    &mut self.list[index]
  }

  pub fn set_entropy(&mut self, starting_entropy: usize, index: usize, new_entropy: usize) {
    self.entropy_cache.set(starting_entropy, index, new_entropy);
  }

  pub fn uncollapsed_indexes_along_dir(&self, dir: DimensionId) -> Vec<usize> {
    let mut cells = Vec::new();

    let dindex = *dir / 2;
    let even = *dir & 1 == 0;
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
  pub fn collapse<'v, F>(&mut self, index: usize, collapse_fn: F) -> Result<(), err::Error<DIM>>
  where
    F: FnOnce(&Self, &BTreeSet<VariantId>) -> Result<VariantId, err::Error<DIM>>,
  {
    let cell = &self.at(index);

    let variant = collapse_fn(self, &cell.possibilities)?;

    let cell = &mut self.list[index];

    self.entropy_cache.clear_entry(cell.entropy, index);
    cell.collapse(variant);

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
pub struct Cell<const DIM: usize> {
  pub possibilities: BTreeSet<VariantId>,
  pub neighbors: Vec<(CellIndex, DimensionId)>,
  pub entropy: usize,

  pub position: IPos<DIM>,
}

impl<const DIM: usize> Cell<DIM> {
  fn new<V: Variant, D: Dimension, S: Socket>(
    position: IPos<DIM>,
    possibilities: impl Into<BTreeSet<VariantId>>,
    size: Size<DIM>,
    legend: &Legend<V, D, S>,
  ) -> Self {
    let possibilities = possibilities.into();
    let entropy = possibilities.len();
    Self {
      possibilities,
      entropy,
      neighbors: Self::neighbors(position, size, legend),
      position,
    }
  }

  pub fn new_collapsed<V: Variant, D: Dimension, S: Socket>(
    position: IPos<DIM>,
    collapsed_variant: &V,
    size: Size<DIM>,
    legend: &Legend<V, D, S>,
  ) -> Self {
    Self {
      possibilities: BTreeSet::from_iter([legend.variant_id(collapsed_variant)]),
      entropy: 0,
      neighbors: Self::neighbors(position, size, legend),
      position,
    }
  }

  pub fn selected_variant(&self) -> Option<VariantId> {
    self
      .collapsed()
      .then(|| self.possibilities.iter().next().cloned())
      .flatten()
  }

  pub fn collapse(&mut self, variant: VariantId) {
    self.possibilities = BTreeSet::from([variant]);
    self.entropy = 0;
  }

  pub fn remove_variant(&mut self, variant: VariantId) {
    self.possibilities.remove(&variant);
    self.entropy = self.possibilities.len();
  }

  pub fn collapsed(&self) -> bool {
    self.entropy == 0
  }

  fn neighbors<V: Variant, D: Dimension, S: Socket>(
    position: IPos<DIM>,
    size: Size<DIM>,
    legend: &Legend<V, D, S>,
  ) -> Vec<(CellIndex, DimensionId)> {
    D::iter()
      .filter_map(|dir| {
        let npos = position + legend.dimension_id(&dir);
        size
          .contains(&npos)
          .then(|| (npos.index(size), legend.dimension_id(&dir)))
      })
      .collect()
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
