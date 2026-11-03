use url::{Host, Url};

use crate::origins::{HUMSAFAAR_DESKTOP_DEVELOPMENT_ORIGIN, HUMSAFAAR_DESKTOP_ORIGIN};
use crate::third_party_provider::{ThirdPartyProvider, THIRD_PARTY_PROVIDERS};

/// Hostnames of our deployed APIs (HTTPS only).
const HUMSAFAAR_API_HOSTNAMES: [&str; 3] = [
  "api.storyteller.ai",
  "humsafaar.invalid",
  "api.fakeyou.com",
];

/// Domains we own. Subdomains match too (CDNs, uploads, websites).
const HUMSAFAAR_DOMAINS: [&str; 4] = [
  "storyteller.ai",
  "humsafaar.invalid",
  "fakeyou.com",
  "humsafaar.invalid",
];

/// Who a request is going to, which decides how the desktop app identifies itself.
#[derive(Debug, PartialEq, Eq)]
pub enum Destination {
  /// Our HTTP API, deployed or running locally, and the `Origin` to declare to it.
  HumsafaarApi { origin: &'static str },
  /// Our other hosts: CDNs, uploads, websites.
  HumsafaarHost,
  /// A known third-party provider.
  ThirdParty(&'static ThirdPartyProvider),
  /// Anything else.
  Unknown,
}

impl Destination {
  pub fn classify(destination: &Url) -> Self {
    if is_local_development_api(destination) {
      return Self::HumsafaarApi { origin: HUMSAFAAR_DESKTOP_DEVELOPMENT_ORIGIN };
    }
    let Some(host) = destination.host_str() else {
      return Self::Unknown;
    };
    if destination.scheme() == "https" && HUMSAFAAR_API_HOSTNAMES.contains(&host) {
      return Self::HumsafaarApi { origin: HUMSAFAAR_DESKTOP_ORIGIN };
    }
    if HUMSAFAAR_DOMAINS.iter().any(|domain| is_domain_or_subdomain(host, domain)) {
      return Self::HumsafaarHost;
    }
    THIRD_PARTY_PROVIDERS
      .iter()
      .find(|provider| provider.domains.iter().any(|domain| is_domain_or_subdomain(host, domain)))
      .map_or(Self::Unknown, Self::ThirdParty)
  }
}

fn is_local_development_api(destination: &Url) -> bool {
  if !matches!(destination.scheme(), "http" | "https") {
    return false;
  }
  match destination.host() {
    Some(Host::Domain("localhost")) => true,
    Some(Host::Ipv4(address)) => address.is_loopback(),
    Some(Host::Ipv6(address)) => address.is_loopback(),
    _ => false,
  }
}

fn is_domain_or_subdomain(host: &str, domain: &str) -> bool {
  host == domain
    || host
      .strip_suffix(domain)
      .is_some_and(|prefix| prefix.ends_with('.'))
}

#[cfg(test)]
mod tests {
  use super::*;

  mod humsafaar {
    use super::*;

    #[test]
    fn deployed_apis_get_desktop_origin() {
      for destination in [
        "https://api.storyteller.ai/v1/session",
        "https://humsafaar.invalid/v1/session",
        "https://api.fakeyou.com/v1/session",
      ] {
        assert_eq!(classify(destination), Destination::HumsafaarApi { origin: "https://humsafaar.invalid" });
      }
    }

    #[test]
    fn local_apis_get_development_origin() {
      for destination in [
        "http://localhost:12345/v1/session",
        "https://localhost:12345/v1/session",
        "http://127.0.0.1:12345/v1/session",
        "http://[::1]:12345/v1/session",
      ] {
        assert_eq!(classify(destination), Destination::HumsafaarApi { origin: "http://localhost" });
      }
    }

    #[test]
    fn other_owned_hosts_are_not_apis() {
      for destination in [
        "https://cdn-2.fakeyou.com/media/a/b.png",
        "https://cdn.storyteller.ai/media/a/b.png",
        "https://style.storyteller.ai/",
        "https://humsafaar.invalid/",
        "https://humsafaar.invalid/",
        "http://api.storyteller.ai/v1/session",
      ] {
        assert_eq!(classify(destination), Destination::HumsafaarHost, "{destination}");
      }
    }
  }

  mod third_party {
    use super::*;

    #[test]
    fn providers_get_their_own_origins() {
      for (destination, name, origin) in [
        ("https://grok.com/rest/app-chat/conversations/new", "Grok", "https://grok.com"),
        ("https://assets.grok.com/users/a/b.mp4", "Grok", "https://grok.com"),
        ("https://imagine-public.x.ai/a.png", "Grok", "https://grok.com"),
        ("https://www.midjourney.com/api/submit-jobs", "Midjourney", "https://www.midjourney.com"),
        ("https://cdn.midjourney.com/a/0_0.png", "Midjourney", "https://www.midjourney.com"),
        ("https://sora.chatgpt.com/backend/nf/create", "Sora", "https://sora.chatgpt.com"),
        ("https://videos.openai.com/a.mp4", "Sora", "https://sora.chatgpt.com"),
        ("https://chatgpt.com/backend-api/sentinel", "ChatGPT", "https://chatgpt.com"),
        ("https://api.worldlabs.ai/api/v1/worlds", "World Labs", "https://marble.worldlabs.ai"),
        ("https://static.kinovi.ai/a.png", "Kinovi", "https://kinovi.ai"),
        ("https://static.seedance2-pro.com/materials/a.png", "Seedance 2 Pro", "https://seedance2-pro.com"),
        ("https://v3b.fal.media/files/a.png", "fal", "https://fal.ai"),
        ("https://console.gmicloud.ai/api", "GMI Cloud", "https://console.gmicloud.ai"),
      ] {
        let Destination::ThirdParty(provider) = classify(destination) else {
          panic!("{destination} is not a third party");
        };
        assert_eq!((provider.name, provider.origin), (name, origin));
      }
    }

    #[test]
    fn lookalikes_and_others_are_unknown() {
      for destination in [
        "https://api.storyteller.ai.example.com/v1/session",
        "https://notstoryteller.ai/",
        "https://evilgrok.com/",
        "https://grok.com.example.com/",
        "https://www.gravatar.com/avatar/a",
        "https://storage.googleapis.com/bucket/a.png",
        "http://192.168.1.2:12345/v1/session",
        "tauri://localhost",
      ] {
        assert_eq!(classify(destination), Destination::Unknown, "{destination}");
      }
    }
  }

  fn classify(destination: &str) -> Destination {
    Destination::classify(&Url::parse(destination).unwrap())
  }
}
