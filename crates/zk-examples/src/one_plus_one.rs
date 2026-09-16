//! `one-plus-one`: the answer sealed in this envelope is 1 + 1.
//!
//! The proof never contains 2; the verifier only sees `envelope = ToyHash(answer, salt)`. Because
//! the right answer is common knowledge, a verifier who accepts can infer it; that is the statement
//! itself, not a leak.

use zk_circuit::toyhash::{toyhash, toyhash_gadget};
use zk_circuit::{Assignment, Circuit, CircuitBuilder, CircuitError, LinearCombination, ZkField};

/// Permanent ID.
pub const ID: &str = "one-plus-one";

const SALT: u64 = 8_675_309;

/// `envelope`.
pub fn public_input_names() -> Vec<String> {
    crate::names(&["envelope"])
}

/// `answer`, `salt`.
pub fn private_input_names() -> Vec<String> {
    crate::names(&["answer", "salt"])
}

/// Asserts `answer = 1 + 1` and `ToyHash(answer, salt) = envelope`.
pub fn circuit<F: ZkField>() -> Result<Circuit<F>, CircuitError> {
    let mut builder = CircuitBuilder::<F>::new()?;
    let envelope = builder.public_input("envelope");
    let answer = builder.private_input("answer");
    let salt = builder.private_input("salt");
    let one = builder.constant(F::one());
    let two = builder.linear(LinearCombination::<F>::from(one) + one);
    builder.assert_zero(LinearCombination::<F>::from(answer) - two, "answer equals 1 + 1");
    let sealed = toyhash_gadget(&mut builder, answer, salt);
    builder
        .assert_zero(LinearCombination::<F>::from(sealed) - envelope, "envelope seals the answer");
    builder.finish()
}

fn sealed<F: ZkField>(answer: u64) -> Assignment<F> {
    let [answer, salt] = [answer, SALT].map(F::from_u64);
    Assignment { public: vec![toyhash(answer, salt)], private: vec![answer, salt] }
}

/// Answer 2 in its envelope.
pub fn honest<F: ZkField>() -> Assignment<F> {
    sealed(2)
}

/// Answer 3, sealed in its own envelope: the envelope is consistent, the arithmetic is not.
pub fn dishonest<F: ZkField>() -> Assignment<F> {
    sealed(3)
}
