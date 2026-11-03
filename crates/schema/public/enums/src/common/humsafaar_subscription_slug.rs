use std::collections::BTreeSet;

#[cfg(test)]
use strum::EnumCount;
#[cfg(test)]
use strum::EnumIter;
use utoipa::ToSchema;

/// NB: This will be used by a variety of tables (MySQL and sqlite)!
/// Keep the max length to 16 characters.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HumsafaarSubscriptionSlug {
  HumsafaarBasic,
  HumsafaarPro,
  HumsafaarMax,
}

impl_enum_display_and_debug_using_to_str!(HumsafaarSubscriptionSlug);
impl_mysql_enum_coders!(HumsafaarSubscriptionSlug);
impl_mysql_from_row!(HumsafaarSubscriptionSlug);

// NB: We can derive `sqlx::Type` instead of using `impl_mysql_enum_coders`

impl HumsafaarSubscriptionSlug {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::HumsafaarBasic => "humsafaar_basic",
      Self::HumsafaarPro => "humsafaar_pro",
      Self::HumsafaarMax => "humsafaar_max",
    }
  }

  pub fn from_str(s: &str) -> Result<Self, String> {
    match s {
      "humsafaar_basic" => Ok(Self::HumsafaarBasic),
      "humsafaar_pro" => Ok(Self::HumsafaarPro),
      "humsafaar_max" => Ok(Self::HumsafaarMax),
      _ => Err(format!("invalid humsafaar_subscription_slug: {:?}", s)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    // NB: BTreeSet is sorted
    // NB: BTreeSet::from() isn't const, but not worth using LazyStatic, etc.
    BTreeSet::from([
      Self::HumsafaarBasic,
      Self::HumsafaarPro,
      Self::HumsafaarMax,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::common::humsafaar_subscription_slug::HumsafaarSubscriptionSlug;
  use crate::test_helpers::assert_serialization;

  mod explicit_checks {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(HumsafaarSubscriptionSlug::HumsafaarBasic, "humsafaar_basic");
      assert_serialization(HumsafaarSubscriptionSlug::HumsafaarPro, "humsafaar_pro");
      assert_serialization(HumsafaarSubscriptionSlug::HumsafaarMax, "humsafaar_max");
    }

    #[test]
    fn to_str() {
      assert_eq!(HumsafaarSubscriptionSlug::HumsafaarBasic.to_str(), "humsafaar_basic");
      assert_eq!(HumsafaarSubscriptionSlug::HumsafaarPro.to_str(), "humsafaar_pro");
      assert_eq!(HumsafaarSubscriptionSlug::HumsafaarMax.to_str(), "humsafaar_max");
    }

    #[test]
    fn from_str() {
      assert_eq!(HumsafaarSubscriptionSlug::from_str("humsafaar_basic").unwrap(), HumsafaarSubscriptionSlug::HumsafaarBasic);
      assert_eq!(HumsafaarSubscriptionSlug::from_str("humsafaar_pro").unwrap(), HumsafaarSubscriptionSlug::HumsafaarPro);
      assert_eq!(HumsafaarSubscriptionSlug::from_str("humsafaar_max").unwrap(), HumsafaarSubscriptionSlug::HumsafaarMax);
    }

    #[test]
    fn all_variants() {
      let mut variants = HumsafaarSubscriptionSlug::all_variants();
      assert_eq!(variants.len(), 3);
      assert_eq!(variants.pop_first(), Some(HumsafaarSubscriptionSlug::HumsafaarBasic));
      assert_eq!(variants.pop_first(), Some(HumsafaarSubscriptionSlug::HumsafaarPro));
      assert_eq!(variants.pop_first(), Some(HumsafaarSubscriptionSlug::HumsafaarMax));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;

    #[test]
    fn variant_length() {
      use strum::IntoEnumIterator;
      assert_eq!(HumsafaarSubscriptionSlug::all_variants().len(), HumsafaarSubscriptionSlug::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in HumsafaarSubscriptionSlug::all_variants() {
        // Test to_str(), from_str(), Display, and Debug.
        assert_eq!(variant, HumsafaarSubscriptionSlug::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, HumsafaarSubscriptionSlug::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, HumsafaarSubscriptionSlug::from_str(&format!("{:?}", variant)).unwrap());
      }
    }

    #[test]
    fn serialized_length_ok_for_database() {
      const MAX_LENGTH : usize = 16;
      for variant in HumsafaarSubscriptionSlug::all_variants() {
        let serialized = variant.to_str();
        assert!(serialized.len() > 0, "variant {:?} is too short", variant);
        assert!(serialized.len() <= MAX_LENGTH, "variant {:?} is too long", variant);
      }
    }
  }
}
