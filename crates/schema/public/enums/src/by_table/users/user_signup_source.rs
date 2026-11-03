// Deprecated model variants (e.g. the Seedance 2.0 Ultra tier) must keep
// their classification arms here so historical rows still map; suppress the
// deprecation lint file-wide.
#![allow(deprecated)]

use std::collections::BTreeSet;

#[cfg(test)]
use strum::EnumCount;
#[cfg(test)]
use strum::EnumIter;
use utoipa::ToSchema;

/// Used in the `users` table in a `VARCHAR(255)` (which should be a `VARCHAR(16)`) field, `maybe_source`.
///
/// DO NOT CHANGE VALUES WITHOUT A MIGRATION STRATEGY.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserSignupSource {
  #[deprecated(since = "2026-01-30", note = "Use other Humsafaar* enum variants instead")]
  #[serde(rename = "humsafaar")]
  Humsafaar,

  /// Humsafaar Studio Tauri App Onboard Flow
  #[serde(rename = "humsafaar_app")]
  HumsafaarApp,

  /// humsafaar.invalid normal onboard flow
  #[serde(rename = "humsafaar_ai_web")]
  HumsafaarAiWeb,

  /// humsafaar.invalid stripe checkout flow
  #[serde(rename = "humsafaar_ai_s")]
  HumsafaarAiStripe,

  /// humsafaar.invalid normal onboard flow
  #[serde(rename = "hum_get_web")]
  HumsafaarGetWeb,

  /// humsafaar.invalid stripe checkout flow
  #[serde(rename = "hum_get_s")]
  HumsafaarGetStripe,

  #[serde(rename = "fakeyou")]
  FakeYou,
  
  #[serde(rename = "storyteller")]
  Storyteller,
}

// TODO(bt, 2022-12-21): This desperately needs MySQL integration tests!
impl_enum_display_and_debug_using_to_str!(UserSignupSource);
impl_mysql_enum_coders!(UserSignupSource);
impl_mysql_from_row!(UserSignupSource);

