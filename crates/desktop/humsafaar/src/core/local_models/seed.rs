use uuid_utils::uuid::generate_random_uuid;

/// A random seed, so repeated runs of the same prompt differ.
pub fn random_seed() -> u64 {
  let hex: String = generate_random_uuid().chars().filter(|c| c.is_ascii_hexdigit()).take(15).collect();
  u64::from_str_radix(&hex, 16).unwrap_or(0)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn seeds_differ() {
    assert_ne!(random_seed(), random_seed());
  }
}
