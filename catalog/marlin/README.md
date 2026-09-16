# Marlin

A pairing-based SNARK whose one setup serves every circuit up to a size, instead of one ceremony per circuit. It
followed Sonic, the first SNARK with a universal setup of linear size, and beat it on proving and verifying time; a
batched descendant of it, Varuna, proves Aleo's transactions.

> **Shelf:** Others · **First published:** 2019 · **Trusted setup:** universal and updatable · **Zero-knowledge:** yes ·
> **Post-quantum:** no · **Runs here with:** `ark-marlin`, arkworks' implementation

## What it is

[Groth16](../groth16/README.md) needs a trusted setup for every circuit. Marlin's setup instead produces a **structured
reference string** that works for any circuit up to a maximum size. Anyone can later update it with fresh randomness, and
it stays safe as long as one of the contributors was honest.

Marlin is built in two layers. The first is an **algebraic holographic proof**: a protocol in which the prover sends
polynomials, and the verifier reads only an encoding of the circuit, called the index, instead of the circuit itself.
The second is a **polynomial commitment** in the style of KZG, which turns those polynomials into short commitments and
openings. Indexing a circuit happens once, and after that each proof is quick to check.

A Marlin proof has 13 G1 points and 8 field elements, 880 bytes on BLS12-381, where Groth16's is 192. Verification needs
two pairings.

## History

- **September 2019.** Alessandro Chiesa, Yuncong Hu, Mary Maller, Pratyush Mishra, Psi Vesely and Nicholas Ward post
  Marlin; it appears at EUROCRYPT 2020. Against Sonic, the paper reports about ten times faster proving and three times
  faster verification.
- **June 2021.** `ark-marlin` 0.3.0, its last release. The repository calls it an academic prototype, not ready for
  production, and its last commit is from August 2022.
- **18 September 2024.** Aleo's mainnet launches with Varuna, which Aleo describes as an iteration of Marlin that adds
  batching.
- **July 2025.** Provable adds a round to Varuna's protocol to fix the security of its batching.
- **June 2026.** Veria Labs publishes a proof forgery in snarkVM, Aleo's implementation: its batch check drew random
  weights without first absorbing the witness values. It was reported on 29 April and fixed on 1 May, and Aleo found no
  sign that anyone had used it.

## Strengths

- **One setup for many circuits.** The reference string is universal and anyone can update it.
- **Faster than Sonic**, its universal-setup predecessor, at both proving and verifying.
- **Still constant-size proofs**, under a kilobyte.

## Weaknesses

- **Proofs about four and a half times Groth16's** on the same curve.
- **Still a trusted setup**, universal rather than per circuit.
- **Not post-quantum.** It rests on pairings.
- **Easy to get wrong in code.** Aleo's 2026 forgery came from what a hash did not absorb, not from the paper.

## How next-generation it is

Not much, today. Marlin was one of the 2019 papers that built a SNARK in two separable layers, a protocol about
polynomials and a polynomial commitment, which is how most SNARKs are described now. Since then, pairing SNARKs with a
universal setup have largely moved to the PLONK family, with its custom gates, and much new work has moved to
transparent, hash-based systems.

## Status today (as of September 2026)

**Active, as Varuna.** Aleo's mainnet has proved with Varuna, a batched variant of Marlin, since September 2024.
arkworks' Marlin, which this exhibit runs, is an unmaintained prototype.

## How this repository runs it

`crates/sys-marlin` uses `ark-marlin` 0.3.0 with arkworks' KZG-style commitment over BLS12-381. Setup runs in two real
steps: a universal reference string sized for the circuit, then the index for that circuit.

Every statement's proof is 951 bytes in arkworks' encoding, from one-plus-one to pool-spend. Each run makes a reference
string sized for its circuit, about 80 KB for the small statements and about 5 MB for pool-spend, and indexing turns it
into keys of 0.6 MB to 40 MB; one string made for the largest circuit could serve all seven. Flipping the museum's byte
leaves a valid number in one of the proof's claimed evaluations, so the proof still decodes and the verifier's own checks
reject it. In builds with debug assertions arkworks' prover stops at a false claim; in a release build it produces a
proof, and the verifier rejects that.

Try `/run marlin one-plus-one`, then `/run groth16 one-plus-one`.

## Sources

- A. Chiesa, Y. Hu, M. Maller, P. Mishra, P. Vesely, N. Ward, *Marlin: Preprocessing zkSNARKs with Universal and Updatable SRS*, EUROCRYPT 2020 — https://eprint.iacr.org/2019/1047
- arkworks Marlin — https://github.com/arkworks-rs/marlin
- Aleo, *Announcing Aleo Mainnet* (2024) — https://aleo.org/post/announcing-aleo-mainnet/
- Provable, *Updates to Aleo records and Varuna* (2025) — https://provable.com/blog/updates-to-aleo-records-and-varuna
- Veria Labs, *Forging transactions on Aleo* (2026) — https://verialabs.com/blog/forging-transactions-on-aleo
- snarkVM — https://github.com/ProvableHQ/snarkVM
