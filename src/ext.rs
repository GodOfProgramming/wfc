use crate::TypeAtlas;

pub trait TypeAtlasExt<const DIM: usize> {
  const DIM: usize;
}

impl<T: TypeAtlas<DIM>, const DIM: usize> TypeAtlasExt<DIM> for T {
  const DIM: usize = DIM;
}

#[cfg(not(feature = "serde"))]
pub trait MaybeSerde {}

#[cfg(not(feature = "serde"))]
impl<T> MaybeSerde for T {}

#[cfg(feature = "serde")]
pub trait MaybeSerde: serde::Serialize + for<'d> serde::Deserialize<'d> {}

#[cfg(feature = "serde")]
impl<T> MaybeSerde for T where T: serde::Serialize + for<'d> serde::Deserialize<'d> {}
