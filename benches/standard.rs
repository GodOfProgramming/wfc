use criterion::{criterion_group, criterion_main};

criterion_main!(base);

criterion_group!(base, text::bench, misc::bench);

const SEED: u64 = 123;

mod text {
  use crate::SEED;
  use criterion::Criterion;
  use maplit::hashmap;
  use wfc::{
    prebuilt::{
      arbiters::{LimitAdjuster, RandomArbiter},
      auto::GenericFinder,
      constraints::UnaryConstraint,
      e2e::maze2d::{Maze2dTypeSet, MazeRuleProvider, Socket},
      Dim2d,
    },
    Adjuster, Arbiter, Constraint, RuleFinder, Rules, Size, StateBuilder,
  };

  #[derive(Debug)]
  struct TextMazeBench;

  impl Maze2dTypeSet<char> for TextMazeBench {
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

  fn get_rules() -> Rules<char, Dim2d, Option<Socket>> {
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
      MazeRuleProvider::<char, TextMazeBench>::default(),
      source,
      [cols, rows],
    );

    finder.find().unwrap()
  }

  pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("text-maze");

    let rules = get_rules();

    for pow in 4_u32..8_u32 {
      let dims = 2_u32.pow(pow);

      let arbiter = RandomArbiter::new(Some(SEED)).chain(LimitAdjuster::new(
        hashmap! {
          TextMazeBench::ENTRANCE => 0,
          TextMazeBench::EXIT => 0,
        },
        &rules,
      ));

      let size = Size::new([dims as usize, dims as usize]);

      let mut builder = StateBuilder::new(size, arbiter, UnaryConstraint::default(), rules.clone());

      builder
        .insert([size.x - 1, size.y - 1], TextMazeBench::EXIT)
        .insert([0, 0], TextMazeBench::ENTRANCE);

      group.bench_function(format!("{dims}x{dims}"), |b| {
        b.iter(|| execute(builder.clone()))
      });
    }
  }

  fn execute<A, C>(builder: StateBuilder<A, C, char, Dim2d, Option<Socket>, 2>)
  where
    A: Arbiter,
    C: Constraint<Socket = Option<Socket>>,
  {
    let mut state = builder.build().expect("Failed to build state");

    wfc::collapse(&mut state).expect("Failed to collapse");
  }
}

mod misc {
  use crate::SEED;
  use criterion::Criterion;
  use prebuilt::{arbiters::RandomArbiter, constraints::UnaryConstraint};
  use std::collections::BTreeSet;
  use wfc::{prebuilt::Dim3d, prelude::*, Size, StateBuilder};

  pub fn bench(c: &mut Criterion) {
    c.benchmark_group("misc")
      .sample_size(10)
      .bench_function("50x50x50", |b| b.iter(|| execute([50, 50, 50])))
      .bench_function("minecraft chunk", |b| b.iter(|| execute([16, 16, 256])));
  }

  fn execute(size: impl Into<Size<3>>) {
    let rules: Rules<i32, Dim3d, BTreeSet<i32>> = RuleBuilder::default()
      .with_rule(0, |_| BTreeSet::from_iter([0, 1]))
      .with_rule(1, |_| BTreeSet::from_iter([0, 1, 2]))
      .with_rule(2, |_| BTreeSet::from_iter([1, 2, 3]))
      .with_rule(3, |_| BTreeSet::from_iter([2, 3]))
      .into();

    let builder = StateBuilder::new(
      size,
      RandomArbiter::new(Some(SEED)),
      UnaryConstraint::default(),
      rules,
    );
    let mut state = builder.build().expect("Failed to build state");
    wfc::collapse(&mut state).expect("Failed to collapse");
  }
}
