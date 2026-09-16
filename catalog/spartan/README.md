# Spartan

A zkSNARK for arithmetic circuits built from the sum-check protocol, with no trusted setup. Published in 2019 by
Srinath Setty of Microsoft Research, its approach now sits inside provers from World's ProveKit to Jolt and Binius64.

> **Shelf:** Others · **First published:** 2019 · **Trusted setup:** none · **Zero-knowledge:** yes ·
> **Post-quantum:** not as implemented here (discrete log) · **Runs here with:** `spartan`, Microsoft's own crate

## What it is

An R1CS instance says: for matrices `A, B, C` and an assignment `z`, every row satisfies `(A·z) × (B·z) = (C·z)`.
Spartan writes the rows' errors as a multilinear polynomial over the boolean hypercube and multiplies it by a random
polynomial, so that one sum over the hypercube is zero exactly when every row is satisfied (except with negligible
probability). The **sum-check protocol** then reduces "this sum is zero" to evaluating a few polynomials at one random
point, which the prover proves with a polynomial commitment.

Two more ideas make it succinct and fast. **Computation commitments** preprocess the matrices once, so the verifier does
not read the whole circuit on every proof. **SPARK** turns a commitment scheme for dense multilinear polynomials into one
for sparse ones, so the prover's work stays linear in the number of non-zero entries.

The polynomial commitment is interchangeable. The original library uses Hyrax-style commitments over ristretto255, which
need no setup and rest on discrete logarithms; later systems plug in KZG or hash-based commitments such as WHIR.

## History

- **23 May 2019.** Srinath Setty posts Spartan, a family of zkSNARKs with no trusted setup, sub-linear verification and a
  prover whose time is linear in the circuit; it appears at CRYPTO 2020.
- **July 2023.** Spartan2, a newer implementation, starts; its repository is later renamed for Vega.
- **January 2025.** Setty and Thaler's Twist and Shout introduces SpeedySpartan for PLONK-style constraints and
  Spartan++, which proves about six times faster than Spartan.
- **2026.** Vega, by Kaviani and Setty, proves an age fact from a mobile driver's licence in about 92 ms on a commodity
  client device.
- **2 September 2026.** World describes ProveKit — Noir circuits proved with Spartan and WHIR — as production-ready.

## Strengths

- **No trusted setup.**
- **A fast prover**, linear in the size of the circuit, with no FFTs.
- **Flexible.** The sum-check core works with any multilinear commitment: discrete-log, pairing-based or hash-based,
  and so can be made plausibly post-quantum.

## Weaknesses

- **Big proofs** compared with pairing SNARKs: tens to hundreds of kilobytes instead of a few hundred bytes. At a million
  constraints the original library's proofs are 48 KB (NIZK) and 142 KB (SNARK), against 128 bytes for Groth16.
- **Unaudited reference code.** The `spartan` crate's README says it has not received a security review or audit.
- **Not post-quantum as implemented here.** The Hyrax-style commitments rest on discrete logarithms.

## How next-generation it is

Very much current. Sum-check-based proving is one of the main directions of the 2020s, and Spartan was the design that
showed it could be practical for general circuits. Its ideas are used in Jolt's zero-knowledge mode, in Binius64's
zero-knowledge wrapper and in ProveKit.

## Status today (as of September 2026)

**Active through its descendants.** The original `spartan` crate is still maintained; Vega and ProveKit carry the design
into production-oriented systems.

## How this repository runs it

`crates/sys-spartan` uses the `spartan` crate 0.9.0 from Microsoft over ristretto255. The museum's R1CS is handed to it
as sparse `A, B, C` matrices, with the public inputs in Spartan's input slot so the verifier depends on them, and padded to
the powers of two Spartan requires.

Proofs here run from 23,488 bytes for `one-plus-one` to 50,520 bytes for `pool-spend`. The exhibit uses Spartan's
`SNARK` mode, in which the matrices are committed once so the verifier does not reread them; its `NIZK` mode hides the
witness just as well but makes the verifier read the whole circuit.

Try `/run spartan pool-spend`, then `/run bulletproofs pool-spend`: two discrete-log systems without a setup, with very
different costs.

## Sources

- S. Setty, *Spartan: Efficient and general-purpose zkSNARKs without trusted setup*, CRYPTO 2020 — https://eprint.iacr.org/2019/550
- Spartan library — https://github.com/microsoft/Spartan
- Spartan2, now Vega — https://github.com/microsoft/vega-prover
- S. Setty, J. Thaler, *Twist and Shout* — https://eprint.iacr.org/2025/105
- D. Kaviani, S. Setty, *Vega* — https://eprint.iacr.org/2025/2094
- World, *ProveKit: privacy for the real world* — https://world.org/blog/engineering/provekit-privacy-for-the-real-world
- a16z crypto, *Jolt zero-knowledge* — https://a16zcrypto.com/posts/article/zkvm-jolt-zero-knowledge/
- Binius64 ZK prover configuration — https://github.com/binius-zk/binius64/blob/main/crates/prover/src/zk_config.rs
