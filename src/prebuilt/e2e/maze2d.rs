use crate::prebuilt::Dim2d;
use crate::{FindResult, NoSocket, SocketProvider, TypeAtlas};
use maplit::hashmap;
use std::{collections::HashMap, hash::Hash};

pub trait Maze2dTechnique: TypeAtlas<2, Dimension = Dim2d> {
  const ENTRANCE: Self::Variant;
  const EXIT: Self::Variant;
  const EMPTY: Self::Variant;
  const VERTICAL: Self::Variant;
  const HORIZONTAL: Self::Variant;
  const CORNER_UL: Self::Variant;
  const CORNER_UR: Self::Variant;
  const CORNER_BL: Self::Variant;
  const CORNER_BR: Self::Variant;
  const FOUR_WAY: Self::Variant;
  const THREE_WAY_UP: Self::Variant;
  const THREE_WAY_DOWN: Self::Variant;
  const THREE_WAY_RIGHT: Self::Variant;
  const THREE_WAY_LEFT: Self::Variant;
}

pub struct MazeRuleProvider<T: Maze2dTechnique> {
  rules: HashMap<(Dim2d, T::Variant), Socket>,
}

impl<T> Default for MazeRuleProvider<T>
where
  T: Maze2dTechnique,
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

impl<T> SocketProvider<T::Variant, Dim2d, Socket> for MazeRuleProvider<T>
where
  T: Maze2dTechnique,
{
  type WorkingType = Socket;

  #[profiling::function]
  fn find(
    &self,
    _current: Option<Self::WorkingType>,
    dir: Dim2d,
    source: &T::Variant,
    _target: &T::Variant,
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

  use super::{Maze2dTechnique, MazeRuleProvider, Socket};
  use crate::{
    prebuilt::{
      arbiters::{LimitAdjuster, MultiPhaseArbitration, WeightArbiter},
      auto::GenericFinder,
      constraints::UnaryConstraint,
      shapes::{InformedShape, MultiShape, WeightedShape},
      Dim2d,
    },
    Adjuster, RuleFinder, StateBuilder, TypeAtlas,
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

  impl Maze2dTechnique for TextMaze {
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

  impl TypeAtlas<2> for TextMaze {
    type Variant = char;
    type Dimension = Dim2d;
    type Socket = Option<Socket>;
    type Constraint = UnaryConstraint;
    type Arbiter = MultiPhaseArbitration<
      WeightArbiter<
        MultiShape<WeightedShape<usize, Self, 2>, InformedShape<usize, Self, 2>, Self, 2>,
        Self,
        2,
      >,
      LimitAdjuster<Self, 2>,
      Self,
      2,
    >;
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
      MazeRuleProvider::<TextMaze>::default(),
      source,
      [cols, rows],
    );

    let rules = finder.find().unwrap();

    let weights: HashMap<char, usize> = rules.keys().map(|k| (*k, 1)).collect();

    let shape = MultiShape::new(
      WeightedShape::new(weights),
      InformedShape::new(INFLUENCE_RADIUS, 1, HashMap::default()),
    );

    let arbiter: <TextMaze as TypeAtlas<2>>::Arbiter =
      WeightArbiter::new(Some(SEED), shape).chain(LimitAdjuster::new(hashmap! {
        TextMaze::ENTRANCE => 0,
        TextMaze::EXIT => 0,
      }));

    let mut builder = StateBuilder::<TextMaze, 2>::new([COLS, ROWS], arbiter, UnaryConstraint);

    builder
      .with_rules(rules)
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
