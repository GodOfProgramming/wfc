use std::any::Any;
use tracing;
use tracing_subscriber::Registry;
use tracing_subscriber::{self, layer::SubscriberExt};

#[must_use]
pub fn enable_profiling() -> Vec<Box<dyn Any>> {
  let subscriber = Registry::default();
  let mut guards: Vec<Box<dyn Any>> = Vec::new();

  #[cfg(feature = "tracing-tracy")]
  let subscriber = {
    use tracing_tracy::client::Client;
    use tracing_tracy::TracyLayer;

    let (tracy_layer, tracy_client) = (TracyLayer::default(), Client::start());

    guards.push(Box::new(tracy_client));
    subscriber.with(tracy_layer)
  };

  #[cfg(feature = "tracing-chrome")]
  let subscriber = {
    use chrono::prelude::*;
    use std::fs::File;
    use tracing_chrome::ChromeLayerBuilder;

    let output_file = format!(
      "target/trace-{variant}.json",
      variant = Local::now().to_string()
    );

    println!("Saving results to {output_file}");

    let (chrome_layer, chrome_guard) = ChromeLayerBuilder::new()
      .writer(File::create(output_file).expect("Failed to create trace file."))
      .build();

    guards.push(Box::new(chrome_guard));
    subscriber.with(chrome_layer)
  };

  tracing::subscriber::set_global_default(subscriber).expect("failed to setup tracy");

  guards
}
