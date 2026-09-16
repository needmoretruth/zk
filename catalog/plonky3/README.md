# Plonky3

Polygon's open toolkit for building STARK provers: fields, hashes, polynomial commitments and provers you assemble into
the proof system you need. Several of today's zkVMs — SP1, OpenVM and Miden among them — are built on it.

> **Shelf:** Polygon · **First released:** 2024 · **Trusted setup:** none · **Zero-knowledge:** only with a hiding
> commitment · **Post-quantum:** believed so (hash-based) · **Runs here with:** `p3-uni-stark`, Plonky3's own crates

## What it is

A STARK proves that a computation was carried out correctly by writing its steps into a table, the **execution trace**,
and a set of polynomial rules, the **AIR**, that every row (and each pair of neighbouring rows) must obey. The prover
turns each column into a polynomial over a small, fast field, commits to those polynomials with a hash-based
commitment, and the verifier checks the rules at a random point outside the table together with a test that the
committed data really is a low-degree polynomial.

Plonky3 does not fix one such system; it is a set of parts. You choose a field (BabyBear, KoalaBear, Mersenne31,
Goldilocks or binary fields), a hash (Poseidon2, Keccak, BLAKE3 and others), a commitment scheme (FRI, STIR, WHIR, or
the circle commitment for Mersenne31), and a prover (`uni-stark` for one table, `batch-stark` and `multi-stark` for
several). Small fields and SIMD instructions make it fast.

Plonky3 is not zero-knowledge by default. Its standard FRI commitment opens trace values in the clear: a proof shows
that the computation was right, and also shows parts of it. For privacy you must pick the hiding commitment, which mixes
random rows and salts into what is committed.

## History

- **2023.** Polygon Zero, the team behind Plonky2, begins Plonky3 in the open, dual-licensed MIT and Apache-2.0.
- **March 2024.** The circle FFT and commitment for the Mersenne31 field land, the basis of Circle STARKs.
- **4 July 2024.** First release on crates.io.
- **16 July 2024.** Polygon calls Plonky3 production-ready, with Valida and SP1 as named users. Least Authority's audit
  report follows on 31 July.
- **18 November 2024.** A hiding FRI commitment is added, which makes zero-knowledge proofs possible.
- **2025.** Three high-severity advisories are fixed in the FRI verifier and the transcript: a malicious prover could
  have cheated because opened values were not bound into the transcript, because size checks were missing, and because a
  final degree check was missing.
- **February 2026.** Miden VM replaces Winterfell with Plonky3.
- **March 2026.** WHIR is integrated.
- **September 2026.** Version 0.7.0.

## Strengths

- **Fast.** Small 31-bit fields fit machine words, and the arithmetic uses AVX2, AVX-512 and NEON.
- **No trusted setup** and hash-based security, which is believed to hold against quantum computers.
- **Modular.** Swap a field, hash or commitment without rewriting the rest; the same code backs very different provers.
- **Stable Rust, audited, widely used.** Many zkVMs depend on it, so its bugs get found.

## Weaknesses

- **Not private unless you ask.** The default commitment leaks the trace; zero-knowledge needs the hiding variant, and
  even that is statistical, not perfect.
- **Large proofs.** Hundreds of kilobytes are common. Projects that post proofs on a blockchain usually wrap the final
  STARK proof in a pairing-based SNARK, which is not post-quantum.
- **Conjectured security.** The security levels of FRI-style commitments rest on proximity-gap conjectures; some of the
  strongest were disproved in 2025, so claimed bit-security has to be read with care.
- **Verifier bugs have happened.** Four high-severity advisories in 2025–2026, three of them soundness bugs.

## How next-generation it is

It is at the front. Small-field, hash-based proving is the approach of the best-known zkVMs in 2026 — RISC Zero, SP1,
OpenVM, Miden — and Plonky3 is the shared toolkit several of them build on. Its newer pieces — STIR, WHIR, multilinear and binary-field commitments — track the research as it lands.

## Status today (as of September 2026)

**Active.** Developed in the open, version 0.7.0 released in September 2026. OpenVM, SP1 and Miden VM are among the
projects built on it; L2BEAT lists OpenVM, which proves Scroll, as "based on Plonky3".

## How this repository runs it

`crates/sys-plonky3` uses `p3-uni-stark` 0.7.0 over BabyBear with **the hiding FRI commitment**, the configuration
Plonky3's own zero-knowledge tests use, with every random seed taken from your operating system.

The museum's statements are one-off circuits, not long repeated computations, so each becomes an AIR with a single
very wide row: one column per wire and one degree-2 rule per gate. That is the least flattering shape for a STARK, whose
proof size grows with the number of columns. Proofs here run from about 55 KB for `one-plus-one` to about 450 KB for
`pool-spend`. A STARK shines when a table has millions of rows of the same step.

Two things you will see:
- A false claim is refused by the prover before any proof exists when the program is built with debug checks, because
  Plonky3's prover checks the trace in that mode; in a release build the proof is made and the verifier rejects it.
- The flipped byte lands in an opened trace value. That value is hashed into the transcript before the commitment's
  proof-of-work check, so the check fails there, before the algebra is reached.

Try `/run plonky3 pool-spend`, then `/run groth16 pool-spend` for the other end of the size scale.

## Sources

- Plonky3 repository and architecture notes — https://github.com/Plonky3/Plonky3 · https://github.com/Plonky3/Plonky3/blob/main/docs/architecture.md
- Polygon, *Plonky3, the next generation of ZK proving systems, is production ready* — https://polygon.technology/blog/polygon-plonky3-the-next-generation-of-zk-proving-systems-is-production-ready
- Circle FFT and PCS — https://github.com/Plonky3/Plonky3/pull/278
- Hiding FRI commitment — https://github.com/Plonky3/Plonky3/pull/536
- Audits — https://github.com/Plonky3/Plonky3/tree/main/audits
- Security advisories — https://github.com/Plonky3/Plonky3/security/advisories
- Release v0.7.0 — https://github.com/Plonky3/Plonky3/releases/tag/v0.7.0
- Miden VM changelog — https://github.com/0xMiden/miden-vm/blob/next/CHANGELOG.md
- WHIR integration — https://github.com/Plonky3/Plonky3/pull/1477
- D. Crites, A. Stewart, refutation of Reed–Solomon proximity-gap conjectures — https://eprint.iacr.org/2025/2046
- L2BEAT ZK catalog, OpenVM — https://l2beat.com/zk-catalog/openvmprover
