//! The smoke detector: does a secret appear, byte for byte, inside a proof?
//!
//! Finding nothing does not prove zero knowledge — a proof can leak through arithmetic without ever
//! containing a secret verbatim. Finding something does prove a leak, which is exactly what a
//! system that is succinct but not hiding should be caught doing.

use zk_examples::ExampleId;

use crate::report::SecretScan;
use crate::system::{FieldBytes, Prepared};

/// Searches `haystack` for every non-trivial secret under every encoding `prepared` declares.
pub fn scan_secrets(
    prepared: &dyn Prepared,
    example: ExampleId,
    secrets: &[FieldBytes],
    haystack: &[u8],
) -> SecretScan {
    let names = example.private_input_names();
    let mut searched_any = false;
    let mut found = Vec::new();
    for (index, secret) in secrets.iter().enumerate() {
        if is_trivial(secret) {
            continue;
        }
        searched_any = true;
        let hit = prepared
            .secret_encodings(secret)
            .iter()
            .any(|pattern| !pattern.is_empty() && contains(haystack, pattern));
        if hit {
            let name =
                names.get(index).cloned().unwrap_or_else(|| format!("private input {index}"));
            if !found.contains(&name) {
                found.push(name);
            }
        }
    }
    match (searched_any, found.is_empty()) {
        (false, _) => SecretScan::Inconclusive,
        (true, true) => SecretScan::NotFound,
        (true, false) => SecretScan::Found(found),
    }
}

/// Values below 2^16 are not searched: a match would prove nothing.
///
/// Such a value is at most two non-zero bytes followed by zeros, and proofs are full of zeros next to
/// small numbers — zero evaluations, padding, length and count fields. Measured on this museum's
/// exhibits, a sudoku cell or a factor of 181 turned up by chance in proofs that hide them perfectly
/// well. A value of three or more significant bytes next to such a run matches by chance about once
/// in 2^24 tries, so a hit on it is evidence. An example whose secrets are all small is reported as
/// [`SecretScan::Inconclusive`] rather than clean.
fn is_trivial(secret: &FieldBytes) -> bool {
    secret.iter().skip(2).all(|byte| *byte == 0)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}
