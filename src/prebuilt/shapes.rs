use crate::{
  cells::{Cell, Cells},
  CellIndex, Dimension, IPos, Rules, Shape, Socket, Variant, VariantId, Weight,
};
use derive_more::derive::{Deref, DerefMut};
use std::{borrow::Borrow, collections::HashMap, ops::Range};

#[derive(Debug, Deref, DerefMut)]
pub struct WeightedShape<W: Weight>(HashMap<VariantId, W>);

impl<W: Weight> WeightedShape<W> {
  pub fn new<V: Variant, D: Dimension, S: Socket>(
    weights: impl IntoIterator<Item = (V, W)>,
    rules: impl Borrow<Rules<V, D, S>>,
  ) -> Self {
    Self(
      weights
        .into_iter()
        .map(|(variant, weight)| (rules.borrow().legend().variant_id(&variant), weight))
        .collect(),
    )
  }
}

impl<W: Weight> Clone for WeightedShape<W> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<W: Weight> Shape for WeightedShape<W> {
  type Weight = W;
  fn weight<const DIM: usize>(
    &self,
    variant: VariantId,
    _index: usize,
    _cells: &Cells<DIM>,
  ) -> Self::Weight {
    self
      .get(&variant)
      .cloned()
      .unwrap_or_else(|| Self::Weight::default())
  }
}

#[derive(Debug)]
pub struct InformedShape<W: Weight> {
  range: f64,
  magnitude: W,
  values: HashMap<VariantId, W>,

  estimated_neighbors: usize,
}

impl<W: Weight> Clone for InformedShape<W> {
  fn clone(&self) -> Self {
    Self {
      range: self.range,
      magnitude: self.magnitude,
      values: self.values.clone(),

      estimated_neighbors: self.estimated_neighbors,
    }
  }
}

impl<W: Weight> InformedShape<W> {
  pub fn new(range: f64, magnitude: W, values: HashMap<VariantId, W>) -> Self {
    Self {
      range,
      magnitude,
      values,

      estimated_neighbors: (0..range as usize).map(|n| (n + 1).pow(2)).sum(),
    }
  }

  #[profiling::function]
  pub fn collapsed_neighbors<const DIM: usize>(
    &self,
    cell: &Cell<DIM>,
    cells: &Cells<DIM>,
  ) -> Vec<(VariantId, f64)> {
    let start = cell.position;

    let mut neighbors = Vec::with_capacity(self.estimated_neighbors);

    let whole_num_range = self.range as isize;

    let iterations: [Range<isize>; DIM] =
      std::array::from_fn(|_| -whole_num_range..whole_num_range + 1);

    let mut current_offset = IPos::default();

    self.get_all_neighbors(
      cells,
      &mut neighbors,
      &start,
      &mut current_offset,
      0,
      &iterations,
    );

    neighbors
  }

  #[profiling::function]
  fn get_all_neighbors<const DIM: usize>(
    &self,
    cells: &Cells<DIM>,
    neighbors: &mut Vec<(VariantId, f64)>,
    start: &IPos<DIM>,
    current_offset: &mut IPos<DIM>,
    depth: usize,
    iters: &[Range<isize>; DIM],
  ) {
    if let Some(iter) = iters.get(depth) {
      for i in iter.clone() {
        current_offset[depth] = i;
        self.get_all_neighbors(cells, neighbors, start, current_offset, depth + 1, iters);
      }
    } else {
      let neighbor = IPos::from(**start + **current_offset);
      let fstart = start.map(|i| i as f64);
      let fneighbor = neighbor.map(|i| i as f64);
      let distance = fstart.metric_distance(&fneighbor);
      if !cells.size.contains(current_offset) || distance > self.range {
        return;
      }

      if let Some(n) = cells.at_pos(&neighbor).and_then(|n| n.selected_variant()) {
        neighbors.push((n, distance))
      }
    }
  }
}

impl<W: Weight> Shape for InformedShape<W> {
  type Weight = W;
  fn weight<const DIM: usize>(
    &self,
    variant: VariantId,
    index: usize,
    cells: &Cells<DIM>,
  ) -> Self::Weight {
    let neighbors = self.collapsed_neighbors(cells.at(index), cells);
    neighbors
      .iter()
      .filter(|(v, _)| variant == *v)
      .filter_map(|(v, _d)| self.values.get(v).map(|w| *w * self.magnitude))
      .sum()
  }
}

#[derive(Debug)]
pub struct MultiShape<S1, S2>
where
  S1: Shape,
  S2: Shape,
{
  shape1: S1,
  shape2: S2,
}

impl<S1, S2> MultiShape<S1, S2>
where
  S1: Shape,
  S2: Shape,
{
  pub fn new(shape1: impl Into<S1>, shape2: impl Into<S2>) -> Self {
    Self {
      shape1: shape1.into(),
      shape2: shape2.into(),
    }
  }
}

impl<S1, S2> Shape for MultiShape<S1, S2>
where
  S1: Shape,
  S2: Shape<Weight = S1::Weight>,
{
  type Weight = S1::Weight;
  fn weight<const DIM: usize>(
    &self,
    variant: VariantId,
    index: CellIndex,
    cells: &Cells<DIM>,
  ) -> Self::Weight {
    self.shape1.weight(variant, index, cells) + self.shape2.weight(variant, index, cells)
  }
}
