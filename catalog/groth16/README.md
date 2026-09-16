# Groth16

A pairing-based zk-SNARK whose proof is three group elements. It is the proof system behind Zcash's Sapling pool
and, a decade after it was published, still the smallest and fastest-to-verify proof in wide use.

> **Shelf:** Zcash · **First published:** 2016 · **Trusted setup:** per circuit · **Zero-knowledge:** yes ·
> **Post-quantum:** no · **Runs here with:** `bellman`, the crate Zcash itself uses

## What it is

A prover who knows a witness for an arithmetic circuit convinces a verifier with a proof of exactly three elliptic-curve
points: two in the group G1 and one in G2. On BLS12-381 that is 192 bytes. The verifier checks a single equation
between pairings, so checking a proof takes the same time whether the circuit has ten constraints or ten million.

The circuit is written as a rank-1 constraint system (R1CS) and turned into polynomials (a quadratic arithmetic
program). Before anyone can prove anything, a **setup** for that exact circuit produces a proving key and a
verifying key from secret random numbers. Those numbers — often called toxic waste — must be destroyed: whoever keeps
them can forge proofs for false statements. The proofs themselves still reveal nothing about the witness either way.

## History

- **2016.** Jens Groth publishes *On the Size of Pairing-based Non-interactive Arguments* (EUROCRYPT 2016). It
  shortens the pairing-based SNARKs of the time — Pinocchio and its BCTV14 variant used eight group
  elements — to three.
- **2017–2018.** To use Groth16 without trusting any single person with the toxic waste, the Zcash community runs a
  multi-party ceremony in two phases. The first, *Powers of Tau*, collected 87 contributions between November 2017
  and April 2018; the second produced the keys for the Sapling circuits. The keys are safe if even one participant
  destroyed their secret.
- **28 October 2018.** Zcash activates Sapling at block 419,200. Shielded spends and outputs are proved with
  Groth16 over BLS12-381, using the Rust crate `bellman`. Sprout's older JoinSplit proofs move to Groth16 at the same
  moment, which also closes a counterfeiting flaw in the previous proof system (see [BCTV14](../bctv14/README.md)).
- **Since then** it has become the usual "last step" for newer systems: RISC Zero, for example, wraps its STARK
  proofs in a Groth16 proof over BN254 so that a blockchain only has to check three points.

## Strengths

- **The smallest proofs.** Three group elements, whatever the circuit.
- **The fastest verification.** A handful of pairings and a short sum over the public inputs.
- **Mature.** Years of production use holding real value, several independent implementations, and a well-studied
  security proof (in the generic group model).
- **A good wrapper.** Systems with large proofs can prove "I checked that proof" inside a Groth16 circuit and ship
  the small proof instead.

## Weaknesses

- **A new ceremony for every circuit.** Change one constraint and the keys are useless. Upgrading a Groth16 system
  means organising a new trusted setup.
- **Toxic waste.** If every participant of a setup kept their secret, or a one-person setup was compromised, false
  proofs become possible — and nobody could tell from the proofs.
- **Not post-quantum.** Pairings rely on discrete-logarithm-type assumptions that a large quantum computer would
  break.
- **Heavy provers.** Proving is dominated by large multi-scalar multiplications; big circuits need a lot of memory.
- **Malleable proofs.** Anyone can re-randomise a valid proof into a different-looking valid proof for the same
  statement without the witness. Systems that need a proof to be unique must bind it to something else.

## How next-generation it is

Not very, by design: it is a 2016 construction that fixed the circuit into its keys. Newer systems remove the
per-circuit setup (PLONK with a universal setup, Halo 2 and STARKs with none) or aim for post-quantum security
(hash-based and lattice-based proofs). Yet Groth16's small proofs keep it at the end of many next-generation
pipelines, as the format a blockchain actually verifies. Its long-term future depends on how soon pairings have to be
retired for post-quantum reasons.

## Status today (as of September 2026)

**Active.** Zcash's Sapling pool held about 525,000 ZEC at the end of August 2026, and no Zcash Improvement Proposal
deprecating it was found. Zcash's newest pools (Orchard, and Ironwood since July 2026) use Halo 2 instead. Outside Zcash,
Groth16 remains the final wrapper of RISC Zero proofs and the proof system of the iden3 identity protocol.

## How this repository runs it

`crates/sys-groth16` uses `bellman` and `bls12_381`, the crates Zcash's Sapling prover is built on. Each example
circuit is lowered to R1CS and synthesized through bellman's constraint API. The setup runs on your machine with fresh
random secrets that are dropped when it finishes — a one-person ceremony, so you are trusting your own computer, not
Zcash's participants.

Try `/run groth16 one-plus-one`, then `/run all one-plus-one` to compare it with everything else.

This repository is not affiliated with Zcash, the Electric Coin Company or the Zcash Foundation.

## Sources

- J. Groth, *On the Size of Pairing-based Non-interactive Arguments*, EUROCRYPT 2016 — https://eprint.iacr.org/2016/260
- Zcash Protocol Specification, §5.4 (proving systems) — https://zips.z.cash/protocol/protocol.pdf
- Conclusion of the Powers of Tau ceremony — https://zfnd.org/conclusion-of-the-powers-of-tau-ceremony/
- ZIP 205, Deployment of the Sapling network upgrade — https://zips.z.cash/zip-0205
- Pine Analytics, Zcash quarterly report Q3 2026 (pool sizes) — https://pineanalytics.substack.com/p/zcash-quarterly-report-q3-2026
- L2BEAT ZK catalog, RISC Zero — https://l2beat.com/zk-catalog/risc0
- bellman — https://github.com/zkcrypto/bellman
