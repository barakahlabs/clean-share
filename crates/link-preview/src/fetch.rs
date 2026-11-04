//! HTTP with limits: see [`crate::Config`].

use std::sync::Arc;

use reqwest::{Client, Response, header, redirect};
use url::Url;

use crate::{Config, Error, Preview, Result, guard, html, merge, oembed};

/// Fetches previews and images. Cheap to share: hold one and reuse it (it keeps connections).
pub struct Fetcher {
  client: Client,
  config: Config,
}

/// A file from [`Fetcher::download`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Download {
  /// The server says it hasn't changed since the `etag` we sent (HTTP 304, no body).
  NotModified,
  Fetched {
    bytes: Vec<u8>,
    /// The server's fingerprint of this version, to send next time.
    etag: Option<String>,
  },
}

/// A downloaded image, not yet resized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
  pub bytes: Vec<u8>,
  /// From the response, e.g. `image/jpeg`.
  pub content_type: String,
}

impl Fetcher {
  pub fn new(config: Config) -> Result<Self> {
    let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
      rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| Error::Network(e.to_string()))?
    .with_root_certificates(rustls::RootCertStore {
      roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    })
    .with_no_client_auth();

    let max_redirects = config.max_redirects;
    let allow_private = config.allow_private;
    let policy = redirect::Policy::custom(move |attempt| {
      if attempt.previous().len() >= max_redirects {
        attempt.error(Error::Network("too many redirects".into()))
      } else if let Err(e) = guard::check_url(attempt.url(), allow_private) {
        attempt.error(e)
      } else {
        attempt.follow()
      }
    });

    let mut headers = header::HeaderMap::new();
    if let Some(languages) = &config.accept_language {
      let value = header::HeaderValue::from_str(languages).map_err(|e| Error::Network(e.to_string()))?;
      headers.insert(header::ACCEPT_LANGUAGE, value);
    }
    let mut builder = Client::builder()
      .default_headers(headers)
      .tls_backend_preconfigured(tls)
      .redirect(policy)
      .timeout(config.timeout)
      .user_agent(config.user_agent.clone())
      // No cookies, no referrer: the request carries nothing about the user.
      .referer(false);
    if !allow_private {
      builder = builder.dns_resolver(guard::resolver());
    }
    let client = builder.build().map_err(|e| Error::Network(e.to_string()))?;
    Ok(Self { client, config })
  }

  /// A preview for a cleaned link: the page and, where available, oEmbed, merged. Fails only if
  /// both fail (the page's error is returned).
  pub async fn preview(&self, link: &str) -> Result<Preview> {
    let url = guard::check(link, self.config.allow_private)?;
    let page = self.page(&url).await;

    let oembed_url = oembed::endpoint(&url).or_else(|| {
      let page = page.as_ref().ok()?;
      Url::parse(page.1.oembed_url.as_deref()?).ok()
    });
    let oembed = match oembed_url {
      Some(endpoint) => self.oembed(&endpoint).await.ok(),
      None => None,
    };

    match (page, oembed) {
      (Ok((final_url, page)), oembed) => Ok(merge(final_url, Some(page), oembed)),
      (Err(_), Some(oembed)) => Ok(merge(url.to_string(), None, Some(oembed))),
      (Err(e), None) => Err(e),
    }
  }

  /// Downloads an image (a preview's `image_url` or `icon_url`) up to the size limit.
  pub async fn image(&self, link: &str) -> Result<Image> {
    let url = guard::check(link, self.config.allow_private)?;
    let response = self.get(url, "image/*").await?;
    let content_type = content_type(&response);
    if !content_type.starts_with("image/") {
      return Err(Error::NotImage(content_type));
    }
    let bytes = read(response, self.config.max_image_bytes, false).await?;
    Ok(Image { bytes, content_type })
  }

  /// Downloads a file (e.g. the hosted rules) up to `limit` bytes. With the `etag` from last time,
  /// the server answers [`Download::NotModified`] without a body if the file hasn't changed.
  pub async fn download(&self, link: &str, etag: Option<&str>, limit: usize) -> Result<Download> {
    let url = guard::check(link, self.config.allow_private)?;
    let mut request = self.client.get(url).header(header::ACCEPT, "*/*");
    if let Some(etag) = etag {
      request = request.header(header::IF_NONE_MATCH, etag);
    }
    let response = request.send().await.map_err(network_error)?;
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
      return Ok(Download::NotModified);
    }
    if !response.status().is_success() {
      return Err(Error::Status(response.status().as_u16()));
    }
    let etag = response.headers().get(header::ETAG).and_then(|v| v.to_str().ok()).map(str::to_string);
    let bytes = read(response, limit, false).await?;
    Ok(Download::Fetched { bytes, etag })
  }

  async fn page(&self, url: &Url) -> Result<(String, html::Page)> {
    let response = self.get(url.clone(), "text/html,application/xhtml+xml").await?;
    let final_url = response.url().clone();
    let content_type = content_type(&response);
    if !(content_type.contains("html") || content_type.is_empty()) {
      return Err(Error::NotHtml(content_type));
    }
    let charset = header_charset(&response);
    let bytes = read(response, self.config.max_page_bytes, true).await?;
    let text = decode(&bytes, charset.as_deref());
    Ok((final_url.to_string(), html::parse_html(&text, &final_url)))
  }

  async fn oembed(&self, endpoint: &Url) -> Result<oembed::OEmbed> {
    guard::check_url(endpoint, self.config.allow_private)?;
    let response = self.get(endpoint.clone(), "application/json").await?;
    oembed::parse(&read(response, self.config.max_json_bytes, false).await?)
  }

  async fn get(&self, url: Url, accept: &str) -> Result<Response> {
    let response = self
      .client
      .get(url)
      .header(header::ACCEPT, accept)
      .send()
      .await
      .map_err(network_error)?;
    if !response.status().is_success() {
      return Err(Error::Status(response.status().as_u16()));
    }
    Ok(response)
  }
}

