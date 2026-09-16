# STARK

The original hash-based proof of computational integrity: no trusted setup, security from hash functions, and verification
exponentially faster than the computation. StarkWare's Stone prover, built on it, proved StarkEx from 2020 and Starknet
until Circle STARKs replaced it in November 2025.

> **Shelf:** Others · **First published:** 2018 · **Trusted setup:** none · **Zero-knowledge:** no ·
> **Post-quantum:** believed so (hash-based) · **Runs here with:** lambdaworks' STARK prover

## What it is

A STARK writes a computation as an **execution trace**, a table with one row per step, and an **AIR**, the polynomial
rules each row and each pair of neighbouring rows must obey. Each column is interpolated over a multiplicative subgroup
of a large prime field and extended to a domain several times larger, so that a wrong trace becomes a polynomial far
from low degree. The prover commits to the extended columns with Merkle trees.

The verifier then asks for the rules to be checked at a random point outside the trace (the **DEEP** method), and for
proof that what was committed really is close to a low-degree polynomial (the **FRI** proximity test). Hashing the
transcript makes the protocol non-interactive. The result is a proof of a few hundred kilobytes that is checked in
milliseconds, however long the computation.

The 2018 paper and StarkWare's ethSTARK description use large fields, like the 252-bit Stark field this exhibit runs on.
Later STARKs moved to smaller fields for speed: [Winterfell](../winterfell/README.md) offers 62- and 64-bit fields, and
[Plonky3](../plonky3/README.md) and [Circle STARK](../circle-stark/README.md) use 31-bit ones.

## History

- **January 2018.** Eli Ben-Sasson, Iddo Bentov, Yinon Horesh and Michael Riabzev post *Scalable, transparent, and
  post-quantum secure computational integrity*. Its demonstration proves that a DNA profile is not in a police database.
- **July 2018.** The Ethereum Foundation funds ethSTARK, a production-grade STARK.
- **June 2020.** StarkEx goes into production on Ethereum mainnet.
- **May 2021.** The ethSTARK documentation is published.
- **August 2023.** StarkWare open-sources Stone, the prover behind StarkEx and Starknet, under Apache-2.0.
- **September 2024.** Last commit to the Stone repository.
- **2025.** Two papers disprove conjectures about Reed–Solomon proximity gaps that FRI and DEEP-FRI parameters had relied
  on.
- **3 November 2025.** S-two, a Circle STARK prover, replaces Stone on Starknet's mainnet.

## Strengths

- **No trusted setup.** Security rests on hash functions.
- **Believed to resist quantum computers**, since it uses no elliptic curves.
- **Very fast verification** for very long computations.
- **A long production record** at StarkWare.

## Weaknesses

- **Large proofs**, around a hundred kilobytes and more.
- **Heavy provers.** Large-field arithmetic and big traces need a lot of memory.
- **Conjectured parameters.** The 2025 disproofs hit the conjectures that conjectured-security parameters assumed.
- **Not zero-knowledge** as described: ethSTARK makes no hiding claim, and Stone has no blinding code.

## How next-generation it is

Not any more. STARK started the hash-based line that today's fastest provers belong to, but the classic large-field
version has been replaced by small-field and circle constructions, even at StarkWare.

## Status today (as of September 2026)

**Superseded on Starknet by S-two** since November 2025. Stone is dormant; whether StarkEx still proves with it is not
confirmed.

## How this repository runs it

`crates/sys-stark` uses the STARK prover from lambdaworks, a Rust implementation that follows the ethSTARK description,
pinned to one commit. StarkWare's own Stone prover is C++, which this repository does not build. Each statement becomes
a trace with the circuit's wires as columns and one rule per gate, with the public inputs checked on the first row.

The trace is the circuit's row repeated four times, the fewest rows lambdaworks' FRI will commit to. Every column is
then constant, so the proof's values at the out-of-domain point are the wire values themselves. lambdaworks writes field
elements in Montgomery form, and the museum's scan looks for that form too: it finds the salt in one-plus-one and age,
the PIN in password, and the spending key and Merkle siblings in pool-spend. With lambdaworks' preset for 128 bits of
conjectured security, proofs run from about 230 KB for one-plus-one to about 3.7 MB for pool-spend, most of it the 55 FRI
query openings, each carrying every column. Proving includes a 20-bit proof-of-work search, so its time varies from run
to run. The prover does not check the witness: a false claim becomes a proof, and the verifier rejects it.

Try `/run stark age`, then `/run winterfell age` and `/run circle-stark age`: three generations of STARK fields.

## Sources

- E. Ben-Sasson, I. Bentov, Y. Horesh, M. Riabzev, *Scalable, transparent, and post-quantum secure computational integrity* — https://eprint.iacr.org/2018/046
- StarkWare, *ethSTARK Documentation* — https://eprint.iacr.org/2021/582
- StarkWare, *Open-sourcing the battle-tested Stone prover* (2023) — https://starkware.co/blog/open-sourcing-the-battle-tested-stone-prover/
- Stone prover repository — https://github.com/starkware-libs/stone-prover
- StarkEx — https://starkware.co/starkex/
- Starknet, *S-two is live on Starknet mainnet* (2025) — https://www.starknet.io/blog/s-two-is-live-on-starknet-mainnet-the-fastest-prover-for-a-more-private-future/
- Proximity-gap counterexamples (2025) — https://eprint.iacr.org/2025/2046 · https://eprint.iacr.org/2025/2010
- lambdaworks — https://github.com/lambdaclass/lambdaworks
