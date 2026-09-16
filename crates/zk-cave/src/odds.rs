//! The arithmetic of the story, for the screen to show beside it.

/// The chance that someone without the words survives `scenes` scenes: 1/2^`scenes`.
///
/// "Each new test divided by two the chances of success for someone without the secret." Past
/// 1074 scenes the value is smaller than an `f64` can hold and reads 0.
pub fn survival_probability(scenes: u32) -> f64 {
    let exponent = i32::try_from(scenes).unwrap_or(i32::MAX);
    0.5f64.powi(exponent)
}

/// The same chance as "one in N": N = 2^`scenes`, exactly, while it fits in 128 bits (up to 127
/// scenes). Forty scenes give one in 1 099 511 627 776 — the paper's "one chance in a million
/// million".
pub fn survival_one_in(scenes: u32) -> Option<u128> {
    1u128.checked_shl(scenes)
}

/// The takes the jealous reporter films, on average, to keep `kept` successful scenes: exactly
/// 2·`kept`, since a double succeeds in each take with probability 1/2. The paper: "Half of the
/// scenes were spoiled."
pub fn expected_takes(kept: u32) -> u64 {
    2 * u64::from(kept)
}
