use maplit::hashmap;
use wfc::{prebuilt::Dim2d, Rule, Rules};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
enum Variants {
  VariantA,
  VariantB,
  VariantC,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
enum Sockets {
  SocketA,
  SocketB,
  SocketC,
}

fn main() {
  let _ = Rules::<Variants, Dim2d, Sockets>::new(hashmap! {
    Variants::VariantA => Rule::splat(Sockets::SocketA),
    Variants::VariantB => Rule::splat(Sockets::SocketB),
    Variants::VariantC => Rule::splat(Sockets::SocketC),
  });

  // println!("{:#?}", rules.socket_cache);
}
