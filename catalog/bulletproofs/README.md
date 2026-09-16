# Bulletproofs

Short proofs without a trusted setup, from nothing but the hardness of discrete logarithms. Built to hide the amounts in
cryptocurrency transactions, they cut Monero's transaction sizes by more than 80% when they went live in 2018.

> **Shelf:** Others · **First published:** 2017 · **Trusted setup:** none · **Zero-knowledge:** yes ·
> **Post-quantum:** no · **Runs here with:** `bulletproofs`, zkcrypto's fork of the dalek implementation

## What it is

A Pedersen commitment `v·G + r·H` hides a number `v` behind a random `r`, and many numbers can be committed together
with many generators. Bulletproofs proves facts about committed vectors with an **inner-product argument**: to show that
two committed vectors have a given inner product, the prover folds both vectors in half with a random challenge, sends
two group elements, and repeats. After `log n` rounds one number is left, so the proof has only about `2·log n` elements.

The best-known use is a **range proof**: "this committed amount lies between 0 and 2^64", written as "its 64 bits are
each 0 or 1 and they add up to the amount", and proved with one inner-product argument. A 64-bit range proof is about
670 bytes, and proofs for many amounts can be aggregated for only a few elements more. The same machinery proves any
arithmetic circuit given as multiplication gates plus linear constraints.

The price is on the verifier's side: it has to recompute the folded generators, which takes time linear in the size of
the statement. Many proofs can be verified together to spread that cost.

## History

- **2016.** Bootle, Cerulli, Chaidos, Groth and Petit give an efficient discrete-log argument for arithmetic circuits
  with logarithmic communication, the argument Bulletproofs builds on.
- **November 2017.** Bünz, Bootle, Boneh, Poelstra, Wuille and Maxwell publish Bulletproofs; it appears at IEEE S&P 2018.
- **18 October 2018.** Monero adopts Bulletproofs for the amounts in its transactions, shrinking them by more than 80%.
- **June 2020.** Bulletproofs+ adds a zero-knowledge weighted inner-product argument; a 64-bit range proof drops to 576
  bytes.
- **15 April 2022.** Trail of Bits' "Frozen Heart" disclosure: the challenges suggested in the paper leave out the
  commitment being proved about, and several implementations that followed them let proofs be forged.
- **13 August 2022.** Monero moves to Bulletproofs+.

## Strengths

- **No trusted setup.** The generators are derived by hashing; nobody holds a secret.
- **Short.** Logarithmic in the statement: a 64-bit range proof is under a kilobyte.
- **Aggregation and batching.** Many range proofs in one, and many proofs checked together.
- **Only the discrete-log assumption**, on curves with fast, well-audited arithmetic.

## Weaknesses

- **Linear-time verification.** Checking a proof costs time in proportion to the statement, which is also why recursion
  on top of Bulletproofs-style commitments needs accumulation rather than verifying proofs inside proofs.
- **Slow for large circuits.** Proving and verifying general circuits is far slower than for pairing SNARKs or STARKs.
- **Fiat–Shamir pitfalls.** The paper's own choice of challenges led implementations to be forgeable.
- **Not post-quantum.** Discrete logarithms fall to Shor's algorithm.

## How next-generation it is

Bulletproofs is mature rather than new, and still the reference point for range proofs without a trusted setup. Its
inner-product argument lives on: the IPA polynomial commitment inside Zcash's Halo 2 is built from the same idea.

## Status today (as of September 2026)

**Superseded in its main deployment by its own successor.** Monero replaced Bulletproofs with Bulletproofs+ in August
2022 and still uses it in release 0.18.5.x; its next proposed upgrade, FCMP++, is not yet activated. Tari maintains a
Rust Bulletproofs+ library.

## How this repository runs it

`crates/sys-bulletproofs` uses the `bulletproofs` crate — zkcrypto's fork of dalek-cryptography's implementation, the one
published on crates.io — pinned to a commit, with its R1CS prover over ristretto255. The published crate leaves the R1CS
prover switched off, so this exhibit builds it from the repository.
Each constraint of the museum's circuit becomes one multiplication gate and a linear constraint, private values are
allocated inside the proof, and public inputs appear as constants that the verifier's constraints are built from.

Proofs here are 801 bytes for `one-plus-one` and 1,057 bytes for `pool-spend`: they grow by 64 bytes each time the
number of multiplication gates doubles. Two things the exhibit adds around the upstream crate:
- The public inputs are hashed into the transcript before any challenge is drawn. Upstream hashes only commitments, and
  constants that are left out of the hash are exactly the Frozen Heart mistake.
- Upstream also accepts the same proof rewritten in a longer layout. The museum accepts only the bytes the prover wrote.

Try `/run bulletproofs membership`, then `/run halo2 membership`: two systems without a trusted setup, one of them a
descendant of the other.

## Sources

- B. Bünz, J. Bootle, D. Boneh, A. Poelstra, P. Wuille, G. Maxwell, *Bulletproofs: Short Proofs for Confidential Transactions and More*, IEEE S&P 2018 — https://eprint.iacr.org/2017/1066
- J. Bootle, A. Cerulli, P. Chaidos, J. Groth, C. Petit, *Efficient Zero-Knowledge Arguments for Arithmetic Circuits in the Discrete Log Setting*, EUROCRYPT 2016 — https://eprint.iacr.org/2016/263
- Monero 0.13.0 release — https://www.getmonero.org/2018/10/11/monero-0.13.0-released.html
- H. Chung, K. Han, C. Ju, M. Kim, J. H. Seo, *Bulletproofs+* — https://eprint.iacr.org/2020/735
- Trail of Bits, *The Frozen Heart vulnerability in Bulletproofs* — https://blog.trailofbits.com/2022/04/15/the-frozen-heart-vulnerability-in-bulletproofs/
- Monero network upgrade, July 2022 — https://web.getmonero.org/2022/04/20/network-upgrade-july-2022.html
- Monero, deprecating unlock time (FCMP++ not yet activated) — https://www.getmonero.org/2026/05/10/deprecating-unlock-time.html
- Ragu book, Bulletproofs and accumulation — https://github.com/tachyon-zcash/ragu/blob/main/book/src/protocol/prelim/bulletproofs.md
- bulletproofs (zkcrypto fork) — https://github.com/zkcrypto/bulletproofs · original — https://github.com/dalek-cryptography/bulletproofs
