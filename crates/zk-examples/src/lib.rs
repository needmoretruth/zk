//! The seven statements every proof system in the museum proves.
//!
//! Each statement is written once over any [`ZkField`], with a deterministic honest assignment
//! (a true claim with a valid witness) and a dishonest one (a false claim that violates at least
//! one assertion), so every system can be shown accepting the first and rejecting the second.
//! [`ExampleId`] is the stable handle a program uses to pick one.

pub mod age;
pub mod factoring;
pub mod membership;
pub mod merkle_tree;
pub mod one_plus_one;
pub mod password;
pub mod pool;
pub mod sudoku;

use zk_circuit::{Assignment, Circuit, CircuitError, ZkField};

/// One of the seven statements.
///
/// The string IDs are permanent: they appear in commands, saved results and documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExampleId {
    /// `one-plus-one`: the answer sealed in an envelope is 1 + 1.
    OnePlusOne,
    /// `password`: I know the PIN behind this digest.
    Password,
    /// `sudoku`: I solved this 4×4 sudoku.
    Sudoku,
    /// `age`: I am at least 18.
    Age,
    /// `membership`: I am one of 16 members, without saying which.
    Membership,
    /// `factoring`: I know two non-trivial factors of `n`.
    Factoring,
    /// `pool-spend`: a Toy Shielded Pool spend.
    PoolSpend,
}

impl ExampleId {
    /// All seven, in teaching order.
    pub const ALL: [ExampleId; 7] = [
        Self::OnePlusOne,
        Self::Password,
        Self::Sudoku,
        Self::Age,
        Self::Membership,
        Self::Factoring,
        Self::PoolSpend,
    ];

    /// The permanent string ID.
    pub fn id(self) -> &'static str {
        match self {
            Self::OnePlusOne => one_plus_one::ID,
            Self::Password => password::ID,
            Self::Sudoku => sudoku::ID,
            Self::Age => age::ID,
            Self::Membership => membership::ID,
            Self::Factoring => factoring::ID,
            Self::PoolSpend => pool::spend::ID,
        }
    }

    /// Looks an example up by its string ID.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|example| example.id() == id)
    }

    /// Public input names in declaration order.
    pub fn public_input_names(self) -> Vec<String> {
        match self {
            Self::OnePlusOne => one_plus_one::public_input_names(),
            Self::Password => password::public_input_names(),
            Self::Sudoku => sudoku::public_input_names(),
            Self::Age => age::public_input_names(),
            Self::Membership => membership::public_input_names(),
            Self::Factoring => factoring::public_input_names(),
            Self::PoolSpend => pool::spend::public_input_names(),
        }
    }

    /// Private input names in declaration order.
    pub fn private_input_names(self) -> Vec<String> {
        match self {
            Self::OnePlusOne => one_plus_one::private_input_names(),
            Self::Password => password::private_input_names(),
            Self::Sudoku => sudoku::private_input_names(),
            Self::Age => age::private_input_names(),
            Self::Membership => membership::private_input_names(),
            Self::Factoring => factoring::private_input_names(),
            Self::PoolSpend => pool::spend::private_input_names(),
        }
    }

    /// The circuit over `F`; fails for fields whose modulus does not exceed 2^30.
    pub fn circuit<F: ZkField>(self) -> Result<Circuit<F>, CircuitError> {
        match self {
            Self::OnePlusOne => one_plus_one::circuit(),
            Self::Password => password::circuit(),
            Self::Sudoku => sudoku::circuit(),
            Self::Age => age::circuit(),
            Self::Membership => membership::circuit(),
            Self::Factoring => factoring::circuit(),
            Self::PoolSpend => pool::spend::circuit(),
        }
    }

    /// A true claim with a valid witness.
    pub fn honest<F: ZkField>(self) -> Assignment<F> {
        match self {
            Self::OnePlusOne => one_plus_one::honest(),
            Self::Password => password::honest(),
            Self::Sudoku => sudoku::honest(),
            Self::Age => age::honest(),
            Self::Membership => membership::honest(),
            Self::Factoring => factoring::honest(),
            Self::PoolSpend => pool::spend::honest(),
        }
    }

    /// A false claim; evaluating it violates at least one assertion.
    pub fn dishonest<F: ZkField>(self) -> Assignment<F> {
        match self {
            Self::OnePlusOne => one_plus_one::dishonest(),
            Self::Password => password::dishonest(),
            Self::Sudoku => sudoku::dishonest(),
            Self::Age => age::dishonest(),
            Self::Membership => membership::dishonest(),
            Self::Factoring => factoring::dishonest(),
            Self::PoolSpend => pool::spend::dishonest(),
        }
    }
}

/// Field elements from integers, for writing sample assignments compactly.
fn values<F: ZkField>(integers: &[u64]) -> Vec<F> {
    integers.iter().map(|value| F::from_u64(*value)).collect()
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_string()).collect()
}