/// NB: Legacy API for older code.
impl UserSignupSource {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::Humsafaar => "humsafaar",
      Self::HumsafaarApp => "humsafaar_app",
      Self::HumsafaarAiWeb => "humsafaar_ai_web",
      Self::HumsafaarAiStripe => "humsafaar_ai_s",
      Self::HumsafaarGetWeb => "hum_get_web",
      Self::HumsafaarGetStripe => "hum_get_s",
      Self::FakeYou => "fakeyou",
      Self::Storyteller => "storyteller",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "humsafaar" => Ok(Self::Humsafaar),
      "humsafaar_app" => Ok(Self::HumsafaarApp),
      "humsafaar_ai_web" => Ok(Self::HumsafaarAiWeb),
      "humsafaar_ai_s" => Ok(Self::HumsafaarAiStripe),
      "hum_get_web" => Ok(Self::HumsafaarGetWeb),
      "hum_get_s" => Ok(Self::HumsafaarGetStripe),
      "fakeyou" => Ok(Self::FakeYou),
      "storyteller" => Ok(Self::Storyteller),
      _ => Err(format!("invalid value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    // NB: BTreeSet is sorted
    // NB: BTreeSet::from() isn't const, but not worth using LazyStatic, etc.
    BTreeSet::from([
      Self::Humsafaar,
      Self::HumsafaarApp,
      Self::HumsafaarAiWeb,
      Self::HumsafaarAiStripe,
      Self::HumsafaarGetWeb,
      Self::HumsafaarGetStripe,
      Self::FakeYou,
      Self::Storyteller,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::by_table::users::user_signup_source::UserSignupSource;
  use crate::test_helpers::assert_serialization;

  mod serde {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(UserSignupSource::Humsafaar, "humsafaar");
      assert_serialization(UserSignupSource::HumsafaarApp, "humsafaar_app");
      assert_serialization(UserSignupSource::HumsafaarAiWeb, "humsafaar_ai_web");
      assert_serialization(UserSignupSource::HumsafaarAiStripe, "humsafaar_ai_s");
      assert_serialization(UserSignupSource::HumsafaarGetWeb, "hum_get_web");
      assert_serialization(UserSignupSource::HumsafaarGetStripe, "hum_get_s");
      assert_serialization(UserSignupSource::FakeYou, "fakeyou");
      assert_serialization(UserSignupSource::Storyteller, "storyteller");
    }
  }

  mod impl_methods {
    use super::*;

    #[test]
    fn to_str() {
      assert_eq!(UserSignupSource::Humsafaar.to_str(), "humsafaar");
      assert_eq!(UserSignupSource::HumsafaarApp.to_str(), "humsafaar_app");
      assert_eq!(UserSignupSource::HumsafaarAiWeb.to_str(), "humsafaar_ai_web");
      assert_eq!(UserSignupSource::HumsafaarAiStripe.to_str(), "humsafaar_ai_s");
      assert_eq!(UserSignupSource::HumsafaarGetWeb.to_str(), "hum_get_web");
      assert_eq!(UserSignupSource::FakeYou.to_str(), "fakeyou");
      assert_eq!(UserSignupSource::Storyteller.to_str(), "storyteller");
    }

    #[test]
    fn from_str() {
      assert_eq!(UserSignupSource::from_str("humsafaar").unwrap(), UserSignupSource::Humsafaar);
      assert_eq!(UserSignupSource::from_str("humsafaar_app").unwrap(), UserSignupSource::HumsafaarApp);
      assert_eq!(UserSignupSource::from_str("humsafaar_ai_web").unwrap(), UserSignupSource::HumsafaarAiWeb);
      assert_eq!(UserSignupSource::from_str("humsafaar_ai_s").unwrap(), UserSignupSource::HumsafaarAiStripe);
      assert_eq!(UserSignupSource::from_str("hum_get_web").unwrap(), UserSignupSource::HumsafaarGetWeb);
      assert_eq!(UserSignupSource::from_str("hum_get_s").unwrap(), UserSignupSource::HumsafaarGetStripe);
      assert_eq!(UserSignupSource::from_str("fakeyou").unwrap(), UserSignupSource::FakeYou);
      assert_eq!(UserSignupSource::from_str("storyteller").unwrap(), UserSignupSource::Storyteller);
      assert!(UserSignupSource::from_str("foo").is_err());
    }
  }

  mod manual_variant_checks {
    use super::*;

    #[test]
    fn all_variants() {
      let mut variants = UserSignupSource::all_variants();
      assert_eq!(variants.len(), 8);
      assert_eq!(variants.pop_first(), Some(UserSignupSource::Humsafaar));
      assert_eq!(variants.pop_first(), Some(UserSignupSource::HumsafaarApp));
      assert_eq!(variants.pop_first(), Some(UserSignupSource::HumsafaarAiWeb));
      assert_eq!(variants.pop_first(), Some(UserSignupSource::HumsafaarAiStripe));
      assert_eq!(variants.pop_first(), Some(UserSignupSource::HumsafaarGetWeb));
      assert_eq!(variants.pop_first(), Some(UserSignupSource::HumsafaarGetStripe));
      assert_eq!(variants.pop_first(), Some(UserSignupSource::FakeYou));
      assert_eq!(variants.pop_first(), Some(UserSignupSource::Storyteller));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;

    #[test]
    fn variant_length() {
      use strum::IntoEnumIterator;
      assert_eq!(UserSignupSource::all_variants().len(), UserSignupSource::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in UserSignupSource::all_variants() {
        assert_eq!(variant, UserSignupSource::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, UserSignupSource::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, UserSignupSource::from_str(&format!("{:?}", variant)).unwrap());
      }
    }

    #[test]
    fn serialized_length_ok_for_database() {
      const MAX_LENGTH : usize = 16;
      for variant in UserSignupSource::all_variants() {
        let serialized = variant.to_str();
        assert!(serialized.len() > 0, "variant {:?} is too short", variant);
        assert!(serialized.len() <= MAX_LENGTH, "variant {:?} is too long", variant);
      }
    }
  }
}
