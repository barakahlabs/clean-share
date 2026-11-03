//! Prints the preview for each link given: `cargo run --example preview -- <link>…`

#[tokio::main(flavor = "current_thread")]
async fn main() {
  // ACCEPT_LANGUAGE=en cargo run --example preview -- <link>
  let config = link_preview::Config {
    accept_language: std::env::var("ACCEPT_LANGUAGE").ok(),
    ..Default::default()
  };
  let fetcher = link_preview::Fetcher::new(config).unwrap();
  for link in std::env::args().skip(1) {
    match fetcher.preview(&link).await {
      Ok(preview) => println!("{}", serde_json::to_string_pretty(&preview).unwrap()),
      Err(e) => println!("{link}: {e}"),
    }
  }
}
