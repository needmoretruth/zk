# Ligero

A zero-knowledge argument built from error-correcting codes and a hash function, with proofs about the square root of
the circuit's size and no trusted setup. It sat in papers for years; since 2025 it has proved people's age from their
digital ID in Google Wallet, as the core of Google's Longfellow ZK.

> **Shelf:** Others · **First published:** 2017 · **Trusted setup:** none · **Zero-knowledge:** yes ·
> **Post-quantum:** believed so (hash only) · **Runs here with:** a teaching implementation written for this repository

## What it is

Ligero comes from the same idea as [ZKBoo](../zkboo/README.md), **MPC in the head**, but uses many parties and
error-correcting codes, so the proof grows with the square root of the circuit instead of with the circuit.

The prover writes the witness, with some random padding, as the rows of a matrix. Each row is encoded with a
**Reed–Solomon code**, which stretches it so that any change to the row changes most of its encoding. The prover commits
to the columns of the encoded matrix with a Merkle tree. The verifier then asks for three random combinations of rows —
one to test that every row really is a codeword, one to test the linear constraints, one to test the multiplications —
and for a set of randomly chosen columns to be opened. Because a cheat would have to change a large fraction of some
row's encoding, the opened columns catch it with high probability, while the random padding keeps those columns from
revealing the witness.

## History

- **October 2017.** Scott Ames, Carmit Hazay, Yuval Ishai and Muthuramakrishnan Venkitasubramaniam present Ligero at CCS.
- **July 2023.** An extended journal version appears in Designs, Codes and Cryptography.
- **December 2024.** Matteo Frigo and abhi shelat of Google post *Anonymous credentials from ECDSA*, which combines
  sumcheck with Ligero to prove facts about existing ECDSA-signed credentials. Proving "over 18" from a mobile driver's
  licence takes under a second on a phone.
- **April 2025.** Google Wallet starts using it for zero-knowledge age checks; the library is open-sourced as Longfellow
  ZK in July.
- **August–December 2025.** Trail of Bits, ISRG and an academic panel publish security reviews.
- **July 2026.** A new revision of Google's IETF draft describing Longfellow ZK is published.

## Strengths

- **Only a hash function.** No trusted setup, no curves.
- **Works with credentials that already exist.** Longfellow proves statements about ECDSA signatures and SHA-256 without
  changing how IDs are issued.
- **Fast provers**, fast enough for phones.

## Weaknesses

- **Proofs grow with the square root** of the circuit: hundreds of kilobytes for real credentials.
- **Verification is not succinct** in time; the verifier's work grows with the circuit.
- **Not a standard yet.** IETF work on it is deferred until a protocol needs it.

## How next-generation it is

Old idea, new life. Ligero predates most of the systems on these shelves, but its prover is simple and fast, and pairing
it with sumcheck made it the first zero-knowledge proof many people use every day without knowing it.

## Status today (as of September 2026)

**Active.** Longfellow ZK runs in Google Wallet and is used by Bumble. The EU age-verification blueprint, updated in July
2026, selects it, and European digital identity wallets are integrating it.

## How this repository runs it

Longfellow ZK and libiop's Ligero are C++, so `crates/sys-ligero` is a **teaching implementation · not audited**, written
from the paper on the Goldilocks field and SHA-256.

It uses a code of rate 1/4 and opens 118 columns, numbers derived from the paper's bound so that each round's error stays
below 2^-80. Proofs run from about 40 KB for one-plus-one to about 57 KB for pool-spend: the circuit grows 21 times over
that range and the proof less than one and a half times, because at these sizes the fixed cost of 118 opened columns
dominates, and the square-root growth takes over only for larger circuits. Random padding in every row keeps the opened
columns from revealing the witness, and a salt in each Merkle leaf keeps the rest hidden. This is the 2017 protocol, not
Longfellow's combination of it with sumcheck.

Try `/run ligero pool-spend`, then `/run zkboo pool-spend`: the same idea, with proofs that grow as the square root of the
circuit instead of with the circuit.

## Sources

- S. Ames, C. Hazay, Y. Ishai, M. Venkitasubramaniam, *Ligero: Lightweight Sublinear Arguments Without a Trusted Setup*, CCS 2017 (extended version) — https://eprint.iacr.org/2022/1608
- Journal version, Designs, Codes and Cryptography (2023) — https://doi.org/10.1007/s10623-023-01222-8
- M. Frigo, a. shelat, *Anonymous credentials from ECDSA* — https://eprint.iacr.org/2024/2010
- Longfellow ZK — https://github.com/google/longfellow-zk
- EU age verification, zero-knowledge annex — https://ageverification.dev/av-doc-technical-specification/docs/annexes/annex-B/annex-B-zkp/
- IETF draft, Longfellow ZK — https://datatracker.ietf.org/doc/draft-google-cfrg-libzk/
