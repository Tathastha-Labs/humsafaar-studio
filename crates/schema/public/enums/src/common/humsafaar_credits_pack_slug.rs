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
pub enum HumsafaarCreditsPackSlug {
  #[serde(rename= "humsafaar_1000")]
  Humsafaar1000,
  #[serde(rename= "humsafaar_2500")]
  Humsafaar2500,
  #[serde(rename= "humsafaar_5000")]
  Humsafaar5000,
  #[serde(rename= "humsafaar_10000")]
  Humsafaar10000,
  #[serde(rename= "humsafaar_25000")]
  Humsafaar25000,
  #[serde(rename= "humsafaar_50000")]
  Humsafaar50000,
}

impl_enum_display_and_debug_using_to_str!(HumsafaarCreditsPackSlug);
impl_mysql_enum_coders!(HumsafaarCreditsPackSlug);
impl_mysql_from_row!(HumsafaarCreditsPackSlug);

// NB: We can derive `sqlx::Type` instead of using `impl_mysql_enum_coders`

impl HumsafaarCreditsPackSlug {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::Humsafaar1000 => "humsafaar_1000",
      Self::Humsafaar2500 => "humsafaar_2500",
      Self::Humsafaar5000 => "humsafaar_5000",
      Self::Humsafaar10000 => "humsafaar_10000",
      Self::Humsafaar25000 => "humsafaar_25000",
      Self::Humsafaar50000 => "humsafaar_50000",
    }
  }

  pub fn from_str(s: &str) -> Result<Self, String> {
    match s {
      "humsafaar_1000" => Ok(Self::Humsafaar1000),
      "humsafaar_2500" => Ok(Self::Humsafaar2500),
      "humsafaar_5000" => Ok(Self::Humsafaar5000),
      "humsafaar_10000" => Ok(Self::Humsafaar10000),
      "humsafaar_25000" => Ok(Self::Humsafaar25000),
      "humsafaar_50000" => Ok(Self::Humsafaar50000),
      _ => Err(format!("invalid humsafaar_credits_pack_slug: {:?}", s)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    // NB: BTreeSet is sorted
    // NB: BTreeSet::from() isn't const, but not worth using LazyStatic, etc.
    BTreeSet::from([
      Self::Humsafaar1000,
      Self::Humsafaar2500,
      Self::Humsafaar5000,
      Self::Humsafaar10000,
      Self::Humsafaar25000,
      Self::Humsafaar50000,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::test_helpers::assert_serialization;
  use crate::common::humsafaar_credits_pack_slug::HumsafaarCreditsPackSlug;

  mod explicit_checks {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(HumsafaarCreditsPackSlug::Humsafaar1000, "humsafaar_1000");
      assert_serialization(HumsafaarCreditsPackSlug::Humsafaar2500, "humsafaar_2500");
      assert_serialization(HumsafaarCreditsPackSlug::Humsafaar5000, "humsafaar_5000");
      assert_serialization(HumsafaarCreditsPackSlug::Humsafaar10000, "humsafaar_10000");
      assert_serialization(HumsafaarCreditsPackSlug::Humsafaar25000, "humsafaar_25000");
      assert_serialization(HumsafaarCreditsPackSlug::Humsafaar50000, "humsafaar_50000");
    }

    #[test]
    fn to_str() {
      assert_eq!(HumsafaarCreditsPackSlug::Humsafaar1000.to_str(), "humsafaar_1000");
      assert_eq!(HumsafaarCreditsPackSlug::Humsafaar2500.to_str(), "humsafaar_2500");
      assert_eq!(HumsafaarCreditsPackSlug::Humsafaar5000.to_str(), "humsafaar_5000");
      assert_eq!(HumsafaarCreditsPackSlug::Humsafaar10000.to_str(), "humsafaar_10000");
      assert_eq!(HumsafaarCreditsPackSlug::Humsafaar25000.to_str(), "humsafaar_25000");
      assert_eq!(HumsafaarCreditsPackSlug::Humsafaar50000.to_str(), "humsafaar_50000");
    }

    #[test]
    fn from_str() {
      assert_eq!(HumsafaarCreditsPackSlug::from_str("humsafaar_1000").unwrap(), HumsafaarCreditsPackSlug::Humsafaar1000);
      assert_eq!(HumsafaarCreditsPackSlug::from_str("humsafaar_2500").unwrap(), HumsafaarCreditsPackSlug::Humsafaar2500);
      assert_eq!(HumsafaarCreditsPackSlug::from_str("humsafaar_5000").unwrap(), HumsafaarCreditsPackSlug::Humsafaar5000);
      assert_eq!(HumsafaarCreditsPackSlug::from_str("humsafaar_10000").unwrap(), HumsafaarCreditsPackSlug::Humsafaar10000);
      assert_eq!(HumsafaarCreditsPackSlug::from_str("humsafaar_25000").unwrap(), HumsafaarCreditsPackSlug::Humsafaar25000);
      assert_eq!(HumsafaarCreditsPackSlug::from_str("humsafaar_50000").unwrap(), HumsafaarCreditsPackSlug::Humsafaar50000);

    }

    #[test]
    fn all_variants() {
      let mut variants = HumsafaarCreditsPackSlug::all_variants();
      assert_eq!(variants.len(), 6);
      assert_eq!(variants.pop_first(), Some(HumsafaarCreditsPackSlug::Humsafaar1000));
      assert_eq!(variants.pop_first(), Some(HumsafaarCreditsPackSlug::Humsafaar2500));
      assert_eq!(variants.pop_first(), Some(HumsafaarCreditsPackSlug::Humsafaar5000));
      assert_eq!(variants.pop_first(), Some(HumsafaarCreditsPackSlug::Humsafaar10000));
      assert_eq!(variants.pop_first(), Some(HumsafaarCreditsPackSlug::Humsafaar25000));
      assert_eq!(variants.pop_first(), Some(HumsafaarCreditsPackSlug::Humsafaar50000));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;

    #[test]
    fn variant_length() {
      use strum::IntoEnumIterator;
      assert_eq!(HumsafaarCreditsPackSlug::all_variants().len(), HumsafaarCreditsPackSlug::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in HumsafaarCreditsPackSlug::all_variants() {
        // Test to_str(), from_str(), Display, and Debug.
        assert_eq!(variant, HumsafaarCreditsPackSlug::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, HumsafaarCreditsPackSlug::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, HumsafaarCreditsPackSlug::from_str(&format!("{:?}", variant)).unwrap());
      }
    }

    #[test]
    fn serialized_length_ok_for_database() {
      const MAX_LENGTH : usize = 16;
      for variant in HumsafaarCreditsPackSlug::all_variants() {
        let serialized = variant.to_str();
        assert!(serialized.len() > 0, "variant {:?} is too short", variant);
        assert!(serialized.len() <= MAX_LENGTH, "variant {:?} is too long", variant);
      }
    }
  }
}
