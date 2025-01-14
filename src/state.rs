use crate::{
  cells::{Cell, Cells},
  err,
  util::{self, Size, UPos},
  Arbiter, Constraint, Dimension, Error, Observation, Rules, Socket, Variant,
};
use derive_more::derive::{Deref, DerefMut};
use std::{
  collections::{BTreeSet, HashMap, HashSet},
  fmt::Debug,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct StateBuilder<A, C, V, D, S, const DIM: usize>
where
  A: Arbiter<V>,
  C: Constraint<S>,
  V: Variant,
  D: Dimension,
  S: Socket,
{
  size: Size<DIM>,
  arbiter: A,
  constraint: C,
  rules: Rules<V, D, S>,
  output_buffer: Vec<Option<V>>,
  external_cells: ExtCells<V, D, DIM>,
}

impl<A, C, V, D, S, const DIM: usize> StateBuilder<A, C, V, D, S, DIM>
where
  A: Arbiter<V>,
  C: Constraint<S>,
  V: Variant,
  D: Dimension,
  S: Socket,
{
  pub fn new(
    size: impl Into<Size<DIM>>,
    arbiter: A,
    constraint: C,
    rules: impl Into<Rules<V, D, S>>,
  ) -> Self {
    let size = size.into();
    Self {
      size,
      arbiter,
      constraint,
      rules: rules.into(),
      output_buffer: vec![None; size.len()],
      external_cells: ExtCells::new(size),
    }
  }

  pub fn with_ext(&mut self, dir: D, source: Vec<V>) -> &mut Self {
    self.external_cells.insert(dir, source);
    self
  }

  pub fn insert(&mut self, pos: impl Into<UPos<DIM>>, value: V) -> &mut Self {
    let pos = pos.into();
    self.output_buffer[pos.index(self.size)] = Some(value);
    self
  }

  pub fn size(&self) -> &Size<DIM> {
    &self.size
  }

  pub fn build(self) -> Result<State<A, C, V, D, S, DIM>, err::Error<DIM>> {
    // seemingly cannot be done at compile time because
    // M::Dimensions::COUNT is not accessible inside static asserts
    if DIM != D::COUNT / 2 {
      return Err(Error::DimensionMismatch {
        const_value: DIM,
        dimension_count: D::COUNT,
      });
    }

    State::new(
      self.size,
      self.arbiter,
      self.constraint,
      self.rules,
      self.output_buffer,
      self.external_cells,
    )
  }
}

impl<A, C, V, D, S, const DIM: usize> Clone for StateBuilder<A, C, V, D, S, DIM>
where
  A: Arbiter<V> + Clone,
  C: Constraint<S> + Clone,
  V: Variant,
  D: Dimension,
  S: Socket,
{
  fn clone(&self) -> Self {
    Self {
      arbiter: self.arbiter.clone(),
      constraint: self.constraint.clone(),
      size: self.size,
      output_buffer: self.output_buffer.clone(),
      rules: self.rules.clone(),
      external_cells: self.external_cells.clone(),
    }
  }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub struct State<A, C, V, D, S, const DIM: usize>
where
  A: Arbiter<V>,
  C: Constraint<S>,
  V: Variant,
  D: Dimension,
  S: Socket,
{
  cells: Cells<V, D, DIM>,
  arbiter: A,
  constraint: C,
  rules: Rules<V, D, S>,
  socket_cache: SocketCache<V, D, S>,
}

impl<A, C, V, D, S, const DIM: usize> State<A, C, V, D, S, DIM>
where
  A: Arbiter<V>,
  C: Constraint<S>,
  V: Variant,
  D: Dimension,
  S: Socket,
{
  /// Creates a new instance of a State, with initial setup
  #[profiling::function]
  fn new(
    size: Size<DIM>,
    arbiter: A,
    constraint: C,
    rules: Rules<V, D, S>,
    input: Vec<Option<V>>,
    external_cells: ExtCells<V, D, DIM>,
  ) -> Result<Self, err::Error<DIM>> {
    // create the state
    let mut this = Self {
      cells: Cells::new(size, input, &rules),
      rules,
      arbiter,
      constraint,
      socket_cache: Default::default(),
    };

    this.apply_external_information(external_cells)?;

    this.apply_predetermined_cells()?;

    Ok(this)
  }

  #[profiling::function]
  pub fn collapse(&mut self) -> Result<Observation, err::Error<DIM>> {
    let Some(index) = self.arbiter.designate(&mut self.cells)? else {
      return Ok(Observation::Complete);
    };

    let cell = &self.cells.list[index];
    let possibility = cell.selected_variant().cloned().unwrap();

    self.arbiter.revise(&possibility, &mut self.cells);
    self.propagate(index)?;

    Ok(Observation::Incomplete(index))
  }

  /// propagate the information of the supplied cell to its neighbors, and repeat until there are no more constraints made
  #[profiling::function]
  fn propagate(&mut self, cell_index: usize) -> Result<(), err::Error<DIM>> {
    let mut stack = Vec::with_capacity(D::COUNT);
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

  pub fn data(&self) -> Vec<V>
  where
    V: Default,
  {
    self
      .cells
      .list
      .iter()
      .map(|cell| cell.selected_variant().cloned().unwrap_or_default())
      .collect()
  }

  pub fn data_raw(&self) -> Vec<Option<V>> {
    self
      .cells
      .list
      .iter()
      .map(|cell| cell.selected_variant().cloned())
      .collect()
  }

  pub fn cells(&self) -> &Cells<V, D, DIM> {
    &self.cells
  }

  pub fn size(&self) -> &Size<DIM> {
    &self.cells.size
  }

  pub fn rules(&self) -> &Rules<V, D, S> {
    &self.rules
  }

  pub fn constrainer(&self) -> &C {
    &self.constraint
  }

  /// Tries to reduce the number of possibilities this tile can be based on a neighbor
  /// Returns true if at least one reduction was made, false otherwise
  #[profiling::function]
  fn constrain(
    cell: &mut Cell<V, D, DIM>,
    constraint: &C,
    neighbor_possibilities: &BTreeSet<V>,
    neighbor_to_self_dir: D,
    rules: &Rules<V, D, S>,
    cache: &mut SocketCache<V, D, S>,
  ) -> Result<(), err::Error<DIM>> {
    let neighbor_sockets = {
      profiling::function_scope!("Neighbor Sockets");

      match cache.lookup(neighbor_possibilities, &neighbor_to_self_dir) {
        Some(Some(sockets)) => sockets,
        Some(None) => cache.partial_create(rules, neighbor_possibilities, neighbor_to_self_dir),
        None => cache.full_create(rules, neighbor_possibilities, neighbor_to_self_dir),
      }
    };

    let opposite = neighbor_to_self_dir.opposite();

    cell.possibilities.retain(|variant| {
      let Some(self_rule) = rules.rule_for(variant) else {
        return false;
      };

      self_rule
        .socket_for(&opposite)
        .map(|socket| constraint.check(socket, neighbor_sockets))
        .unwrap_or(false) // no rule means no connection
    });

    if cell.possibilities.is_empty() {
      return Err(Error::Contradiction {
        position: cell.position,
        neighbor: cell.position + neighbor_to_self_dir.opposite(),
      });
    }

    cell.entropy = cell.possibilities.len();

    Ok(())
  }

  /// Propagates information to cells if there is a generation on a neighboring side
  fn apply_external_information(
    &mut self,
    external_cells: ExtCells<V, D, DIM>,
  ) -> Result<(), err::Error<DIM>> {
    for (dir, ext) in external_cells.sides.into_iter() {
      let indexes = self.cells.uncollapsed_indexes_along_dir(dir);
      for index in indexes {
        let cell = self.cells.at_mut(index);
        let neighbor = cell.position + dir;
        let neighbor_index = neighbor.index_in(external_cells.size);
        let external_variant = BTreeSet::from_iter([ext[neighbor_index].clone()]);

        let starting_entropy = cell.entropy;
        Self::constrain(
          cell,
          &self.constraint,
          &external_variant,
          dir.opposite(),
          &self.rules,
          &mut self.socket_cache,
        )?;
        let new_entropy = cell.entropy;

        if starting_entropy != new_entropy {
          self.cells.set_entropy(starting_entropy, index, new_entropy);
        }

        self.propagate(index)?;
      }
    }

    Ok(())
  }

  /// For any cells that are collapsed, propagate that information
  fn apply_predetermined_cells(&mut self) -> Result<(), err::Error<DIM>> {
    let propagations = self
      .cells
      .list
      .iter()
      .enumerate()
      .filter_map(|(i, cell)| cell.selected_variant().cloned().map(|variant| (i, variant)))
      .collect::<Vec<_>>();

    for (i, variant) in propagations {
      self.arbiter.revise(&variant, &mut self.cells);
      self.propagate(i)?;
    }

    Ok(())
  }
}

impl<A, C, V, D, S, const DIM: usize> From<State<A, C, V, D, S, DIM>> for Vec<V>
where
  A: Arbiter<V>,
  C: Constraint<S>,
  V: Variant,
  D: Dimension,
  S: Socket,
{
  fn from(state: State<A, C, V, D, S, DIM>) -> Self {
    state
      .cells
      .list
      .into_iter()
      .map(|cell| cell.possibilities.into_iter().next().unwrap())
      .collect()
  }
}

type InnerSocketCache<V, D, S> = HashMap<BTreeSet<V>, HashMap<D, HashSet<S>>>;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
struct SocketCache<V: Variant, D: Dimension, S: Socket>(InnerSocketCache<V, D, S>);

impl<V: Variant, D: Dimension, S: Socket> Default for SocketCache<V, D, S> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<V: Variant, D: Dimension, S: Socket> SocketCache<V, D, S> {
  fn lookup(&mut self, variants: &BTreeSet<V>, dir: &D) -> Option<Option<&HashSet<S>>> {
    self.0.get(variants).map(|dirmap| dirmap.get(dir))
  }

  fn full_create(&mut self, rules: &Rules<V, D, S>, variants: &BTreeSet<V>, dir: D) -> &HashSet<S> {
    let dir_map = self.0.entry(variants.clone()).or_default();
    dir_map.entry(dir).or_insert_with(|| {
      variants
        .iter()
        .flat_map(|id| {
          rules
            .rule_for(id)
            .and_then(|rule| rule.socket_for(&dir).cloned())
        })
        .collect::<HashSet<S>>()
    })
  }

  fn partial_create(
    &mut self,
    rules: &Rules<V, D, S>,
    variants: &BTreeSet<V>,
    dir: D,
  ) -> &HashSet<S> {
    let dir_map = self.0.get_mut(variants).unwrap();
    dir_map.entry(dir).or_insert_with(|| {
      variants
        .iter()
        .flat_map(|id| {
          rules
            .rule_for(id)
            .and_then(|rule| rule.socket_for(&dir).cloned())
        })
        .collect::<HashSet<S>>()
    })
  }
}

#[derive(Debug, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
struct ExtCells<V, D, const DIM: usize>
where
  D: Dimension,
{
  size: Size<DIM>,
  #[deref]
  #[deref_mut]
  sides: HashMap<D, Vec<V>>,
}

impl<V, D, const DIM: usize> Clone for ExtCells<V, D, DIM>
where
  V: Clone,
  D: Dimension,
{
  fn clone(&self) -> Self {
    Self {
      size: self.size,
      sides: self.sides.clone(),
    }
  }
}

impl<V, D, const DIM: usize> ExtCells<V, D, DIM>
where
  D: Dimension,
{
  fn new(size: Size<DIM>) -> Self {
    Self {
      size,
      sides: Default::default(),
    }
  }
}
