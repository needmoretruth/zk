# Nova

The proof system that introduced folding. Instead of checking a whole proof inside the next proof, Nova folds two
claims into one, so a long computation can be proved step by step at almost no cost per step — and the result can then
be compressed into a short zero-knowledge proof.

> **Shelf:** Others · **First published:** 2021 · **Trusted setup:** none (with IPA commitments) · **Zero-knowledge:** yes
> (compressed proof) · **Post-quantum:** no · **Runs here with:** `nova-snark`, Microsoft's own crate

## What it is

**Incrementally verifiable computation** proves a long computation one step at a time: after every step there is a
proof that all steps so far were right. The classic way puts a verifier for the previous proof inside the circuit of the
next step, which is expensive.

Nova changes what is carried from step to step. It works with **relaxed R1CS**, a version of R1CS with an error term and
a scalar that lets two instances be added together. A **folding scheme** takes a running instance and a new one and
combines them, with a random challenge, into a single instance that is satisfiable only if both were. The per-step
circuit only has to check the folding, which costs about two scalar multiplications — roughly ten thousand constraints —
however large the step itself is.

At the end, the folded instance is proved with a Spartan-style SNARK, which shrinks the proof to a few kilobytes. Before
that compression, the implementation folds in one more, random instance, which makes the compressed proof
zero-knowledge.

## History

- **March 2021.** Abhiram Kothapalli, Srinath Setty and Ioanna Tzialla post Nova; it appears at CRYPTO 2022.
- **December 2022.** SuperNova extends folding to programs whose steps are not all the same.
- **June 2023.** Nguyen, Boneh and Setty show that the original implementation over a two-curve cycle was unsound: they
  forge a proof of 2⁷⁵ rounds of the MinRoot delay function in 1.46 seconds. The implementation moves to a variant with
  a security proof.
- **August 2023.** CycleFold simplifies how folding uses the second curve.
- **October 2024.** NeutronNova folds a SHA-256 circuit about ten times faster than Nova; `nova-snark` ships it as
  experimental.
- **September 2026.** `nova-snark` 0.76.0.

## Strengths

- **The smallest recursion overhead** at its publication: folding costs a constant number of operations per step.
- **No FFTs and no trusted setup** with the default inner-product commitments.
- **Short compressed proofs** of a few kilobytes, and they are zero-knowledge.

## Weaknesses

- **Subtle to implement.** The 2023 attack came from how the proof system used its cycle of curves, not from the folding
  idea itself.
- **Uncompressed proofs are large** — as large as a step — and are not claimed to hide anything.
- **Not post-quantum.** It rests on discrete logarithms.

## How next-generation it is

Very. Folding, which Nova started, is behind HyperNova, ProtoGalaxy and Aztec's client-side proving, and the research
line continues with faster variants such as NeutronNova. Nova itself is the reference point that later schemes measure
themselves against.

## Status today (as of September 2026)

**Experimental.** `nova-snark` is actively released, but no production deployment of Nova folding is confirmed; one
project depends on the crate only for its HyperKZG commitments.

## How this repository runs it

`crates/sys-nova` uses `nova-snark` over the Pallas–Vesta cycle of curves. Each of the museum's statements becomes the
step function of a two-step computation: the public inputs are the state carried from step to step, and the step checks
the statement with the secrets as private advice. The two steps are folded, the result is compressed into a SNARK, and the
verifier checks that compressed proof against the public inputs.

The statements themselves are small, from 34 constraints for one-plus-one to 707 for pool-spend. Each step runs inside
Nova's augmented circuit, which also hashes the carried state and checks the fold, and that brings every statement to
roughly ten to thirteen thousand constraints: the constant overhead of recursion the paper describes, dwarfing the
statement. Setup makes public parameters and keys of 25 to 35 MB. Compressed proofs are about 10.6 KB for every
statement. Nova's prover checks nothing: a false claim is folded and compressed like a true one, and the verifier
rejects it.

Try `/run nova one-plus-one`, then `/run spartan one-plus-one`: Nova compresses with a Spartan-style SNARK.

## Sources

- A. Kothapalli, S. Setty, I. Tzialla, *Nova: Recursive Zero-Knowledge Arguments from Folding Schemes*, CRYPTO 2022 — https://eprint.iacr.org/2021/370
- Nova repository and README — https://github.com/microsoft/Nova
- A. Kothapalli, S. Setty, *SuperNova* — https://eprint.iacr.org/2022/1758
- W. Nguyen, D. Boneh, S. Setty, *Revisiting the Nova Proof System on a Cycle of Curves* — https://eprint.iacr.org/2023/969
- A. Kothapalli, S. Setty, *CycleFold* — https://eprint.iacr.org/2023/1192
- A. Kothapalli, S. Setty, *NeutronNova* — https://eprint.iacr.org/2024/1606
- nova-snark on crates.io — https://crates.io/crates/nova-snark
