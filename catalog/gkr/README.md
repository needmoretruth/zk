# GKR

The interactive proof that checks a layered arithmetic circuit one layer at a time, with a sumcheck per layer, so the
verifier does far less work than evaluating the circuit. It hides nothing by itself; Hyrax, Libra and Virgo added
commitments to turn it into zero-knowledge arguments, and today it runs inside other provers as LogUp-GKR.

> **Shelf:** Others · **First published:** 2008 · **Trusted setup:** none · **Zero-knowledge:** yes, with Hyrax ·
> **Post-quantum:** depends on the commitment; not with Hyrax · **Runs here with:** Worldcoin's Remainder, GKR with Hyrax

## What it is

A **layered circuit** is a circuit whose gates are arranged in layers, each gate reading two wires from the layer below.
GKR starts from a claim about the output layer — for example, that every output is zero. It turns that claim into a
claim about the layer below with the **sumcheck protocol**, a conversation in which the prover sends one small polynomial
per variable and the verifier checks each against the last. After one sumcheck per layer, what is left is a claim about
the inputs, which the verifier checks directly.

The verifier's work grows with the depth of the circuit and the logarithm of its width, not with its size. The prover
does work proportional to the circuit, and later papers brought that to linear time.

Plain GKR is a proof for **delegated computation**: the verifier must know every input. To prove knowledge of a secret
input, the input is committed to instead, and the final check is an opening of that commitment. Hyrax did this with
Pedersen commitments and added zero-knowledge; Libra added masking polynomials; Virgo used a hash-based commitment.

## History

- **2008.** Shafi Goldwasser, Yael Tauman Kalai and Guy Rothblum present *Delegating Computation: Interactive Proofs for
  Muggles* at STOC; the journal version follows in 2015.
- **2013.** Justin Thaler's work at CRYPTO brings the prover to linear time for regular circuits.
- **2018.** Hyrax turns GKR into a zero-knowledge argument with discrete-log commitments.
- **2019–2020.** Libra reaches a linear-time prover with a one-time setup; Virgo removes the setup with a hash-based
  commitment.
- **August 2023.** Papini and Haböck publish LogUp-GKR, which proves lookups with GKR.
- **January 2025.** Khovratovich, Rothblum and Soukhanov show *How to Prove False Statements*: when a GKR circuit can
  compute its own Fiat–Shamir hash, the non-interactive version can be made to accept false statements. Polyhedra's
  Expander, named as a deployed system, fixes its Fiat–Shamir first.
- **March 2026.** OtterSec reports claimed values left unbound in GKR- and sumcheck-based provers, fixed in Expander in
  January and in Scroll's Ceno in March.

## Strengths

- **Linear-time proving**, and only the witness needs a commitment, not every intermediate layer.
- **A small verifier** for shallow, regular circuits.
- **No trusted setup** in its basic form.

## Weaknesses

- **Not zero-knowledge by itself.** Hiding needs a commitment and masking on top.
- **Depth matters.** Deep or irregular circuits make the verifier and the proof larger.
- **Fiat–Shamir is delicate.** The 2025 result shows where the usual hash-based conversion fails.

## How next-generation it is

Very, as a component. GKR is behind the lookup arguments of S-two and SP1 and behind several sumcheck-based zkVMs, and
sumcheck-based proving is one of the main directions in current research.

## Status today (as of September 2026)

**Active as a component.** S-two uses GKR for LogUp. Worldcoin's Remainder, a GKR and Hyrax prover, is not deployed in
production.

## How this repository runs it

`crates/sys-gkr` uses Worldcoin's Remainder, pinned to one commit, in its Hyrax mode over BN254. Public inputs form one
input layer, which the verifier fills in itself. Secrets form a second input layer that is committed to with Pedersen
commitments, so they never reach the verifier.

Remainder's gates multiply pairs of values from earlier layers and add the products, without coefficients. So the exhibit
writes each statement as rows of products, supplies constants and ones as public values, and copies every value it still
needs up through the layers one level at a time. The last layer holds every check, and Remainder requires all of them to
be zero.

That makes the circuits deep: 52 layers for one-plus-one, and 549 layers with about 118,000 gates for pool-spend. Proofs
grow with depth, from 16 KB for factoring's five layers to 1.4 MB for pool-spend, which takes tens of seconds to prove.
Remainder's prover checks nothing: a false claim produces a proof, and the verifier rejects it.

Try `/run gkr one-plus-one`, then `/run spartan one-plus-one`: two ways to build a proof from sumchecks.

## Sources

- S. Goldwasser, Y. T. Kalai, G. N. Rothblum, *Delegating Computation: Interactive Proofs for Muggles*, STOC 2008 / JACM 2015 — https://doi.org/10.1145/2699436
- J. Thaler, *Time-Optimal Interactive Proofs for Circuit Evaluation*, CRYPTO 2013 — https://eprint.iacr.org/2013/351
- R. S. Wahby, I. Tzialla, a. shelat, J. Thaler, M. Walfish, *Doubly-efficient zkSNARKs without trusted setup* (Hyrax), IEEE S&P 2018 — https://eprint.iacr.org/2017/1132
- T. Xie, J. Zhang, Y. Zhang, C. Papamanthou, D. Song, *Libra*, CRYPTO 2019 — https://eprint.iacr.org/2019/317
- J. Zhang, T. Xie, Y. Zhang, D. Song, *Virgo*, IEEE S&P 2020 — https://eprint.iacr.org/2019/1482
- S. Papini, U. Haböck, *Improving logarithmic derivative lookups using GKR* — https://eprint.iacr.org/2023/1284
- D. Khovratovich, R. D. Rothblum, L. Soukhanov, *How to Prove False Statements* — https://eprint.iacr.org/2025/118
- Polyhedra Expander, Fiat–Shamir fix — https://github.com/PolyhedraZK/Expander/pull/184
- OtterSec, report on unbound claims in zkVMs (2026) — https://osec.io/blog/zkvms-unfaithful-claims/
- S-two — https://github.com/starkware-libs/stwo
- Worldcoin Remainder — https://github.com/worldcoin/Remainder_CE
