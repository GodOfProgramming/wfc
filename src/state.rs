use crate::{
  cells::{Cell, Cells},
  err,
  rules::AbstractRules,
  util::{self, Size, UPos},
  Arbiter, Constraint, DimensionId, Error, Observation, Rules, SocketId, TypeAtlas, VariantId,
};
use derive_more::derive::{Deref, DerefMut};
use std::{
  collections::{BTreeSet, HashMap, HashSet},
  fmt::Debug,
};
use strum::EnumCount;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct StateBuilder<A, C, T: TypeAtlas<DIM>, const DIM: usize>
where
  A: Arbiter,
  C: Constraint,
{
  arbiter: A,
  constraint: C,
  rules: Rules<T::Variant, T::Dimension, T::Socket>,
  size: Size<DIM>,
  input: Vec<Option<T::Variant>>,
  external_cells: ExtCells<T, DIM>,
}

impl<A, C, T: TypeAtlas<DIM>, const DIM: usize> StateBuilder<A, C, T, DIM>
where
  A: Arbiter,
  C: Constraint,
{
  pub fn new(
    size: impl Into<Size<DIM>>,
    arbiter: impl Into<A>,
    constraint: impl Into<C>,
    rules: Rules<T::Variant, T::Dimension, T::Socket>,
  ) -> Self {
    let size = size.into();
    Self {
      size,
      arbiter: arbiter.into(),
      constraint: constraint.into(),
      rules,
      input: vec![None; size.len()],
      external_cells: ExtCells::new(size),
    }
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

  pub fn build(self) -> Result<State<A, C, T, DIM>, err::Error<DIM>> {
    // seemingly cannot be done at compile time because
    // M::Dimensions::COUNT is not accessible inside static asserts
    if DIM != T::Dimension::COUNT / 2 {
      return Err(Error::DimensionMismatch {
        const_value: DIM,
        dimension_count: T::Dimension::COUNT,
      });
    }

    State::new(
      self.size,
      self.input,
      self.rules,
      self.arbiter,
      self.constraint,
      self.external_cells,
    )
  }
}

impl<A, C, T: TypeAtlas<DIM>, const DIM: usize> Clone for StateBuilder<A, C, T, DIM>
where
  A: Arbiter + Clone,
  C: Constraint + Clone,
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
pub struct State<A, C, T: TypeAtlas<DIM>, const DIM: usize>
where
  A: Arbiter,
  C: Constraint,
{
  cells: Cells<DIM>,
  rules: Rules<T::Variant, T::Dimension, T::Socket>,
  arbiter: A,
  constraint: C,
  socket_cache: SocketCache,
}

impl<A, C, T: TypeAtlas<DIM>, const DIM: usize> State<A, C, T, DIM>
where
  A: Arbiter,
  C: Constraint,
{
  #[profiling::function]
  fn new(
    size: Size<DIM>,
    input: Vec<Option<T::Variant>>,
    rules: Rules<T::Variant, T::Dimension, T::Socket>,
    arbiter: A,
    constraint: C,
    external_cells: ExtCells<T, DIM>,
  ) -> Result<Self, err::Error<DIM>> {
    let mut this = Self {
      cells: Cells::new(size, input, rules.abstract_rules(), rules.legend()),
      rules,
      arbiter,
      constraint,
      socket_cache: Default::default(),
    };

    for (dir, ext) in external_cells.sides.into_iter() {
      let dir = this.rules.legend().dimension_id(&dir);
      let indexes = this.cells.uncollapsed_indexes_along_dir(dir);
      for index in indexes {
        let cell = this.cells.at_mut(index);
        let neighbor = cell.position + dir;
        let neighbor_index = neighbor.index_in(external_cells.size);
        let external_variant =
          BTreeSet::from_iter([this.rules.legend().variant_id(&ext[neighbor_index])]);

        let starting_entropy = cell.entropy;
        Self::constrain(
          cell,
          &this.constraint,
          &external_variant,
          dir.opposite(),
          this.rules.abstract_rules(),
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
      .filter_map(|(i, cell)| cell.selected_variant().map(|variant| (i, variant)))
      .collect::<Vec<_>>();

    for (i, variant) in propagations {
      this.arbiter.revise(variant, &mut this.cells);
      this.propagate(i)?;
    }

    Ok(this)
  }

  #[profiling::function]
  pub fn collapse(&mut self) -> Result<Observation, err::Error<DIM>> {
    let Some(index) = self.arbiter.designate(&mut self.cells)? else {
      return Ok(Observation::Complete);
    };

    let cell = &self.cells.list[index];
    let possibility = cell.selected_variant().unwrap();

    self.arbiter.revise(possibility, &mut self.cells);
    self.propagate(index)?;

    Ok(Observation::Incomplete(index))
  }

  #[profiling::function]
  fn propagate(&mut self, cell_index: usize) -> Result<(), err::Error<DIM>> {
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
          direction,
          self.rules.abstract_rules(),
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
          .map(|variant_id| self.rules.legend().variant(variant_id))
          .unwrap_or_default()
      })
      .collect()
  }

  pub fn data_raw(&self) -> Vec<Option<T::Variant>> {
    self
      .cells
      .list
      .iter()
      .map(|cell| {
        cell
          .selected_variant()
          .map(|variant_id| self.rules.legend().variant(variant_id))
      })
      .collect()
  }

  pub fn cells(&self) -> &Cells<DIM> {
    &self.cells
  }

  pub fn size(&self) -> &Size<DIM> {
    &self.cells.size
  }

  pub fn rules(&self) -> &Rules<T::Variant, T::Dimension, T::Socket> {
    &self.rules
  }

  pub fn constrainer(&self) -> &C {
    &self.constraint
  }

  /// Tries to reduce the number of possibilities this tile can be based on a neighbor
  /// Returns true if at least one reduction was made, false otherwise
  #[profiling::function]
  fn constrain(
    cell: &mut Cell<DIM>,
    constraint: &C,
    neighbor_possibilities: &BTreeSet<VariantId>,
    neighbor_to_self_dir: DimensionId,
    rules: &AbstractRules,
    cache: &mut SocketCache,
  ) -> Result<(), err::Error<DIM>> {
    let neighbor_sockets: &HashSet<SocketId> = {
      profiling::function_scope!("Neighbor Sockets");

      match cache.lookup(neighbor_possibilities, neighbor_to_self_dir) {
        Some(Some(sockets)) => sockets,
        Some(None) => cache.partial_create(rules, neighbor_possibilities, neighbor_to_self_dir),
        None => cache.full_create(rules, neighbor_possibilities, neighbor_to_self_dir),
      }
    };

    let opposite = neighbor_to_self_dir.opposite();

    cell.possibilities.retain(|variant| {
      let Some(self_rule) = rules.rule_for(*variant) else {
        return false;
      };

      self_rule
        .socket_for(opposite)
        .map(|socket| constraint.check(socket, neighbor_sockets))
        .unwrap_or(false) // no rule means no connection
    });

    if cell.possibilities.is_empty() {
      return Err(Error::Contradiction {
        position: cell.position,
        neighbor: cell.position + neighbor_to_self_dir.opposite(),
        direction: opposite,
        neighbor_variants: Vec::from_iter(neighbor_possibilities.iter().cloned()),
        neighbor_sockets: neighbor_sockets.iter().cloned().collect(),
      });
    }

    cell.entropy = cell.possibilities.len();

    Ok(())
  }
}

impl<A, C, T: TypeAtlas<DIM>, const DIM: usize> From<State<A, C, T, DIM>> for Vec<T::Variant>
where
  A: Arbiter,
  C: Constraint,
{
  fn from(state: State<A, C, T, DIM>) -> Self {
    state
      .cells
      .list
      .into_iter()
      .map(|cell| {
        state
          .rules
          .legend()
          .variant(cell.possibilities.into_iter().next().unwrap())
      })
      .collect()
  }
}

type InnerSocketCache = HashMap<BTreeSet<VariantId>, HashMap<DimensionId, HashSet<SocketId>>>;

#[derive(Default, Debug, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
struct SocketCache(InnerSocketCache);

impl SocketCache {
  fn lookup(
    &mut self,
    variants: &BTreeSet<VariantId>,
    dir: DimensionId,
  ) -> Option<Option<&HashSet<SocketId>>> {
    self.get(variants).map(|dirmap| dirmap.get(&dir))
  }

  fn full_create(
    &mut self,
    rules: &AbstractRules,
    variants: &BTreeSet<VariantId>,
    dir: DimensionId,
  ) -> &HashSet<SocketId> {
    let dir_map = self.entry(variants.clone()).or_default();
    dir_map.entry(dir).or_insert_with(|| {
      variants
        .iter()
        .flat_map(|id| rules.rule_for(*id).and_then(|rule| rule.socket_for(dir)))
        .collect::<HashSet<SocketId>>()
    })
  }

  fn partial_create(
    &mut self,
    rules: &AbstractRules,
    variants: &BTreeSet<VariantId>,
    dir: DimensionId,
  ) -> &HashSet<SocketId> {
    let dir_map = self.get_mut(variants).unwrap();
    dir_map.entry(dir).or_insert_with(|| {
      variants
        .iter()
        .flat_map(|id| rules.rule_for(*id).and_then(|rule| rule.socket_for(dir)))
        .collect::<HashSet<SocketId>>()
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
