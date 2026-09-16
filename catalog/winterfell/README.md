# Winterfell

A general-purpose STARK prover that Meta's blockchain research group released in 2021, and that Polygon Miden ran on
for four years. It is succinct but, by its own README, not zero-knowledge — which makes it the museum's clearest
example that those are two different properties.

> **Shelf:** Polygon · **First published:** 2021 · **Trusted setup:** none · **Zero-knowledge:** no ·
> **Post-quantum:** believed so (hash-based) · **Runs here with:** `winter-prover`, Winterfell's own crates

## What it is

You describe a computation as an **execution trace** — a table whose rows are the machine's successive states — and as
an **AIR**: rules that every pair of neighbouring rows must satisfy, plus assertions about particular cells, such as the
first row's inputs. Winterfell extends each column into a polynomial over a larger domain, combines all the rules into
one composition polynomial, checks it at a random point outside the trace (the DEEP method), and proves with FRI that
everything committed really is a low-degree polynomial.

It offers three fields — a 62-bit, a 64-bit and a 128-bit prime field, with extensions for challenges — and several
hashes, from SHA3 and BLAKE3 to the algebraic Rescue Prime. It reports conjectured and proven security levels for the
parameters you choose.

What it does not do is hide the trace. Its README says it "provides succinct proofs but NOT perfect zero-knowledge". A
Winterfell proof convinces the verifier that the computation was right, and some of the computation's values travel in
the proof.

## History

- **April 2021.** Novi Research, Meta's blockchain research group, opens the repository. An issue asking for perfect
  zero-knowledge is opened five days later; it is still open.
- **4 August 2021.** Meta announces Winterfell, by Irakliy Khaburzaniya, Kostas Chalkias, Harjasleen Malvai and Kevin Lewi.
- **November 2021.** Polygon announces Polygon Miden, a STARK-based rollup led by the former Meta researcher who led
  Winterfell's development; Miden VM proves with Winterfell.
- **2022–2025.** Randomized AIR, proven-security estimates, Lagrange kernel constraints and, for a while, LogUp-GKR are
  added. A pull request adding zero-knowledge is opened in 2024 and not merged.
- **19 July 2025.** Version 0.13.1, the last release so far.
- **14 February 2026.** Miden VM 0.21 replaces its Winterfell backend with Plonky3.

## Strengths

- **A simple way to write an AIR**: rules over two neighbouring rows, and assertions on cells.
- **Fast and portable.** Multi-threaded, and it builds without the standard library and for WebAssembly.
- **No trusted setup** and hash-based security.
- **Security estimates built in** for the chosen field, hash and parameters.

## Weaknesses

- **Not zero-knowledge.** Proofs can reveal values of the trace, so it is unsuitable where inputs must stay secret.
- **Not audited.** Its README calls it a research project not ready for production use.
- **Dormant.** No release since July 2025, and its main user has moved on.
- **Large proofs**, as for other STARKs: its README reports 128 KB for a million-step hash chain at 96-bit security.

## How next-generation it is

Winterfell is from the first wave of practical Rust STARKs, built on large 62- to 128-bit fields. Practice has since
moved to 31-bit and binary fields with SIMD arithmetic (Plonky3, S-two, Binius) and to hiding commitments, and Winterfell's
main user followed. Its AIR interface and DEEP-FRI design remain a clear illustration of how STARKs work.

## Status today (as of September 2026)

**Superseded in Polygon Miden by Plonky3.** The code is still published under MIT and a few projects depend on it, but no
production user was found and development has stopped.

## How this repository runs it

`crates/sys-winterfell` uses Winterfell's crates as published. The museum's statements are one-off circuits, so each
becomes a trace of identical rows whose columns are the circuit's wires, with one transition rule per gate and a rule
keeping neighbouring rows equal; public inputs are assertions on the first row.

The trace has eight identical rows and one extra column, a step counter: Winterfell's prover checks the degree of a
polynomial it builds from the trace, and a trace made only of constant columns fails that check. Because the wire
columns are constant, the row the proof opens at the verifier's random point is the circuit's row itself, byte for byte.
The museum's scan finds the salt in one-plus-one and age and the PIN in password. A Winterfell trace must have fewer than
255 columns, so membership (321 wires) and pool-spend (1,008 wires) do not fit this one-row layout and are marked
unsupported. With Winterfell's documented parameters for about 96 bits of security, proofs run from about 25 KB for
one-plus-one to about 64 KB for sudoku.

Try `/run winterfell age`, then `/run plonky3 age`: two STARKs, one of which hides the birth year.

## Sources

- Winterfell repository and README — https://github.com/facebook/winterfell
- Meta Engineering, *Winterfell: A STARK prover and verifier* (2021) — https://engineering.fb.com/2021/08/04/open-source/winterfell/
- Issue #9, *Implement perfect zero-knowledge* — https://github.com/facebook/winterfell/issues/9
- Pull request #293, *Adding zero-knowledge* — https://github.com/facebook/winterfell/pull/293
- Winterfell changelog — https://github.com/facebook/winterfell/blob/main/CHANGELOG.md
- Polygon, *Polygon Miden, a STARK-based Ethereum-compatible rollup* (2021) — https://polygon.technology/blog/polygon-announces-polygon-miden-a-stark-based-ethereum-compatible-rollup
- Miden VM changelog — https://github.com/0xMiden/miden-vm/blob/next/CHANGELOG.md
- Reverse dependencies of winter-prover — https://crates.io/crates/winter-prover/reverse_dependencies