/// Reads the body up to `limit` bytes. Past it, either stops there (`truncate`, for pages, whose
/// metadata is near the top) or fails.
async fn read(mut response: Response, limit: usize, truncate: bool) -> Result<Vec<u8>> {
  if !truncate && response.content_length().is_some_and(|n| n as usize > limit) {
    return Err(Error::TooLarge(limit));
  }
  let mut body = Vec::new();
  while let Some(chunk) = response.chunk().await.map_err(network_error)? {
    if body.len() + chunk.len() > limit {
      if !truncate {
        return Err(Error::TooLarge(limit));
      }
      body.extend_from_slice(&chunk[..limit - body.len()]);
      break;
    }
    body.extend_from_slice(&chunk);
  }
  Ok(body)
}

fn content_type(response: &Response) -> String {
  response
    .headers()
    .get(header::CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .unwrap_or("")
    .to_ascii_lowercase()
}

fn header_charset(response: &Response) -> Option<String> {
  let content_type = content_type(response);
  content_type.split(';').find_map(|part| {
    let (key, value) = part.trim().split_once('=')?;
    (key.trim() == "charset").then(|| value.trim().trim_matches('"').to_string())
  })
}

/// Bytes to text: the header's charset, else a `<meta charset>` near the top, else UTF-8 (bad
/// bytes become U+FFFD).
pub(crate) fn decode(bytes: &[u8], header_charset: Option<&str>) -> String {
  let label = header_charset.map(str::to_string).or_else(|| meta_charset(bytes));
  let encoding = label
    .and_then(|l| encoding_rs::Encoding::for_label(l.as_bytes()))
    .unwrap_or(encoding_rs::UTF_8);
  encoding.decode(bytes).0.into_owned()
}

/// `<meta charset="…">` or `<meta http-equiv content="…; charset=…">` in the first 2 KB.
fn meta_charset(bytes: &[u8]) -> Option<String> {
  let head = String::from_utf8_lossy(&bytes[..bytes.len().min(2048)]).to_ascii_lowercase();
  let at = head.find("charset=")? + "charset=".len();
  let value: String = head[at..]
    .trim_start_matches(['"', '\''])
    .chars()
    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
    .collect();
  (!value.is_empty()).then_some(value)
}

fn network_error(e: reqwest::Error) -> Error {
  if e.is_timeout() {
    return Error::Timeout;
  }
  // Our own refusals (private address, too many redirects) come back wrapped; find them.
  let mut source: Option<&dyn std::error::Error> = Some(&e);
  while let Some(err) = source {
    if let Some(ours) = err.downcast_ref::<Error>() {
      return match ours {
        Error::Blocked(host) => Error::Blocked(host.clone()),
        other => Error::Network(other.to_string()),
      };
    }
    source = err.source();
  }
  Error::Network(e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn decodes_with_the_header_charset_first() {
    let latin1 = b"<p>Caf\xe9</p>";
    assert_eq!(decode(latin1, Some("iso-8859-1")), "<p>Café</p>");
  }

  #[test]
  fn falls_back_to_the_meta_charset_then_utf8() {
    let page = b"<html><head><meta charset=\"windows-1252\"></head><body>na\xefve</body></html>";
    assert!(decode(page, None).contains("naïve"));
    let page = b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=ISO-8859-1\"><p>\xfc</p>";
    assert!(decode(page, None).contains('ü'));
    assert_eq!(decode("ünï".as_bytes(), None), "ünï");
  }
}
