//! Which query parameters count as tracking.
//!
//! A pattern ending in `*` matches by prefix. Matching ignores case.
//! Site rules apply to the listed domains and their subdomains, so the same
//! name can be tracking on one site and meaningful on another (e.g. `t` is a
//! share token on X but the start time on YouTube).

/// Tracking parameters removed on every site.
pub const GLOBAL: &[&str] = &[
  // Campaign tags (Google Analytics and friends)
  "utm_*",
  // Ad click IDs
  "fbclid", "gclid", "gclsrc", "dclid", "gbraid", "wbraid", "msclkid", "yclid", "twclid",
  "ttclid", "li_fat_id", "rb_clickid", "s_kwcid",
  // Analytics / email marketing
  "_ga", "_gl", "_hsenc", "_hsmi", "mc_cid", "mc_eid", "mkt_tok", "oly_anon_id", "oly_enc_id",
  "vero_id",
  // Instagram share IDs (also seen on links shared out of Instagram)
  "igshid", "igsh",
];

pub struct SiteRule {
  pub domains: &'static [&'static str],
  pub params: &'static [&'static str],
}

pub const SITES: &[SiteRule] = &[
  SiteRule {
    // `si` identifies who shared the link; keep `v`, `t`, `list`.
    domains: &["youtube.com", "youtu.be"],
    params: &["si", "feature", "pp"],
  },
  SiteRule {
    // `s` = share source; `a` (newer) and `t` (older) = per-user share token.
    domains: &["x.com", "twitter.com"],
    params: &["s", "a", "t", "ref_src", "ref_url"],
  },
  SiteRule {
    domains: &["threads.net", "threads.com"],
    params: &["xmt", "slof"],
  },
  SiteRule {
    domains: &["tiktok.com"],
    params: &[
      "_r", "_t", "is_from_webapp", "is_copy_url", "sender_device", "sender_web_id",
      "share_app_id", "share_item_id", "share_link_id", "social_share_type", "social_sharing",
      "tt_from", "u_code", "user_id", "sec_user_id", "checksum", "timestamp", "enter_from",
      "source", "web_id", "preview_pb",
    ],
  },
  SiteRule {
    domains: &["facebook.com", "fb.watch"],
    params: &["mibextid", "rdid", "share_url", "sfnsn", "__cft__*", "__tn__"],
  },
  SiteRule {
    domains: &["open.spotify.com"],
    params: &["si", "nd", "dlsi"],
  },
  SiteRule {
    domains: &["linkedin.com"],
    params: &["trk", "trackingid", "lipi", "midtoken", "midsig", "trkemail", "eid", "rcm"],
  },
  SiteRule {
    domains: &["reddit.com"],
    params: &["share_id", "rdt", "ref", "ref_source", "ref_campaign"],
  },
];

/// True if `name` is a tracking parameter for a link on `host` (lowercase, no port).
pub fn is_tracking(host: &str, name: &str) -> bool {
  let name = name.to_ascii_lowercase();
  if matches_any(GLOBAL, &name) {
    return true;
  }
  SITES
    .iter()
    .filter(|site| site.domains.iter().any(|d| on_domain(host, d)))
    .any(|site| matches_any(site.params, &name))
}

fn matches_any(patterns: &[&str], name: &str) -> bool {
  patterns.iter().any(|p| match p.strip_suffix('*') {
    Some(prefix) => name.starts_with(prefix),
    None => name == *p,
  })
}

fn on_domain(host: &str, domain: &str) -> bool {
  host == domain
    || host
      .strip_suffix(domain)
      .is_some_and(|rest| rest.ends_with('.'))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Keeps README.md in sync: every rule must be documented there.
  #[test]
  fn every_rule_is_documented_in_readme() {
    let readme = include_str!("../README.md").to_ascii_lowercase();
    let documented = |text: &str| readme.contains(&format!("`{}`", text.to_ascii_lowercase()));

    for param in GLOBAL {
      assert!(documented(param), "README.md is missing every-site parameter `{param}`");
    }
    for site in SITES {
      for domain in site.domains {
        assert!(documented(domain), "README.md is missing domain `{domain}`");
      }
      for param in site.params {
        assert!(documented(param), "README.md is missing `{param}` for {:?}", site.domains);
      }
    }
  }
}
