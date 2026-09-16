# GM17

Groth16's sibling from the following year: a proof just as small, which nobody can alter into a different valid proof.
That property made it a signature of knowledge, at the cost of a slower prover and a bigger setup.

> **Shelf:** Others · **First published:** 2017 · **Trusted setup:** per circuit · **Zero-knowledge:** yes ·
> **Post-quantum:** no · **Runs here with:** `ark-gm17`, arkworks' implementation

## What it is

Like [Groth16](../groth16/README.md), GM17 proves a circuit with three elliptic-curve points — two in G1, one in G2 —
after a trusted setup made for that circuit. The difference is what an attacker can do with a proof they have seen.

A Groth16 proof can be **re-randomized**: anyone can turn a valid proof into a different-looking valid proof of the same
statement. That is harmless for many uses, and a problem for others — for example when a proof's bytes serve as an
identifier. GM17 is **simulation-extractable**: even after seeing many proofs, including proofs of statements chosen by
the attacker, nobody can produce a new valid proof without knowing a witness. So a GM17 proof, with a message folded
into the statement, works as a short signature of knowledge.

The price is in the arithmetic. GM17 writes the circuit as a square arithmetic program, where each multiplication counts
as two squarings, so the prover does more work than Groth16 and the setup holds more elements. The verifier checks two
pairing equations, five pairings in all, against Groth16's three.

## History

- **June 2017.** Jens Groth and Mary Maller post *Snarky Signatures*; it appears at CRYPTO 2017. It also proves that
  three group elements and two verification equations are the minimum for a pairing-based scheme of this kind.
- **September 2023.** Last commit to arkworks' GM17 repository.
- **November 2023.** Last release of ZoKrates, which offers GM17 as a proving scheme and recommends non-malleable schemes
  such as GM17 where Groth16's malleability matters.

## Strengths

- **Non-malleable.** Seen proofs cannot be turned into new ones.
- **As small as Groth16.** Three group elements.
- **A signature of knowledge.** A proof can sign a message by including it in the statement.

## Weaknesses

- **Slower than Groth16** to prove and to verify, with a larger setup.
- **A trusted setup for every circuit**, as with Groth16.
- **Not post-quantum.** It rests on pairings.

## How next-generation it is

Not much. GM17 answered a precise question — how small a non-malleable SNARK can be — and answered it well, but the
field moved to universal setups, transparent systems and folding. Its lasting contribution is the optimality result and
the idea of SNARK-based signatures of knowledge.

## Status today (as of September 2026)

**Historical.** No production user is confirmed. Implementations remain in libsnark (`r1cs_se_ppzksnark`, a
modification of the protocol) and arkworks; ZoKrates, which offers it, describes itself as a proof-of-concept and has not
released since 2023.

## How this repository runs it

`crates/sys-gm17` uses `ark-gm17` 0.3.0 from arkworks, the published version, over BLS12-381. The museum's R1CS goes
through arkworks' constraint API row by row, with the public inputs as the verifier's inputs. Proofs are 192 bytes, the
same as Groth16's on this curve.

One thing this exhibit cannot show yet is the difference that defines GM17. The museum's byte-flip attack breaks the
encoding of a point, so both GM17 and Groth16 turn the result down as malformed. The attack that separates them is
negating two of the proof's points: `e(−A, −B) = e(A, B)`, so Groth16 accepts the altered proof and GM17 does not.

Try `/run gm17 one-plus-one`, then `/run groth16 one-plus-one`.

## Sources

- J. Groth, M. Maller, *Snarky Signatures: Minimal Signatures of Knowledge from Simulation-Extractable SNARKs*, CRYPTO 2017 — https://eprint.iacr.org/2017/540
- libsnark `r1cs_se_ppzksnark` — https://github.com/scipr-lab/libsnark/blob/master/libsnark/zk_proof_systems/ppzksnark/r1cs_se_ppzksnark/r1cs_se_ppzksnark.hpp
- arkworks GM17 — https://github.com/arkworks-rs/gm17
- ZoKrates proving schemes — https://zokrates.github.io/toolbox/proving_schemes.html
- ZoKrates repository — https://github.com/Zokrates/ZoKrates
