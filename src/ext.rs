#[cfg(not(feature = "serde"))]
pub trait MaybeSerde {}

#[cfg(not(feature = "serde"))]
impl<T> MaybeSerde for T {}

#[cfg(feature = "serde")]
pub trait MaybeSerde: serde::Serialize + for<'de> serde::Deserialize<'de> {}

#[cfg(feature = "serde")]
impl<T> MaybeSerde for T where T: serde::Serialize + for<'de> serde::Deserialize<'de> {}
