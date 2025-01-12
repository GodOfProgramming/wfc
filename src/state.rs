use crate::{
  cells::{Cell, Cells},
  util::{self, Size, UPos},
  Adjuster, Arbiter, Constraint, Dimension, Error, Observation, Rules, TResult, TypeAtlas,
};
use derive_more::derive::{Deref, DerefMut};
use std::{
  collections::{BTreeSet, HashMap, HashSet},
  fmt::Debug,
};
use strum::EnumCount;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct StateBuilder<T: TypeAtlas<DIM>, const DIM: usize> {
  arbiter: T::Arbiter,
  constraint: T::Constraint,
  size: Size<DIM>,
  input: Vec<Option<T::Variant>>,
  rules: Option<Rules<T::Variant, T::Dimension, T::Socket>>,
  external_cells: ExtCells<T, DIM>,
}

impl<T: TypeAtlas<DIM>, const DIM: usize> StateBuilder<T, DIM> {
  pub fn new(
    size: impl Into<Size<DIM>>,
    adjuster: impl Into<T::Arbiter>,
    constraint: impl Into<T::Constraint>,
  ) -> Self {
    let size = size.into();
    Self {
      arbiter: adjuster.into(),
      constraint: constraint.into(),
      size,
      input: vec![None; size.len()],
      rules: Default::default(),
      external_cells: ExtCells::new(size),
    }
  }

  pub fn with_rules(
    &mut self,
    rules: impl Into<Rules<T::Variant, T::Dimension, T::Socket>>,
  ) -> &mut Self {
    self.rules = Some(rules.into());
    self
  }

  pub fn with_ext(&mut self, dir: T::Dimension, source: Vec<T::Variant>) -> &mut Self {
    self.external_cells.insert(dir, source);
    self
  }

  pub fn insert(&mut self, pos: impl Into<UPos<DIM>>, value: T::Variant) -> &mut Self {
    let pos = pos.into();
    self.input[pos.index(self.size)] = Some(value);
    self
  }

  pub fn size(&self) -> &Size<DIM> {
    &self.size
  }

