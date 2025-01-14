use crate::prebuilt::Dim2d;
use crate::{FindResult, NoSocket, SocketProvider, Variant};
use maplit::hashmap;
use std::marker::PhantomData;
use std::{collections::HashMap, hash::Hash};

pub trait Maze2dTypeSet<V: Variant> {
  const ENTRANCE: V;
  const EXIT: V;
  const EMPTY: V;
  const VERTICAL: V;
  const HORIZONTAL: V;
  const CORNER_UL: V;
  const CORNER_UR: V;
  const CORNER_BL: V;
  const CORNER_BR: V;
  const FOUR_WAY: V;
  const THREE_WAY_UP: V;
  const THREE_WAY_DOWN: V;
  const THREE_WAY_RIGHT: V;
  const THREE_WAY_LEFT: V;
}

pub struct MazeRuleProvider<V: Variant, T: Maze2dTypeSet<V>> {
  rules: HashMap<(Dim2d, V), Socket>,
  _pd: PhantomData<T>,
}

impl<V, T> Default for MazeRuleProvider<V, T>
where
  V: Variant,
  T: Maze2dTypeSet<V>,
{
  fn default() -> Self {
    Self {
      rules: hashmap! {
        (Dim2d::Up, T::ENTRANCE) => Socket::Vertical,
        (Dim2d::Down, T::ENTRANCE) => Socket::Vertical,
        (Dim2d::Left, T::ENTRANCE) => Socket::Horizontal,
        (Dim2d::Right, T::ENTRANCE) => Socket::Horizontal,

        (Dim2d::Up, T::EXIT) => Socket::Vertical,
        (Dim2d::Down, T::EXIT) => Socket::Vertical,
        (Dim2d::Left, T::EXIT) => Socket::Horizontal,
        (Dim2d::Right, T::EXIT) => Socket::Horizontal,

        (Dim2d::Up, T::EMPTY) => Socket::VerticalBreak,
        (Dim2d::Down, T::EMPTY) => Socket::VerticalBreak,
        (Dim2d::Left, T::EMPTY) => Socket::HorizontalBreak,
        (Dim2d::Right, T::EMPTY) => Socket::HorizontalBreak,

        (Dim2d::Up, T::VERTICAL) => Socket::Vertical,
        (Dim2d::Down,T:: VERTICAL) => Socket::Vertical,
        (Dim2d::Left, T::VERTICAL) => Socket::VerticalBreak,
        (Dim2d::Right, T::VERTICAL) => Socket::VerticalBreak,

        (Dim2d::Up, T::HORIZONTAL) => Socket::HorizontalBreak,
        (Dim2d::Down, T::HORIZONTAL) => Socket::HorizontalBreak,
        (Dim2d::Left, T::HORIZONTAL) => Socket::Horizontal,
        (Dim2d::Right, T::HORIZONTAL) => Socket::Horizontal,

        (Dim2d::Up, T::CORNER_UL) => Socket::VerticalBreak,
        (Dim2d::Down, T::CORNER_UL) => Socket::Vertical,
        (Dim2d::Left, T::CORNER_UL) => Socket::HorizontalBreak,
        (Dim2d::Right, T::CORNER_UL) => Socket::Horizontal,

        (Dim2d::Up, T::CORNER_UR) => Socket::VerticalBreak,
        (Dim2d::Down, T::CORNER_UR) => Socket::Vertical,
        (Dim2d::Left, T::CORNER_UR) => Socket::Horizontal,
        (Dim2d::Right, T::CORNER_UR) => Socket::HorizontalBreak,

        (Dim2d::Up, T::CORNER_BL) => Socket::Vertical,
        (Dim2d::Down, T::CORNER_BL) => Socket::VerticalBreak,
        (Dim2d::Left, T::CORNER_BL) => Socket::HorizontalBreak,
        (Dim2d::Right, T::CORNER_BL) => Socket::Horizontal,

        (Dim2d::Up, T::CORNER_BR) => Socket::Vertical,
        (Dim2d::Down, T::CORNER_BR) => Socket::VerticalBreak,
        (Dim2d::Left, T::CORNER_BR) => Socket::Horizontal,
        (Dim2d::Right, T::CORNER_BR) => Socket::HorizontalBreak,

        (Dim2d::Up, T::FOUR_WAY) => Socket::Vertical,
        (Dim2d::Down, T::FOUR_WAY) => Socket::Vertical,
        (Dim2d::Left, T::FOUR_WAY) => Socket::Horizontal,
        (Dim2d::Right, T::FOUR_WAY) => Socket::Horizontal,

        (Dim2d::Up, T::THREE_WAY_UP) => Socket::Vertical,
        (Dim2d::Down, T::THREE_WAY_UP) => Socket::VerticalBreak,
        (Dim2d::Left, T::THREE_WAY_UP) => Socket::Horizontal,
        (Dim2d::Right, T::THREE_WAY_UP) => Socket::Horizontal,

        (Dim2d::Up, T::THREE_WAY_DOWN) => Socket::VerticalBreak,
        (Dim2d::Down, T::THREE_WAY_DOWN) => Socket::Vertical,
        (Dim2d::Left, T::THREE_WAY_DOWN) => Socket::Horizontal,
        (Dim2d::Right, T::THREE_WAY_DOWN) => Socket::Horizontal,

        (Dim2d::Up, T::THREE_WAY_RIGHT) => Socket::Vertical,
        (Dim2d::Down, T::THREE_WAY_RIGHT) => Socket::Vertical,
        (Dim2d::Left, T::THREE_WAY_RIGHT) => Socket::HorizontalBreak,
        (Dim2d::Right, T::THREE_WAY_RIGHT) => Socket::Horizontal,

        (Dim2d::Up, T::THREE_WAY_LEFT) => Socket::Vertical,
        (Dim2d::Down, T::THREE_WAY_LEFT) => Socket::Vertical,
        (Dim2d::Left, T::THREE_WAY_LEFT) => Socket::Horizontal,
        (Dim2d::Right, T::THREE_WAY_LEFT) => Socket::HorizontalBreak,
      },
      _pd: PhantomData,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy", derive(bevy_reflect::Reflect))]
pub enum Socket {
  Vertical,
  VerticalBreak,
  Horizontal,
  HorizontalBreak,
}

impl<V, T> SocketProvider<V, Dim2d, Socket> for MazeRuleProvider<V, T>
where
  V: Variant,
  T: Maze2dTypeSet<V>,
{
  type WorkingType = Socket;

  #[profiling::function]
  fn find(
    &self,
    _current: Option<Self::WorkingType>,
    dir: Dim2d,
    source: &V,
    _target: &V,
  ) -> FindResult<Socket> {
    self
      .rules
      .get(&(dir, source.clone()))
      .cloned()
      .ok_or(NoSocket)
      .map(Some)
  }

  fn finalize(&self, _dir: Dim2d, socket: Self::WorkingType) -> Socket {
    socket
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::{Maze2dTypeSet, MazeRuleProvider};
  use crate::{
    prebuilt::{
      arbiters::{LimitAdjuster, WeightArbiter},
      auto::GenericFinder,
      constraints::UnaryConstraint,
      shapes::{InformedShape, MultiShape, WeightedShape},
    },
    Adjuster, RuleFinder, StateBuilder,
  };
  use maplit::hashmap;

  const ROWS: usize = 32;
  const COLS: usize = 32;

  const INFLUENCE_RADIUS: f64 = 2.0;
  const SEED: u64 = 123;

  const EXPECTED_OUTPUT: &str = "\
    E╦╦╗╔╗╠╗╠╩╝╔╦╗╔╝╚╣╔╩╣╔╝╔╗╠╣╠╩╦╝╔\
    ╝╚╩╩╩╩╩╩╩╦╦╝╠╝╠╦╗╠╣╔╣╠╗╠╝╚╝╠╦╩╦╝\
    .╔╗╔╦╗.╔╗╠╝╔╣╔╩╩╩╩╝╚╣╚╣╠╗╔╗╠╝.╠╗\
    .╚╩╩╩╣.╠╩╝.╚╩╣.╔╗.╔╦╣╔╩╩╣╚╩╩╦╗╠╣\
    ╦╗╔╗╔╩╗╠╦╗.╔╦╣.╚╩╗╠╝╚╝..╚╗.╔╩╣╠╣\
    ╣╠╩╝╚╦╝╚╩╩╦╣╠╣╔╗.╚╩╦╗╔╦╦╦╝╔╣.╠╩╣\
    ╩╝╔╦╗╠╦╦╦╦╝╠╣╠╝╚╗.╔╝╠╩╣╠╩╦╩╝╔╝╔╩\
    ╔╗╠╩╣╚╝╚╣╚╦╝╠╩╗.╚╦╣.╚╗╚╩╗╚╦╗╠╗╠╗\
    ╣╚╩╦╝.╔╦╣.╚╦╩╦╩╦╗╠╣..╠╗╔╝╔╣╠╩╩╝╠\
    ╩╦╦╣╔╦╩╝╠╦╦╝.╚╗╠╣╚╣╔╗╚╝╚╗╠╣╚╦╦╗╚\
    ╔╩╝╚╝╠╦╗╚╝╠╦╦╦╝╚╝╔╝╠╣.╔╦╩╩╣╔╣╚╝╔\
    ╝.╔╗.╠╣╚╗╔╩╝╚╣╔╗╔╩╦╩╩╦╣╚╗╔╩╣╠╗.╚\
    ╦╗╠╣╔╣╠╦╩╩╦╗.╠╣╠╝╔╝╔╗╠╣.╠╩╦╩╩╩╦╦\
    ╚╩╝╚╩╣╚╩╗╔╩╩╦╩╩╩╦╣.╚╣╠╣╔╩╦╝..╔╩╩\
    ╦╦╦╦╦╝╔╦╩╝.╔╣..╔╝╠╦╗╠╝╚╣.╚╗.╔╣╔╦\
    ╣╚╣╚╩╦╣╚╦╦╗╚╩╗.╚╗╚╝╠╝..╠╦╦╩╗╠╝╚╣\
    ╩╗╚╗.╚╝.╚╩╝╔╦╩╗.╚╦╦╣..╔╝╚╣╔╣╠╗╔╩\
    ╦╣╔╝.╔╗..╔╗╠╝╔╣╔╦╩╩╣╔╦╣..╚╝╠╩╣╠╦\
    ╚╝╠╗.╚╝╔╦╣╠╣╔╩╝╚╝╔╦╣╠╣╚╗..╔╣╔╣╠╩\
    .╔╩╝╔╗╔╩╣╠╣╚╣╔╦╦╦╩╩╣╚╣╔╩╗╔╩╣╚╩╣╔\
    .╚╗╔╝╚╝.╠╝╚╦╩╣╠╣╠╦╗╠╗╚╝╔╩╣╔╝╔╦╣╠\
    ╗╔╣╚╗..╔╝..╚╗╚╣╠╝╚╩╝╚╦╗╚╦╣╠╗╚╩╣╠\
    ╩╣╚╦╩╗╔╝...╔╣.╚╣...╔╦╣╠╦╩╣╠╩╦╗╠╝\
    ╔╩╗╚╦╩╩╦╦╗╔╝╚╦╦╝.╔╦╣╠╝╚╣.╚╩╗╠╝╠╗\
    ╣.╚╦╝..╚╝╚╝.╔╩╩╗╔╣╚╣╚╗╔╩╦╗.╠╣╔╝╠\
    ╠╦╦╝╔╦╦╗.╔╦╦╣.╔╝╚╝╔╣╔╩╣╔╝╚╗╚╩╩╦╣\
    ╣╚╩╦╝╚╝╚╦╣╚╝╚╗╠╦╦╦╝╠╝╔╝╠╦╗╚╦╦╦╝╚\
    ╠╗╔╝.╔╗╔╣╚╗..╚╩╣╚╝.╠╗╚╦╝╚╝╔╣╚╩╗╔\
    ╩╩╣╔╦╩╣╠╩╗╠╗...╠╦╦╗╚╣.╠╦╦╦╩╝.╔╣╠\
    ╗╔╣╚╝╔╝╚╗╠╣╚╗╔╦╩╩╩╝.╠╦╣╚╣╠╦╗.╠╩╩\
    ╚╝╚╗╔╩╦╦╩╝╚╦╣╠╝╔╦╗.╔╩╩╣╔╝╚╣╚╗╚╗╔\
    ╔╗.╠╝.╠╩╦╦╗╠╣╚╦╣╠╩╦╝..╠╝╔╦╣.╚╗╠X\
  ";

  #[derive(Debug)]
  struct TextMaze;

  impl Maze2dTypeSet<char> for TextMaze {
    const ENTRANCE: char = 'E';
    const EXIT: char = 'X';
    const EMPTY: char = '.';
    const VERTICAL: char = '║';
    const HORIZONTAL: char = '═';
    const CORNER_UL: char = '╔';
    const CORNER_UR: char = '╗';
    const CORNER_BL: char = '╚';
    const CORNER_BR: char = '╝';
    const FOUR_WAY: char = '╬';
    const THREE_WAY_UP: char = '╩';
    const THREE_WAY_DOWN: char = '╦';
    const THREE_WAY_RIGHT: char = '╠';
    const THREE_WAY_LEFT: char = '╣';
  }

  #[test]
  fn output_test() {
    let rows = 7;
    let cols = 10;
    let source = "\
    ╔════════╗\
    ║...╔═╦═╗║\
    ║.╔═╩═╣.║║\
    E╦╣╔══╣.╠X\
    ║║╚╝.╔╩═╝║\
    ║╚═══╝...║\
    ╚════════╝\
    "
    .chars()
    .collect::<Vec<_>>();

    assert_eq!(source.len(), rows * cols);
    assert_eq!(EXPECTED_OUTPUT.chars().count(), ROWS * COLS);

    let finder = GenericFinder::new(
      MazeRuleProvider::<char, TextMaze>::default(),
      source,
      [cols, rows],
    );

    let rules = finder.find().unwrap();

    let weights = rules
      .variants()
      .map(|k| (*k, 1))
      .collect::<HashMap<char, usize>>();

    let shape = MultiShape::new(
      WeightedShape::new(weights),
      InformedShape::new(INFLUENCE_RADIUS, 1, HashMap::default()),
    );

    let arbiter = WeightArbiter::new(Some(SEED), shape).chain(LimitAdjuster::new(hashmap! {
      TextMaze::ENTRANCE => 0,
      TextMaze::EXIT => 0,
    }));

    let mut builder = StateBuilder::new([COLS, ROWS], arbiter, UnaryConstraint, rules);

    builder
      .insert([0, 0], TextMaze::ENTRANCE)
      .insert([COLS - 1, ROWS - 1], TextMaze::EXIT);

    let mut state = builder.build().unwrap();

    crate::collapse(&mut state).unwrap();

    let actual: Vec<_> = state.into();

    let expected = Vec::from_iter(EXPECTED_OUTPUT.chars());
    assert_eq!(expected.len(), actual.len());
    assert_eq!(expected, actual);
  }
}
