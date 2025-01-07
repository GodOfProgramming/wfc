use crate::Weight;

#[derive(Default, Debug, Clone)]
pub struct NoWeight;

impl Weight for NoWeight {
  type ValueType = usize;
  fn value(&self) -> Self::ValueType {
    1 // so that every possibility has the same weight, thus none
  }
}

#[derive(Debug, Clone)]
pub struct DirectWeight(pub usize);

impl Default for DirectWeight {
  fn default() -> Self {
    Self(1)
  }
}

impl Weight for DirectWeight {
  type ValueType = usize;
  fn value(&self) -> Self::ValueType {
    self.0
  }
}
