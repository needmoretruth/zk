# ZKBoo

A zero-knowledge proof built from nothing but a hash function: the prover acts out a three-party computation in its head
and lets the verifier look at two of the three parties. It proves any circuit, is believed to resist quantum computers,
and became the basis of the Picnic signature scheme.

> **Shelf:** Others · **First published:** 2016 · **Trusted setup:** none · **Zero-knowledge:** yes ·
> **Post-quantum:** believed so (hash only) · **Runs here with:** a teaching implementation written for this repository

## What it is

**MPC in the head** turns secure multi-party computation into a proof. In multi-party computation, several parties
compute a function together on inputs split into shares, so that no one party learns the input. Here the prover plays
all the parties alone.

ZKBoo uses three parties. The prover splits the secret into three shares that add up to it, runs the circuit on the
shares — each multiplication mixes a party's shares with its neighbour's and with random values from their tapes — and
commits to each party's **view**: its random seed, its input share and every value it computed. The verifier picks two
neighbouring parties, and the prover opens their views. The verifier recomputes one party's values from the two views,
checks them against the commitments, and checks that the output shares add up to what the statement claims.

Two views reveal nothing, because the missing share could be anything. A cheating prover must break the computation
between some pair of neighbouring parties, and one challenge in three opens that pair: a cheat survives a round with
probability 2/3. So the proof repeats the round many times, and derives every challenge from a hash of all the
commitments.

## History

- **February 2016.** Irene Giacomelli, Jesper Madsen and Claudio Orlandi post ZKBoo; it appears at USENIX Security 2016.
  It proves knowledge of a SHA-1 preimage in about 13 ms, with a 444 KB proof.
- **March 2017.** ZKB++ halves the proof size and becomes Picnic, a post-quantum signature scheme that proves knowledge of
  a LowMC key; it appears at CCS 2017.
- **2018.** Katz, Kolesnikov and Wang's KKW uses a preprocessing phase for shorter proofs, the basis of Picnic3.
- **July 2022.** NIST does not select Picnic for standardization.
- **June 2023.** Open Quantum Safe removes Picnic from liboqs.
- **May 2026.** Signature schemes built on the same idea's successors (FAEST, SDitH, MQOM) enter the third round of
  NIST's additional signatures process.

## Strengths

- **Only a hash function.** No curves, no pairings, no setup.
- **Believed post-quantum**, since it uses only symmetric primitives.
- **Fast for small circuits**, and any circuit works.

## Weaknesses

- **Proofs grow with the circuit.** Every multiplication adds values to every round, so large circuits give large
  proofs.
- **Many rounds.** A cheat survives one round with probability 2/3, so a proof needs hundreds of rounds.
- **No production use.** Picnic was not standardized.

## How next-generation it is

Its line is. ZKBoo itself is historical, but MPC in the head and its newer relative VOLE in the head are behind several
post-quantum signature candidates still under evaluation at NIST. The museum's own [Trio](../trio/README.md) is built on
the same idea with a different decomposition.

## Status today (as of September 2026)

**Historical.** No deployment of ZKBoo or Picnic is known; the ideas live on in NIST's third-round candidates.

## How this repository runs it

No permissively licensed Rust implementation of ZKBoo for arithmetic circuits exists, so `crates/sys-zkboo` is a
**teaching implementation · not audited**, written from the paper on the Goldilocks field from `p3-goldilocks` and
SHA-256 from `sha2`. It uses the paper's linear three-party decomposition, SHA-256 commitments to each view, and
Fiat–Shamir challenges.

Each proof has 137 rounds: with a cheat surviving a round with probability 2/3, that brings the error to about 2^-80,
the number the paper itself uses for non-interactive proofs. Every round carries two views, so proofs grow with the
multiplications in the circuit: about 100 KB for one-plus-one's 32 and about 1.8 MB for pool-spend's 609. The challenge
hash covers every party's output shares, which the paper puts in the prover's first message; without them a prover could
choose the hidden party's shares after seeing the challenge. A test makes one party cheat in a multiplication and shows
that, of the three challenges, only the one that recomputes that party's values catches it.

Try `/run zkboo age`, then `/run trio age`: two ways to prove with parties in the head.

## Sources

- I. Giacomelli, J. Madsen, C. Orlandi, *ZKBoo: Faster Zero-Knowledge for Boolean Circuits*, USENIX Security 2016 — https://eprint.iacr.org/2016/163
- M. Chase et al., *Post-Quantum Zero-Knowledge and Signatures from Symmetric-Key Primitives* (ZKB++ and Picnic), CCS 2017 — https://eprint.iacr.org/2017/279
- J. Katz, V. Kolesnikov, X. Wang, *Improved Non-Interactive Zero Knowledge with Applications to Post-Quantum Signatures*, CCS 2018 — https://eprint.iacr.org/2018/475
- Picnic — https://microsoft.github.io/Picnic/
- liboqs 0.8.0 release notes — https://github.com/open-quantum-safe/liboqs/releases/tag/0.8.0
