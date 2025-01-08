use maplit::hashmap;
use prebuilt::{
  arbiters::{LimitAdjuster, MultiPhaseArbitration, WeightArbiter},
  auto::GenericFinder,
  constraints::{DefaultConstrainer, UnaryConstrainer},
  e2e::maze2d::{Maze2dTechnique, MazeRuleProvider, Socket},
  shapes::{InformedShape, MultiShape, WeightedShape},
  Dim2d,
};
use std::{collections::HashMap, error::Error, fmt::Debug};
use wfc::{prelude::*, Adjuster};

const STEP_BY_STEP: bool = false;
const ROWS: usize = 16;
const COLS: usize = 32;
const INFLUENCE_RADIUS: f64 = 2.0;

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
  type Dimension = Dim2d;
  type Variant = char;
  type Socket = Option<Socket>;
  type Constraint = DefaultConstrainer;
  type Arbiter = MultiPhaseArbitration<WeightArbiter<Self, 2>, LimitAdjuster<Self, 2>, Self, 2>;
  type Weight = u8;
  type Shape = MultiShape<WeightedShape<Self, 2>, InformedShape<Self, 2>, Self, 2>;
}

fn main() -> Result<(), Box<dyn Error>> {
  run()?;
  run()
}

fn run() -> Result<(), Box<dyn Error>> {
  #[cfg(feature = "profiling")]
  let _guards = wfc::perf::enable_profiling();

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

  let finder = GenericFinder::new(MazeRuleProvider::<TextMaze>::new(), source, [cols, rows]);

  let rules = match finder.find() {
    Ok(rules) => rules,
    Err(e) => {
      eprintln!("{e}");
      return Ok(());
    }
  };

  let args = std::env::args().collect::<Vec<_>>();

  let seed: Option<u64> = args.get(1).map(|arg| arg.parse()).transpose().unwrap();

  let weights = rules.keys().map(|k| (k.clone(), 1)).collect();

  let shape = MultiShape::new(
    WeightedShape::new(weights),
    InformedShape::new(INFLUENCE_RADIUS, 1, HashMap::default()),
  );

  let arbiter = WeightArbiter::new(seed, shape);
  println!("Seed: {}", arbiter.seed());

  let arbiter = arbiter.chain(LimitAdjuster::new(hashmap! {
    TextMaze::ENTRANCE => 0,
    TextMaze::EXIT => 0,
  }));

  let mut builder = StateBuilder::<TextMaze, 2>::new([COLS, ROWS], arbiter, UnaryConstrainer);

  let vertical = vec![TextMaze::VERTICAL; COLS * ROWS];
  let horizontal = vec![TextMaze::HORIZONTAL; COLS * ROWS];
  builder
    .with_ext(Dim2d::Up, vertical.clone())
    .with_ext(Dim2d::Down, vertical)
    .with_ext(Dim2d::Left, horizontal.clone())
    .with_ext(Dim2d::Right, horizontal)
    .with_rules(rules)
    .insert([0, 0], TextMaze::ENTRANCE)
    .insert([COLS - 1, ROWS - 1], TextMaze::EXIT);

  let state = builder.build()?;

  if STEP_BY_STEP {
    step_by_step(state);
  } else {
    all_at_once(state);
  }

  Ok(())
}

fn all_at_once(mut state: State<TextMaze, 2>) {
  if let Err(e) = wfc::collapse(&mut state) {
    eprintln!("{e}");
    return;
  }

  print_state(state);
}

fn step_by_step(mut state: State<TextMaze, 2>) {
  'c: loop {
    match state.collapse() {
      Ok(Observation::Incomplete(_)) => {
        println!("\n");

        let data: Vec<_> = state.data();

        let output = itertools::join(
          (0..ROWS).map(|i| {
            let slice = &data[i * COLS..i * COLS + COLS];
            itertools::join(slice, "")
          }),
          "\n",
        );

        println!("{output}");
      }
      Err(err) => {
        eprintln!("{err}");
        break 'c;
      }
      _ => {
        break 'c;
      }
    }
  }

  print_state(state);
}

fn print_state(state: State<TextMaze, 2>) {
  let data: Vec<_> = state.data_default(' ');

  let output = itertools::join(
    (0..ROWS).map(|i| {
      let slice = &data[i * COLS..i * COLS + COLS];
      itertools::join(slice, "")
    }),
    "\n",
  );

  println!("\n{output}\n");
}
