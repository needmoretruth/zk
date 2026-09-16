//! `password`: I know the 6-digit PIN behind this digest.
//!
//! Zero knowledge does not protect a guessable secret: anyone can hash all million PINs and compare
//! with the public digest. The museum uses that brute force as this example's attack step.

use zk_circuit::gadgets::range_check;
use zk_circuit::toyhash::{toyhash, toyhash_gadget};
use zk_circuit::{Assignment, Circuit, CircuitBuilder, CircuitError, LinearCombination, ZkField};

/// Permanent ID.
pub const ID: &str = "password";

/// Bits a 6-digit PIN needs: 999 999 < 2^20.
pub const PIN_BITS: u32 = 20;

const HONEST_PIN: u64 = 314_159;
const WRONG_PIN: u64 = 271_828;

/// `digest`.
pub fn public_input_names() -> Vec<String> {
    crate::names(&["digest"])
}

/// `pin`.
pub fn private_input_names() -> Vec<String> {
    crate::names(&["pin"])
}

/// Asserts `pin < 2^20` and `ToyHash(pin, 0) = digest`.
pub fn circuit<F: ZkField>() -> Result<Circuit<F>, CircuitError> {
    let mut builder = CircuitBuilder::<F>::new()?;
    let digest = builder.public_input("digest");
    let pin = builder.private_input("pin");
    range_check(&mut builder, pin, PIN_BITS, "pin fits in 20 bits")?;
    let zero = builder.constant(F::zero());
    let hashed = toyhash_gadget(&mut builder, pin, zero);
    builder.assert_zero(
        LinearCombination::<F>::from(hashed) - digest,
        "digest is the hash of the pin",
    );
    builder.finish()
}

fn digest<F: ZkField>(pin: u64) -> F {
    toyhash(F::from_u64(pin), F::zero())
}

/// The right PIN for its digest.
pub fn honest<F: ZkField>() -> Assignment<F> {
    Assignment { public: vec![digest(HONEST_PIN)], private: vec![F::from_u64(HONEST_PIN)] }
}

/// A wrong PIN against the honest digest.
pub fn dishonest<F: ZkField>() -> Assignment<F> {
    Assignment { public: vec![digest(HONEST_PIN)], private: vec![F::from_u64(WRONG_PIN)] }
}
