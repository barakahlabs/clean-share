//! Fetching from a small local server: limits, content types, redirects and oEmbed discovery.

use std::time::Duration;

use link_preview::{Config, Error, Fetcher};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::TcpListener,
};

/// Serves a few fixed routes on 127.0.0.1 and returns its base URL.
async fn server() -> String {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let base = format!("http://{}", listener.local_addr().unwrap());
  let origin = base.clone();
  tokio::spawn(async move {
    loop {
      let (mut socket, _) = listener.accept().await.unwrap();
      let origin = origin.clone();
      tokio::spawn(async move {
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
        let (status, content_type, body, delay) = route(&path, &origin);
        tokio::time::sleep(delay).await;
        let head = format!(
          "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n{}\r\n",
          body.len(),
          if status.starts_with("302") { format!("location: {origin}/loop\r\n") } else { String::new() }
        );
        let _ = socket.write_all(head.as_bytes()).await;
        let _ = socket.write_all(&body).await;
      });
    }
  });
  base
}

fn route(path: &str, origin: &str) -> (&'static str, &'static str, Vec<u8>, Duration) {
  let html = |s: String| ("200 OK", "text/html; charset=utf-8", s.into_bytes(), Duration::ZERO);
  match path {
    "/page" => html(
      r#"<html><head><meta property="og:title" content="Local page"><meta property="og:image" content="/pic.png"></head><body><p>Hello.</p></body></html>"#.into(),
    ),
    // 3 MB of body after the metadata: cut off at the limit, still parsed.
    "/big" => html(format!(r#"<html><head><title>Big page</title></head><body>{}</body></html>"#, "x".repeat(3 << 20))),
    "/discover" => html(format!(
      r#"<html><head><link rel="alternate" type="application/json+oembed" href="{origin}/oembed.json"></head></html>"#
    )),
    "/oembed.json" => (
      "200 OK",
      "application/json",
      br#"{"type":"video","title":"From oEmbed","provider_name":"Local","thumbnail_url":"https://cdn.example/t.jpg"}"#.to_vec(),
      Duration::ZERO,
    ),
    "/latin1" => ("200 OK", "text/html; charset=iso-8859-1", b"<title>Caf\xe9</title>".to_vec(), Duration::ZERO),
    "/file.pdf" => ("200 OK", "application/pdf", b"%PDF-1.7".to_vec(), Duration::ZERO),
    "/pic.png" => ("200 OK", "image/png", b"\x89PNG\r\n\x1a\nfake".to_vec(), Duration::ZERO),
    "/huge.png" => ("200 OK", "image/png", vec![0; 6 << 20], Duration::ZERO),
    "/loop" => ("302 Found", "text/html", Vec::new(), Duration::ZERO),
    "/slow" => html("<title>late</title>".into()).with_delay(Duration::from_secs(3)),
    _ => ("404 Not Found", "text/html", b"nope".to_vec(), Duration::ZERO),
  }
}

trait WithDelay {
  fn with_delay(self, d: Duration) -> Self;
}

impl WithDelay for (&'static str, &'static str, Vec<u8>, Duration) {
  fn with_delay(self, d: Duration) -> Self {
    (self.0, self.1, self.2, d)
  }
}

fn local() -> Fetcher {
  Fetcher::new(Config { allow_private: true, timeout: Duration::from_secs(1), ..Config::default() }).unwrap()
}

#[tokio::test]
async fn previews_a_page_with_absolute_image_links() {
  let base = server().await;
  let p = local().preview(&format!("{base}/page")).await.unwrap();
  assert_eq!(p.url, format!("{base}/page"));
  assert_eq!(p.title.as_deref(), Some("Local page"));
  assert_eq!(p.image_url, Some(format!("{base}/pic.png")));
  assert_eq!(p.excerpt.as_deref(), Some("Hello."));
}

#[tokio::test]
async fn big_pages_are_cut_off_and_still_parsed() {
  let base = server().await;
  let p = local().preview(&format!("{base}/big")).await.unwrap();
  assert_eq!(p.title.as_deref(), Some("Big page"));
}

#[tokio::test]
async fn follows_the_pages_own_oembed_link() {
  let base = server().await;
  let p = local().preview(&format!("{base}/discover")).await.unwrap();
  assert_eq!(p.title.as_deref(), Some("From oEmbed"));
  assert_eq!(p.kind.as_deref(), Some("video"));
  assert_eq!(p.image_url.as_deref(), Some("https://cdn.example/t.jpg"));
}

#[tokio::test]
async fn uses_the_charset_from_the_header() {
  let base = server().await;
  let p = local().preview(&format!("{base}/latin1")).await.unwrap();
  assert_eq!(p.title.as_deref(), Some("Café"));
}

#[tokio::test]
async fn refuses_what_isnt_a_page() {
  let base = server().await;
  assert!(matches!(local().preview(&format!("{base}/file.pdf")).await, Err(Error::NotHtml(t)) if t == "application/pdf"));
  assert!(matches!(local().preview(&format!("{base}/missing")).await, Err(Error::Status(404))));
}

#[tokio::test]
async fn stops_redirect_loops_and_slow_sites() {
  let base = server().await;
  assert!(matches!(local().preview(&format!("{base}/loop")).await, Err(Error::Network(_))));
  assert!(matches!(local().preview(&format!("{base}/slow")).await, Err(Error::Timeout)));
}

#[tokio::test]
async fn downloads_images_within_the_size_limit() {
  let base = server().await;
  let image = local().image(&format!("{base}/pic.png")).await.unwrap();
  assert_eq!(image.content_type, "image/png");
  assert!(image.bytes.starts_with(b"\x89PNG"));
  assert!(matches!(local().image(&format!("{base}/huge.png")).await, Err(Error::TooLarge(_))));
  assert!(matches!(local().image(&format!("{base}/page")).await, Err(Error::NotImage(_))));
}

#[tokio::test]
async fn private_addresses_are_refused_by_default() {
  let base = server().await;
  let fetcher = Fetcher::new(Config::default()).unwrap();
  assert!(matches!(fetcher.preview(&format!("{base}/page")).await, Err(Error::Blocked(_))));
  assert!(matches!(fetcher.image(&format!("{base}/pic.png")).await, Err(Error::Blocked(_))));
  // A name that resolves to a local address is refused too.
  let by_name = base.replace("127.0.0.1", "localhost");
  assert!(matches!(fetcher.preview(&format!("{by_name}/page")).await, Err(Error::Blocked(_))));
}
