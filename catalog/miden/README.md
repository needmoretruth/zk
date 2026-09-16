# Miden VM

A virtual machine whose every run can be proved with a STARK: you write a program, the machine runs it, and the proof
says the program produced this output from these inputs, without the verifier running it again. It began at Polygon in
2021 on the Winterfell prover and has proved with a Plonky3-based STARK since February 2026.

> **Shelf:** Polygon · **First published:** 2021 · **Trusted setup:** none · **Zero-knowledge:** not as configured ·
> **Post-quantum:** believed so (hash-based) · **Runs here with:** Miden VM's own crates, `miden-prover` and its siblings

## What it is

The other exhibits on this shelf prove a table that someone laid out by hand for one computation. A **zkVM** fixes one
table layout, the machine's, once and for all, and proves any program written for that machine. That is easier for the
programmer and costs more rows per step.

Miden VM is a stack machine over the Goldilocks field `2⁶⁴ − 2³² + 1`. Programs are written in **Miden Assembly**. Inputs
come in two kinds: values placed on the stack, which the verifier sees, and values the program reads from an **advice
provider**, which only the prover has. Running a program produces an execution trace of the stack, memory and helper
components called chiplets (hashing, bitwise operations, memory); the prover commits to that trace and proves it obeys
the machine's rules.

The proof binds a program by its hash, its stack inputs and its stack outputs. Since version 0.22 the backend is a
"lifted STARK": several traces of different heights are proved together, with shorter traces virtually extended to the
tallest, on Plonky3's FRI.

## History

- **2020.** Distaff VM, the project Miden VM grew from.
- **16 November 2021.** Polygon announces Polygon Miden, a STARK-based rollup led by the former Meta researcher who led
  Winterfell; Miden VM 0.1.0 proves with Winterfell.
- **December 2022.** A bug report, "Proof of 0 == 1", shows that two different kinds of program blocks hashed the same
  way, so two different programs shared a hash. It is fixed in February 2023.
- **April 2025.** Miden spins out of Polygon.
- **14 February 2026.** Version 0.21 replaces Winterfell with Plonky3 and the native hash RPO with Poseidon2.
- **March 2026.** Version 0.22 moves to the lifted STARK backend. The team writes that Poseidon2 was found to have
  serious weaknesses that month and that Miden must migrate.
- **June–August 2026.** Versions 0.24, 0.28 and 0.30 fix soundness gaps in the machine's rules that could have allowed
  forged traces.

## Strengths

- **Any program, one machine.** No circuit design for each statement.
- **Fast verification**, usually under a millisecond by the project's own figures.
- **No trusted setup**, security from hashes.
- **Recursion inside the VM.** A verifier for Miden proofs is written in Miden Assembly.

## Weaknesses

- **Alpha.** The README says it has not been audited and is not ready for production.
- **Not zero-knowledge as configured.** The prover's configuration uses a non-hiding commitment, although the
  documentation calls it a zero-knowledge virtual machine.
- **Several soundness fixes in 2026**, and a hash migration still ahead.
- **Large proofs**, 80 to 136 KB in the README's own measurements.

## How next-generation it is

Current. Small-field STARKs are where most zkVMs have gone, and Miden's lifted STARK and client-side design are
recent work. It is not yet proven in production.

## Status today (as of September 2026)

**Experimental.** Miden's site describes testnet v0.16 as the last major testnet release before mainnet, so no mainnet and
no production users yet. `miden-vm` 0.32.1 was released on 9 September 2026.

## How this repository runs it

`crates/sys-miden` uses Miden VM 0.32.1's own crates. The exhibit writes each of the museum's statements as a Miden Assembly program:
the public inputs are placed on the stack, the secrets come from the advice provider, every gate is field arithmetic on
the stack, and every check is an assertion. A false claim makes the program fail its assertion while running, so the
prover refuses it.

Programs run from 540 operations for one-plus-one to about 10,200 for pool-spend, and the machine's trace from 1,024 to
16,384 rows. Sudoku's sixteen public inputs fill the machine's stack exactly. Proofs are 62 KB to 91 KB. The flipped byte
lands in the commitment to the execution trace, which the verifier absorbs before it draws any challenge.

The scan finds no secret, because the proof carries whole columns evaluated at points outside the trace rather than single
cells. The missing hiding shows another way: the prover takes no randomness, so a witness always gives the same proof,
and anyone holding a proof can test a guessed witness by proving it again. Factoring has two witnesses for the same
number, `(p, q)` and `(q, p)`: both verify, their proofs differ, and proving each guess again reproduces its proof byte for
byte.

The exhibit takes the assembler, processor, prover and verifier crates one by one instead of the `miden-vm` crate that
bundles them, because that crate's standard-library feature brings in dependencies under licences this repository does
not accept.

Try `/run miden age`, then `/run plonky3 age`: the same statement as a program for a machine and as a table laid out by
hand, over the same toolkit.

## Sources

- Miden VM repository and README — https://github.com/0xMiden/miden-vm
- Miden VM changelog — https://github.com/0xMiden/miden-vm/blob/next/CHANGELOG.md
- Lifted STARK README — https://github.com/0xMiden/miden-vm/blob/next/crates/lifted-stark/README.md
- Polygon, *Polygon Miden, a STARK-based Ethereum-compatible rollup* (2021) — https://polygon.technology/blog/polygon-announces-polygon-miden-a-stark-based-ethereum-compatible-rollup
- Miden VM issue #605, *Proof of 0 == 1* — https://github.com/0xMiden/miden-vm/issues/605
- Bobbin Threadbare, Miden update (April 2026) — https://hackmd.io/@bobbinth/rk_YT47AZx
- Miden VM proof configuration — https://github.com/0xMiden/miden-vm/blob/next/air/src/config.rs
- Miden — https://miden.xyz/
