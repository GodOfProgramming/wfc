use crate::{
  cells::{Cell, Cells},
  CellIndex, Dimension, IPos, Shape, Variant, Weight,
};
use derive_more::derive::{Deref, DerefMut};
use derive_new::new;
use std::{collections::HashMap, ops::Range};

#[derive(Debug, Deref, DerefMut)]
pub struct WeightedShape<V: Variant, W: Weight>(HashMap<V, W>);

impl<V: Variant, W: Weight> WeightedShape<V, W> {
  pub fn new(weights: impl Into<HashMap<V, W>>) -> Self {
    Self(weights.into())
  }
}

impl<V: Variant, W: Weight> Clone for WeightedShape<V, W> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<V: Variant, W: Weight> Shape for WeightedShape<V, W> {
  type Variant = V;
  type Weight = W;
  fn weight<D: Dimension, const DIM: usize>(
    &self,
    variant: &Self::Variant,
    _index: usize,
    _cells: &Cells<Self::Variant, D, DIM>,
  ) -> Self::Weight {
    self
      .get(&variant)
      .cloned()
      .unwrap_or_else(|| Self::Weight::default())
  }
}

#[derive(Debug)]
pub struct InformedShape<V: Variant, W: Weight> {
  range: f64,
  magnitude: W,
  values: HashMap<V, W>,

  estimated_neighbors: usize,
}

impl<V: Variant, W: Weight> Clone for InformedShape<V, W> {
  fn clone(&self) -> Self {
    Self {
      range: self.range,
      magnitude: self.magnitude,
      values: self.values.clone(),

      estimated_neighbors: self.estimated_neighbors,
    }
  }
}

impl<V: Variant, W: Weight> InformedShape<V, W> {
  pub fn new(range: f64, magnitude: W, values: impl Into<HashMap<V, W>>) -> Self {
    Self {
      range,
      magnitude,
      values: values.into(),

      estimated_neighbors: (0..range as usize).map(|n| (n + 1).pow(2)).sum(),
    }
  }

  #[profiling::function]
  pub fn collapsed_neighbors<'c, D: Dimension, const DIM: usize>(
    &self,
    cell: &Cell<V, D, DIM>,
    cells: &'c Cells<V, D, DIM>,
  ) -> Vec<(&'c V, f64)> {
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
  fn get_all_neighbors<'c, D: Dimension, const DIM: usize>(
    &self,
    cells: &'c Cells<V, D, DIM>,
    neighbors: &mut Vec<(&'c V, f64)>,
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

impl<V: Variant, W: Weight> Shape for InformedShape<V, W> {
  type Variant = V;
  type Weight = W;
  fn weight<D: Dimension, const DIM: usize>(
    &self,
    variant: &Self::Variant,
    index: usize,
    cells: &Cells<Self::Variant, D, DIM>,
  ) -> Self::Weight {
    let neighbors = self.collapsed_neighbors(cells.at(index), cells);
    neighbors
      .iter()
      .filter(|(v, _)| variant == *v)
      .filter_map(|(v, _d)| self.values.get(v).map(|w| *w * self.magnitude))
      .sum()
  }
}

#[derive(new, Debug)]
pub struct MultiShape<S1, S2>
where
  S1: Shape,
  S2: Shape,
{
  shape1: S1,
  shape2: S2,
}

impl<S1, S2> Shape for MultiShape<S1, S2>
where
  S1: Shape,
  S2: Shape<Variant = S1::Variant, Weight = S1::Weight>,
{
  type Variant = S1::Variant;
  type Weight = S1::Weight;
  fn weight<D: Dimension, const DIM: usize>(
    &self,
    variant: &Self::Variant,
    index: CellIndex,
    cells: &Cells<Self::Variant, D, DIM>,
  ) -> Self::Weight {
    self.shape1.weight(variant, index, cells) + self.shape2.weight(variant, index, cells)
  }
}