  pub fn build(self) -> TResult<State<T, DIM>, T, DIM> {
    // seemingly cannot be done at compile time because
    // M::Dimensions::COUNT is not accessible inside static asserts
    if DIM != T::Dimension::COUNT / 2 {
      return Err(Error::DimensionMismatch(DIM, T::Dimension::COUNT));
    }

    let rules = self.rules.unwrap_or_default();

    State::new(
      self.size,
      self.input,
      rules,
      self.arbiter,
      self.constraint,
      self.external_cells,
    )
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Clone for StateBuilder<T, DIM>
where
  T::Arbiter: Clone,
  T::Constraint: Clone,
{
  fn clone(&self) -> Self {
    Self {
      arbiter: self.arbiter.clone(),
      constraint: self.constraint.clone(),
      size: self.size,
      input: self.input.clone(),
      rules: self.rules.clone(),
      external_cells: self.external_cells.clone(),
    }
  }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct State<T: TypeAtlas<DIM>, const DIM: usize> {
  cells: Cells<T, DIM>,
  rules: Rules<T::Variant, T::Dimension, T::Socket>,
  arbiter: T::Arbiter,
  constraint: T::Constraint,
  socket_cache: SocketCache<T, DIM>,
}

impl<T: TypeAtlas<DIM>, const DIM: usize> State<T, DIM> {
  #[profiling::function]
  fn new(
    size: Size<DIM>,
    input: Vec<Option<T::Variant>>,
    rules: Rules<T::Variant, T::Dimension, T::Socket>,
    arbiter: T::Arbiter,
    constraint: T::Constraint,
    external_cells: ExtCells<T, DIM>,
  ) -> TResult<Self, T, DIM> {
    let mut this = Self {
      cells: Cells::new(size, input, &rules),
      rules,
      arbiter,
      constraint,
      socket_cache: Default::default(),
    };

    for (dir, ext) in external_cells.sides.into_iter() {
      let indexes = this.cells.uncollapsed_indexes_along_dir(dir);
      for index in indexes {
        let cell = this.cells.at_mut(index);
        let neighbor = cell.position + dir;
        let neighbor_index = neighbor.index_in(external_cells.size);
        let external_variant = BTreeSet::from_iter([ext[neighbor_index].clone()]);

        let starting_entropy = cell.entropy;
        Self::constrain(
          cell,
          &this.constraint,
          &external_variant,
          &dir.opposite(),
          &this.rules,
          &mut this.socket_cache,
        )?;
        let new_entropy = cell.entropy;

        if starting_entropy != new_entropy {
          this.cells.set_entropy(starting_entropy, index, new_entropy);
        }

        this.propagate(index)?;
      }
    }

    let propagations = this
      .cells
      .list
      .iter()
      .enumerate()
      .filter_map(|(i, cell)| cell.selected_variant().cloned().map(|variant| (i, variant)))
      .collect::<Vec<_>>();

    for (i, variant) in propagations {
      this.arbiter.revise(&variant, &mut this.cells);
      this.propagate(i)?;
    }

    Ok(this)
  }

  #[profiling::function]
  pub fn collapse(&mut self) -> TResult<Observation, T, DIM> {
    let Some(index) = self.arbiter.designate(&mut self.cells)? else {
      return Ok(Observation::Complete);
    };

    let cell = &self.cells.list[index];
    let possibility = cell.selected_variant().cloned().unwrap();

    self.arbiter.revise(&possibility, &mut self.cells);
    self.propagate(index)?;

    Ok(Observation::Incomplete(index))
  }

  #[profiling::function]
  fn propagate(&mut self, cell_index: usize) -> TResult<(), T, DIM> {
    let mut stack = Vec::with_capacity(T::Dimension::COUNT);
    stack.push(cell_index);

    while let Some(cell_index) = stack.pop() {
      let cell = &self.cells.at(cell_index);

      let neighbors = cell
        .neighbors
        .iter()
        .filter(|(i, _)| !self.cells.list[*i].collapsed())
        .cloned()
        .collect::<Vec<_>>();

      for (neighbor_index, direction) in neighbors {
        let [cell, neighbor] =
          unsafe { util::index_twice_mut(&mut self.cells.list, cell_index, neighbor_index) };

        let starting_entropy = neighbor.entropy;
        Self::constrain(
          neighbor,
          &self.constraint,
          &cell.possibilities,
          &direction,
          &self.rules,
          &mut self.socket_cache,
        )?;
        let new_entropy = neighbor.entropy;

        // if reduced, then push this neighbor onto the stack to propagate its changes to its neighbors
        if starting_entropy != new_entropy {
          self
            .cells
            .set_entropy(starting_entropy, neighbor_index, new_entropy);
          stack.push(neighbor_index);
        }
      }
    }

    Ok(())
  }

  pub fn data(&self) -> Vec<T::Variant>
  where
    T::Variant: Default,
  {
    self
      .cells
      .list
      .iter()
      .map(|cell| {
        cell
          .selected_variant()
          .cloned()
          .unwrap_or_else(T::Variant::default)
      })
      .collect()
  }

  pub fn data_raw(&self) -> Vec<Option<T::Variant>> {
    self
      .cells
      .list
      .iter()
      .map(|cell| cell.selected_variant().cloned())
      .collect()
  }

  pub fn cells(&self) -> &Cells<T, DIM> {
    &self.cells
  }

  pub fn size(&self) -> &Size<DIM> {
    &self.cells.size
  }

  pub fn rules(&self) -> &Rules<T::Variant, T::Dimension, T::Socket> {
    &self.rules
  }

  pub fn constrainer(&self) -> &T::Constraint {
    &self.constraint
  }

  /// Tries to reduce the number of possibilities this tile can be based on a neighbor
  /// Returns true if at least one reduction was made, false otherwise
  #[profiling::function]
  fn constrain(
    cell: &mut Cell<T::Variant, T::Dimension, DIM>,
    constraint: &T::Constraint,
    neighbor_possibilities: &BTreeSet<T::Variant>,
    neighbor_to_self_dir: &T::Dimension,
    rules: &Rules<T::Variant, T::Dimension, T::Socket>,
    cache: &mut SocketCache<T, DIM>,
  ) -> TResult<(), T, DIM> {
    let neighbor_sockets: &HashSet<T::Socket> = {
      profiling::function_scope!("Neighbor Sockets");

      match cache.lookup(neighbor_possibilities, neighbor_to_self_dir) {
        Some(Some(sockets)) => sockets,
        Some(None) => cache.partial_create(rules, neighbor_possibilities, neighbor_to_self_dir),
        None => cache.full_create(rules, neighbor_possibilities, neighbor_to_self_dir),
      }
    };

    let opposite = neighbor_to_self_dir.opposite();

    cell.possibilities.retain(|variant| {
      let Some(self_rule) = rules.get(variant) else {
        return false;
      };

      self_rule
        .get(&opposite)
        .map(|socket| constraint.check(socket, neighbor_sockets))
        .unwrap_or(false) // no rule means no connection
    });

    if cell.possibilities.is_empty() {
      return Err(Error::Contradiction {
        position: cell.position,
        neighbor: cell.position + neighbor_to_self_dir.opposite(),
        direction: neighbor_to_self_dir.opposite(),
        neighbor_variants: Vec::from_iter(neighbor_possibilities.iter().cloned()),
        neighbor_sockets: neighbor_sockets.iter().cloned().collect(),
      });
    }

    cell.entropy = cell.possibilities.len();

    Ok(())
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> From<State<T, DIM>> for Vec<T::Variant> {
  fn from(value: State<T, DIM>) -> Self {
    value
      .cells
      .list
      .into_iter()
      .map(|cell| cell.possibilities.into_iter().next().unwrap())
      .collect()
  }
}

type InnerSocketCache<T, const DIM: usize> = HashMap<
  BTreeSet<<T as TypeAtlas<DIM>>::Variant>,
  HashMap<<T as TypeAtlas<DIM>>::Dimension, HashSet<<T as TypeAtlas<DIM>>::Socket>>,
>;

#[derive(Debug, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
struct SocketCache<T: TypeAtlas<DIM>, const DIM: usize>(InnerSocketCache<T, DIM>);

impl<T: TypeAtlas<DIM>, const DIM: usize> Default for SocketCache<T, DIM> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> SocketCache<T, DIM> {
  fn lookup(
    &mut self,
    variants: &BTreeSet<T::Variant>,
    dir: &T::Dimension,
  ) -> Option<Option<&HashSet<T::Socket>>> {
    self.get(variants).map(|dirmap| dirmap.get(dir))
  }

  fn full_create(
    &mut self,
    rules: &Rules<T::Variant, T::Dimension, T::Socket>,
    variants: &BTreeSet<T::Variant>,
    dir: &T::Dimension,
  ) -> &HashSet<T::Socket> {
    let dir_map = self.entry(variants.clone()).or_default();
    dir_map.entry(*dir).or_insert_with(|| {
      variants
        .iter()
        .flat_map(|id| rules.get(id).and_then(|rule| rule.get(dir)))
        .cloned()
        .collect::<HashSet<T::Socket>>()
    })
  }

  fn partial_create(
    &mut self,
    rules: &Rules<T::Variant, T::Dimension, T::Socket>,
    variants: &BTreeSet<T::Variant>,
    dir: &T::Dimension,
  ) -> &HashSet<T::Socket> {
    let dir_map = self.get_mut(variants).unwrap();
    dir_map.entry(*dir).or_insert_with(|| {
      variants
        .iter()
        .flat_map(|id| rules.get(id).and_then(|rule| rule.get(dir)))
        .cloned()
        .collect::<HashSet<T::Socket>>()
    })
  }
}

#[derive(Debug, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
struct ExtCells<T: TypeAtlas<DIM>, const DIM: usize> {
  size: Size<DIM>,
  #[deref]
  #[deref_mut]
  sides: HashMap<T::Dimension, Vec<T::Variant>>,
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Clone for ExtCells<T, DIM> {
  fn clone(&self) -> Self {
    Self {
      size: self.size,
      sides: self.sides.clone(),
    }
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> ExtCells<T, DIM> {
  fn new(size: Size<DIM>) -> Self {
    Self {
      size,
      sides: Default::default(),
    }
  }
}
