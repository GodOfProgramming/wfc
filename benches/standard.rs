use criterion::{criterion_group, criterion_main};

criterion_main!(base);

criterion_group!(base, text::bench, misc::bench);

const SEED: u64 = 123;

mod text {
  use crate::SEED;
  use criterion::Criterion;
  use maplit::hashmap;
  use std::collections::HashMap;
  use wfc::{
    prebuilt::{
      arbiters::{LimitAdjuster, MultiPhaseArbitration, WeightArbiter},
      auto::GenericFinder,
      constraints::{DefaultConstrainer, UnaryConstrainer},
      e2e::maze2d::{Maze2dTechnique, MazeRuleProvider, Socket},
      shapes::WeightedShape,
      weights::DirectWeight,
      Dim2d,
    },
    Adjuster, RuleFinder, Rules, Size, StateBuilder, TypeAtlas,
  };

  #[derive(Debug)]
  struct TextMazeBench;

  impl Maze2dTechnique for TextMazeBench {
    const ENTRANCE: char = 'E';
    const EXIT: char = 'X';
    const EMPTY: char = ' ';
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

  impl TypeAtlas<2> for TextMazeBench {
    type Dimension = Dim2d;
    type Variant = char;
    type Socket = Option<Socket>;
    type Constraint = DefaultConstrainer;
    type Arbiter = MultiPhaseArbitration<WeightArbiter<Self, 2>, LimitAdjuster<Self, 2>, Self, 2>;
    type Weight = DirectWeight;
    type Shape = WeightedShape<Self, 2>;
  }

  fn get_rules() -> Rules<
    <TextMazeBench as TypeAtlas<2>>::Variant,
    <TextMazeBench as TypeAtlas<2>>::Dimension,
    <TextMazeBench as TypeAtlas<2>>::Socket,
  > {
    let rows = 7;
    let cols = 10;
    let source = "\
  ╔════════╗\
  ║   ╔═╦═╗║\
  ║ ╔═╩═╣ ║║\
  E╦╣╔══╣ ╠X\
  ║║╚╝ ╔╩═╝║\
  ║╚═══╝   ║\
  ╚════════╝\
  "
    .chars()
    .collect::<Vec<_>>();

    let finder = GenericFinder::new(
      MazeRuleProvider::<TextMazeBench>::new(),
      source,
      [cols, rows],
    );

    finder.find().unwrap()
  }

  pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("text-maze");

    for pow in 4_u32..8_u32 {
      let dims = 2_u32.pow(pow);

      let arbiter = WeightArbiter::new(Some(SEED), WeightedShape::new(HashMap::default())).chain(
        LimitAdjuster::new(hashmap! {
          TextMazeBench::ENTRANCE => 0,
          TextMazeBench::EXIT => 0,
        }),
      );

      let size = Size::new([dims as usize, dims as usize]);

      let mut builder = StateBuilder::<TextMazeBench, 2>::new(size, arbiter, UnaryConstrainer);

      builder
        .with_rules(get_rules())
        .insert([size.x - 1, size.y - 1], TextMazeBench::EXIT)
        .insert([0, 0], TextMazeBench::ENTRANCE);

      group.bench_function(format!("{dims}x{dims}"), |b| {
        b.iter(|| execute(builder.clone()))
      });
    }
  }

  fn execute(builder: StateBuilder<TextMazeBench, 2>) {
    let mut state = builder.build().expect("Failed to build state");

    wfc::collapse(&mut state).expect("Failed to collapse");
  }
}

mod misc {
  use crate::SEED;
  use criterion::Criterion;
  use maplit::hashmap;
  use prebuilt::{
    arbiters::RandomArbiter, constraints::SetConstrainer, shapes::NoShape, weights::DirectWeight,
  };
  use std::collections::BTreeSet;
  use wfc::{prebuilt::Dim3d, prelude::*, Rule, Size, StateBuilder};

  #[derive(Debug)]
  struct Bench;

  impl TypeAtlas<3> for Bench {
    type Variant = usize;
    type Dimension = Dim3d;
    type Socket = BTreeSet<usize>;
    type Arbiter = RandomArbiter<Self, 3>;
    type Constraint = SetConstrainer;
    type Weight = DirectWeight;
    type Shape = NoShape;
  }

  pub fn bench(c: &mut Criterion) {
    c.benchmark_group("misc")
      .sample_size(10)
      .bench_function("50x50x50", |b| b.iter(|| execute([50, 50, 50])))
      .bench_function("minecraft chunk", |b| b.iter(|| execute([16, 16, 256])));
  }

  fn execute(size: impl Into<Size<3>>) {
    let mut builder =
      StateBuilder::<Bench, 3>::new(size, RandomArbiter::new(Some(SEED)), SetConstrainer);
    builder.with_rules(hashmap! {
      0 => Rule::from_fn(|_| BTreeSet::from_iter([0, 1])),
      1 => Rule::from_fn(|_| BTreeSet::from_iter([0, 1, 2])),
      2 => Rule::from_fn(|_| BTreeSet::from_iter([1, 2, 3])),
      3 => Rule::from_fn(|_| BTreeSet::from_iter([2, 3])),
    });
    let mut state = builder.build().expect("Failed to build state");
    wfc::collapse(&mut state).expect("Failed to collapse");
  }
}
