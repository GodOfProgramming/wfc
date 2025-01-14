use maplit::hashmap;
use prebuilt::{
  arbiters::{LimitAdjuster, WeightArbiter},
  auto::GenericFinder,
  constraints::UnaryConstraint,
  e2e::maze2d::{Maze2dTypeSet, MazeRuleProvider},
  shapes::{InformedShape, MultiShape, WeightedShape},
  Dim2d,
};
use std::{
  collections::HashMap,
  error::Error,
  fmt::{Debug, Display},
};
use wfc::{prelude::*, Adjuster, Arbiter, Constraint, Dimension, Socket, Variant};

const STEP_BY_STEP: bool = false;

const ROWS: usize = 16;
const COLS: usize = 32;

const INFLUENCE_RADIUS: f64 = 2.0;

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

fn main() -> Result<(), Box<dyn Error>> {
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

  let finder = GenericFinder::new(
    MazeRuleProvider::<char, TextMaze>::default(),
    source,
    [cols, rows],
  );

  let rules = match finder.find() {
    Ok(rules) => rules,
    Err(e) => {
      eprintln!("{e}");
      return Ok(());
    }
  };

  let args = std::env::args().collect::<Vec<_>>();

  let seed: Option<u64> = args.get(1).map(|arg| arg.parse()).transpose().unwrap();

  let weights = rules.variants().map(|k| (*k, 1));

  let shape = MultiShape::new(
    WeightedShape::new(weights, &rules),
    InformedShape::new(INFLUENCE_RADIUS, 1, HashMap::default()),
  );

  let arbiter = WeightArbiter::new(seed, shape);
  println!("Seed: {}", arbiter.seed());

  let arbiter = arbiter.chain(LimitAdjuster::new(
    hashmap! {
      TextMaze::ENTRANCE => 0,
      TextMaze::EXIT => 0,
    },
    &rules,
  ));

  let mut builder = StateBuilder::new([COLS, ROWS], arbiter, UnaryConstraint::default(), rules);

  let vertical = vec![TextMaze::VERTICAL; COLS * ROWS];
  let horizontal = vec![TextMaze::HORIZONTAL; COLS * ROWS];
  builder
    .with_ext(Dim2d::Up, vertical.clone())
    .with_ext(Dim2d::Down, vertical)
    .with_ext(Dim2d::Left, horizontal.clone())
    .with_ext(Dim2d::Right, horizontal)
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

fn all_at_once<A, C, V, D, S, const DIM: usize>(mut state: State<A, C, V, D, S, DIM>)
where
  A: Arbiter,
  C: Constraint<Socket = S>,
  V: Variant + Display,
  D: Dimension,
  S: Socket,
{
  if let Err(e) = wfc::collapse(&mut state) {
    eprintln!("{e}");
    return;
  }

  print_state(state);
}

fn step_by_step<A, C, V, D, S, const DIM: usize>(mut state: State<A, C, V, D, S, DIM>)
where
  A: Arbiter,
  C: Constraint<Socket = S>,
  V: Variant + Display + Default,
  D: Dimension,
  S: Socket,
{
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

fn print_state<A, C, V, D, S, const DIM: usize>(state: State<A, C, V, D, S, DIM>)
where
  A: Arbiter,
  C: Constraint<Socket = S>,
  V: Variant + Display,
  D: Dimension,
  S: Socket,
{
  let data: Vec<_> = state.into();

  let output = itertools::join(
    (0..ROWS).map(|i| {
      let slice = &data[i * COLS..i * COLS + COLS];
      itertools::join(slice, "")
    }),
    "\n",
  );

  println!("\n{output}\n");

  let mut rows = (0..ROWS)
    .map(|i| {
      let slice = &data[i * COLS..i * COLS + COLS];
      itertools::join(slice, "")
    })
    .collect::<Vec<_>>();

  rows.reverse();

  let output = itertools::join(rows, "\n");
  println!("{output}");
}
