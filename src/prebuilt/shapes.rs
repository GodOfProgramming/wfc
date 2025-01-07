use crate::{
  cells::{Cell, Cells},
  IPos, Shape, TypeAtlas, Weight,
};
use derive_more::derive::{Deref, DerefMut};
use derive_new::new;
use std::{collections::HashMap, marker::PhantomData, ops::RangeInclusive};

#[derive(Debug)]
pub struct NoShape;

impl<T: TypeAtlas<DIM>, const DIM: usize> Shape<T, DIM> for NoShape {
  fn weight(
    &self,
    _variant: &T::Variant,
    _index: usize,
    _cells: &Cells<T, DIM>,
  ) -> <T::Weight as Weight>::ValueType {
    T::Weight::default().value()
  }
}

#[derive(Debug, new, Deref, DerefMut)]
pub struct WeightedShape<T: TypeAtlas<DIM>, const DIM: usize>(HashMap<T::Variant, T::Weight>);

impl<T: TypeAtlas<DIM>, const DIM: usize> Default for WeightedShape<T, DIM> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Clone for WeightedShape<T, DIM> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Shape<T, DIM> for WeightedShape<T, DIM> {
  fn weight(
    &self,
    variant: &T::Variant,
    _index: usize,
    _cells: &Cells<T, DIM>,
  ) -> <T::Weight as Weight>::ValueType {
    self
      .get(variant)
      .map(|w| w.value())
      .unwrap_or_else(|| T::Weight::default().value())
  }
}

#[derive(Debug)]
pub struct InformedShape<T: TypeAtlas<DIM>, const DIM: usize> {
  range: f64,
  magnitude: <T::Weight as Weight>::ValueType,
  values: HashMap<T::Variant, T::Weight>,

  estimated_neighbors: usize,
  _pd: PhantomData<T>,
}

impl<T: TypeAtlas<DIM>, const DIM: usize> Clone for InformedShape<T, DIM> {
  fn clone(&self) -> Self {
    Self {
      range: self.range,
      magnitude: self.magnitude.clone(),
      values: self.values.clone(),

      estimated_neighbors: self.estimated_neighbors,
      _pd: PhantomData,
    }
  }
}

impl<T: TypeAtlas<DIM>, const DIM: usize> InformedShape<T, DIM> {
  pub fn new(
    range: f64,
    magnitude: <T::Weight as Weight>::ValueType,
    values: HashMap<T::Variant, T::Weight>,
  ) -> Self {
    Self {
      range,
      magnitude,
      values,

      estimated_neighbors: (0..range as usize).map(|n| (n + 1).pow(2)).sum(),
      _pd: PhantomData,
    }
  }

  #[profiling::function]
  pub fn collapsed_neighbors<'v>(
    &self,
    cell: &'v Cell<T::Variant, T::Dimension, DIM>,
    cells: &'v Cells<T, DIM>,
  ) -> Vec<(&'v T::Variant, f64)> {
    let start = cell.position;

    let mut neighbors = Vec::with_capacity(self.estimated_neighbors);

    let whole_num_range = self.range as isize;

    let iterations: [RangeInclusive<isize>; DIM] =
      std::array::from_fn(|_| -whole_num_range..=whole_num_range);

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
  fn get_all_neighbors<'v>(
    &self,
    cells: &'v Cells<T, DIM>,
    neighbors: &mut Vec<(&'v T::Variant, f64)>,
    start: &IPos<DIM>,
    current_offset: &mut IPos<DIM>,
    depth: usize,
    iters: &[RangeInclusive<isize>; DIM],
  ) {
    if let Some(iter) = iters.get(depth) {
      for i in iter.clone().into_iter() {
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

impl<T: TypeAtlas<DIM>, const DIM: usize> Shape<T, DIM> for InformedShape<T, DIM> {
  fn weight(
    &self,
    variant: &T::Variant,
    index: usize,
    cells: &Cells<T, DIM>,
  ) -> <T::Weight as Weight>::ValueType {
    let neighbors = self.collapsed_neighbors(cells.at(index), cells);
    neighbors
      .iter()
      .filter(|(v, _)| variant == *v)
      .filter_map(|(v, _d)| self.values.get(v).map(|w| w.value() * self.magnitude))
      .sum()
  }
}

#[derive(Debug)]
pub struct MultiShape<S1, S2, T, const DIM: usize>
where
  S1: Shape<T, DIM>,
  S2: Shape<T, DIM>,
  T: TypeAtlas<DIM>,
{
  shape1: S1,
  shape2: S2,
  _pd: PhantomData<T>,
}

impl<S1, S2, T, const DIM: usize> MultiShape<S1, S2, T, DIM>
where
  S1: Shape<T, DIM>,
  S2: Shape<T, DIM>,
  T: TypeAtlas<DIM>,
{
  pub fn new(shape1: impl Into<S1>, shape2: impl Into<S2>) -> Self {
    Self {
      shape1: shape1.into(),
      shape2: shape2.into(),
      _pd: PhantomData,
    }
  }
}

impl<S1, S2, T, const DIM: usize> Shape<T, DIM> for MultiShape<S1, S2, T, DIM>
where
  S1: Shape<T, DIM>,
  S2: Shape<T, DIM>,
  T: TypeAtlas<DIM>,
{
  fn weight(
    &self,
    variant: &T::Variant,
    index: usize,
    cells: &Cells<T, DIM>,
  ) -> <T::Weight as Weight>::ValueType {
    self.shape1.weight(variant, index, cells) + self.shape2.weight(variant, index, cells)
  }
}
