# UltraPlonk

PLONK with two additions from Aztec: custom gates that do more work per row, and lookup tables that check a value
against a precomputed list instead of taking it apart bit by bit. Noir programs were proved with it until Aztec
replaced it with UltraHonk in 2025.

> **Shelf:** Aztec · **First published:** 2020 · **Trusted setup:** universal and updatable · **Zero-knowledge:** yes ·
> **Post-quantum:** no · **Runs here with:** `jf-plonk` from Espresso Systems' jellyfish

## What it is

Plain [PLONK](../plonk/README.md) has one gate shape: `q_L·a + q_R·b + q_O·c + q_M·a·b + q_C = 0` on three wires.
**TurboPlonk** widens the row (to four wires in Aztec's implementation) and lets a gate read the next row as well, so
one row can do what used to take several — Aztec's example was fixed-base scalar multiplication at one gate per two
bits.

**plookup** adds a different kind of check. Some facts are expensive as arithmetic but trivial as a table: "this value
is below 256", or "these three bytes are an XOR". plookup proves that every value in a committed column appears in a
public table, by sorting the column together with the table and comparing the two with a grand-product argument, much
like PLONK's permutation argument.

**UltraPlonk** is TurboPlonk plus plookup, committed with KZG over BN254. The proof stays constant-size, and one
universal reference string serves every circuit.

## History

- **February 2020.** Aztec posts TurboPlonk benchmarks showing a five-fold prover speed-up over Groth16 on Pedersen
  hashes.
- **15 March 2020.** Ariel Gabizon and Zachary J. Williamson post plookup.
- **May 2020.** Gabizon presents the TurboPlonk program syntax at the ZKProof workshop.
- **September 2020.** Aztec describes UltraPlonk: PLONK with plookup gates.
- **March 2021.** Aztec 2.0's private rollup launches on TurboPlonk, and Aztec says UltraPlonk is close to deployment.
- **September 2021.** Aztec discloses bugs fixed in Aztec 2.0: several in its circuits, and two in its proving-system
  implementation, including a non-cryptographic random number generator used for secrets. Aztec states no funds were
  lost.
- **2023–2025.** Aztec moves to Honk. In March 2025 Noir switches its examples to UltraHonk, and on 20 May 2025 UltraPlonk
  is removed from Aztec's barretenberg prover.

## Strengths

- **Fewer gates.** Custom gates and lookups make hashes, range checks and bitwise operations far cheaper than in plain
  PLONK or R1CS.
- **One setup for all circuits**, as in PLONK, and constant-size proofs.
- **Practical for non-field algorithms.** Lookups make SHA-256 or AES-style operations affordable inside a circuit.

## Weaknesses

- **An FFT-based prover.** Aztec's own successor design notes that moving to sum-check removes FFTs, cutting prover time
  and memory at the cost of longer proofs.
- **A trusted setup**, if a universal one, and **not post-quantum**: it rests on pairings.
- **More moving parts.** Custom gates and lookup arguments are more code to get right than plain PLONK, and Aztec's
  2021 disclosure shows how implementation details matter.

## How next-generation it is

Its ideas are current; its prover is not. Custom gates and lookups are in nearly every modern proof system, and Aztec's
UltraHonk keeps the same circuit shape. What changed is the proving engine underneath: sum-check instead of univariate
quotients, and folding for client-side proving.

## Status today (as of September 2026)

**Superseded by UltraHonk.** Removed from barretenberg in May 2025; verification services such as zkVerify point to
older barretenberg releases for it. Independent Rust implementations, such as the one this exhibit uses, remain.

## How this repository runs it

`crates/sys-ultraplonk` uses `jf-plonk`, the UltraPlonk implementation in Espresso Systems' MIT-licensed jellyfish
library, over BN254. It builds jellyfish's circuit directly from the museum's gates: each addition, multiplication and
assertion becomes one of jellyfish's four-wire gates.

It also uses what makes UltraPlonk different. Wherever a statement checks that a value fits in a number of bits, the
exhibit replaces the museum's bit-by-bit check with jellyfish's range lookup against an 8-bit table, and a test confirms
that the rewritten circuit accepts and rejects exactly the same claims.

Every proof is 1,481 bytes. The reference string is drawn from your operating system's randomness for each statement: a
one-person setup. A false claim never reaches the verifier here — jellyfish's prover notices that the quotient
polynomial has the wrong degree and refuses to finish, and the museum shows that as the prover refusing.

Try `/run ultraplonk pool-spend`, then `/run plonk pool-spend` to see what lookups and wider gates save.

## Sources

- A. Gabizon, Z. J. Williamson, *plookup: A simplified polynomial protocol for lookup tables* — https://eprint.iacr.org/2020/315
- A. Gabizon, Z. J. Williamson, *Proposal: The Turbo-PLONK program syntax for specifying SNARK programs*, ZKProof — https://docs.zkproof.org/pages/standards/accepted-workshop3/proposal-turbo_plonk.pdf
- Aztec, TurboPlonk benchmarks (February 2020) — https://x.com/aztecnetwork/status/1230523630745980930
- Aztec, UltraPlonk primer (September 2020) — https://x.com/aztecnetwork/status/1303315879392874496
- Aztec, *Aztec's ZK-ZK-Rollup, looking behind the cryptocurtain* — https://aztec.network/blog/aztecs-zk-zk-rollup-looking-behind-the-cryptocurtain
- Aztec, *Vulnerabilities patched in Aztec 2.0* — https://aztec.network/blog/vulnerabilities-patched-in-aztec-2-0
- Noir moves examples to UltraHonk — https://github.com/noir-lang/noir/pull/7653
- UltraPlonk removed from barretenberg — https://github.com/AztecProtocol/aztec-packages/pull/14205
- zkVerify, UltraPlonk verification pallet — https://docs.zkverify.io/architecture/verification_pallets/ultraplonk
- Aztec CHONK design notes — https://github.com/AztecProtocol/aztec-packages/blob/next/barretenberg/cpp/src/barretenberg/chonk/README.md
- jellyfish — https://github.com/EspressoSystems/jellyfish
