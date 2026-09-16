# Circle STARK

A STARK over the Mersenne prime 2³¹ − 1, a field whose arithmetic is among the cheapest a computer can do, made
possible by working on a circle instead of a multiplicative group. Designed by Polygon and StarkWare cryptographers in
2024, it is what StarkWare's S-two prover uses to prove Starknet.

> **Shelf:** Polygon · **First published:** 2024 · **Trusted setup:** none · **Zero-knowledge:** no ·
> **Post-quantum:** believed so (hash-based) · **Runs here with:** `p3-circle` from Plonky3

## What it is

A STARK needs, inside its field, a group of points whose size is a large power of two: that is where the trace's
polynomials live and where fast Fourier transforms run. The prime `p = 2³¹ − 1` (Mersenne31) makes multiplication very
cheap on 32-bit hardware, but `p − 1` has only one factor of two, so the usual multiplicative group is useless.

Circle STARKs use a different group: the points `(x, y)` with `x² + y² = 1` over the field. That circle has `p + 1`
points, and `p + 1 = 2³¹` is a perfect power of two. The paper rebuilds everything a STARK needs on it — a circle FFT,
Reed–Solomon codes on the circle, and FRI low-degree testing — and shows that the result is as efficient as a classic
STARK. Challenges come from an extension field, because 31 bits are too few for security on their own.

Like other STARKs, the core protocol opens trace values to the verifier. Without extra hiding it is not zero-knowledge.

## History

- **June 2023.** Haböck, Lubarov and Nabaglo publish Reed–Solomon codes over the circle group, the groundwork.
- **July 2023.** StarkWare creates the S-two (Stwo) repository.
- **February 2024.** Ulrich Haböck (Polygon), David Levit and Shahar Papini (StarkWare) post *Circle STARKs*. At ETHDenver
  StarkWare announces S-two as open source, developed with Polygon's cryptographers.
- **March 2024.** Plonky3 adds a circle FFT and commitment scheme.
- **July 2025.** S-two 1.0.
- **3 November 2025.** S-two replaces Stone as Starknet's prover.
- **June 2026.** A Lean 4 proof that the S-two Cairo AIR encoding is sound is published.

## Strengths

- **Very fast arithmetic.** Mersenne31 elements fit 32-bit SIMD lanes, and reduction modulo `2³¹ − 1` is a shift and an add.
- **No trusted setup** and hash-based security.
- **In production.** S-two proves Starknet and several other applications without wrapping its proofs in a pairing SNARK.

## Weaknesses

- **Not zero-knowledge by default.** Privacy needs extra blinding, which StarkWare adds only in a separate pipeline.
- **Conjectured security.** Like other FRI-based systems it rests partly on proximity-gap conjectures, plus proof-of-work
  grinding.
- **Needs special fields.** It works because `p + 1` is highly divisible by two; that is true of few primes.
- **Large proofs**, as for any STARK.

## How next-generation it is

At the front. It is one of the newest designs for fast small-field STARKs, it runs in production on a major rollup, and its
AIR for Cairo has a machine-checked soundness proof. Its main trade-offs — conjectured security and no default privacy —
are those of the whole STARK family.

## Status today (as of September 2026)

**Active.** L2BEAT lists the S-two verifier as serving Starknet, Paradex, Sorare, edgeX and tanX. Plonky3 maintains its
own circle commitment scheme, which this exhibit uses.

## How this repository runs it

`crates/sys-circle-stark` uses Plonky3's circle crates over Mersenne31, in the same one-wide-row form as the
[Plonky3](../plonky3/README.md) exhibit: one column per wire and one rule per gate. S-two itself needs a nightly compiler;
Plonky3's circle commitment builds with the stable compiler this repository uses.

The trace repeats the circuit's row four times, the fewest rows `CirclePcs` will commit to. Every column is then
constant, so its value at the verifier's random point is the wire value itself, and the proof carries that value in the
clear. The museum's scan finds every secret of 65,536 or more in the proof bytes: the salt in one-plus-one, the PIN in
password, the spending key and Merkle siblings in pool-spend. Proofs run from about 29 KB for one-plus-one to about
420 KB for pool-spend. The commitment takes no random input, so the same claim always gives the same proof.

Try `/run circle-stark one-plus-one`, then `/run plonky3 one-plus-one`: the same kind of proof over two fields, one hiding
its secrets and one not.

## Sources

- U. Haböck, D. Levit, S. Papini, *Circle STARKs* — https://eprint.iacr.org/2024/278
- U. Haböck, D. Lubarov, J. Nabaglo, *Reed-Solomon Codes over the Circle Group* — https://eprint.iacr.org/2023/824
- S-two repository — https://github.com/starkware-libs/stwo
- The Block, StarkWare open-sources Stwo (2024) — https://www.theblock.co/post/279907/starkware-open-source-zero-knowledge-prover-stwo
- Starknet, *S-two is live on Starknet mainnet* (2025) — https://www.starknet.io/blog/s-two-is-live-on-starknet-mainnet-the-fastest-prover-for-a-more-private-future/
- Lean 4 soundness proof of the S-two Cairo AIR (2026) — https://arxiv.org/abs/2606.04311
- StarkWare proving pipeline with ZK blinding — https://github.com/starkware-libs/proving/blob/main/crates/circuit_common/src/finalize.rs
- L2BEAT ZK catalog, Stwo — https://l2beat.com/zk-catalog/stwo
- Plonky3 circle FFT and PCS — https://github.com/Plonky3/Plonky3/pull/278
