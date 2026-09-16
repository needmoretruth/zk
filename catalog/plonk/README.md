# PLONK

The zk-SNARK Aztec published in 2019 that made one trusted setup serve every circuit. Its way of writing circuits —
rows of wires switched by selectors, glued by copy constraints — became the common language of Halo 2, Kimchi and
Aztec's own later systems.

> **Shelf:** Aztec · **First published:** 2019 · **Trusted setup:** universal and updatable · **Zero-knowledge:** yes ·
> **Post-quantum:** no · **Runs here with:** lambdaworks' PLONK prover

## What it is

A PLONK circuit is a table. Every row is one gate with three wires `a, b, c` and five selectors that say what the gate
does: `q_L·a + q_R·b + q_O·c + q_M·a·b + q_C = 0`. Addition, multiplication, a constant or a public input are all the
same equation with different selectors. Copy constraints say which cells hold the same value, and PLONK proves them all
at once with a permutation argument.

The prover turns each column into a polynomial over a multiplicative subgroup — the "Lagrange bases" of the name — and
commits to it with KZG polynomial commitments. It then shows that the gate equation and the permutation hold everywhere
on the subgroup by opening the polynomials at one random point. The proof is a constant number of curve points and field
elements, and the verifier does two pairings whatever the circuit size.

The setup is **universal**. Groth16 needs a new ceremony for every circuit. PLONK needs one list of powers of a secret
`τ`, and that list serves every circuit up to its size. It is also **updatable**: anyone can mix in more randomness later,
and the list stays safe as long as one contributor was honest.

## History

- **2018.** Groth, Kohlweiss, Maller, Meiklejohn and Miers show that a SNARK can use a universal, updatable reference
  string. Sonic (2019) is the first potentially practical one, but its fully succinct mode is slow to prove.
- **21 August 2019.** Ariel Gabizon, Zachary J. Williamson and Oana Ciobotaru of Aztec post PLONK. Working with
  evaluations on a subgroup instead of coefficients simplifies both the permutation argument and the arithmetization,
  and needs far fewer group exponentiations than Sonic.
- **25 October 2019 – 2 January 2020.** Aztec's Ignition ceremony: 176 participants build a KZG reference string over
  BN254 that Aztec still uses.
- **2020.** TurboPlonk adds custom gates, and plookup (Gabizon and Williamson) adds lookup tables; together they make
  UltraPlonk.
- **March 2021.** Aztec 2.0's private rollup launches on TurboPlonk.
- **2022–2023.** The PLONK family spreads. Zcash's Orchard goes live in May 2022 on Halo 2, which writes PLONK-style
  circuits without a trusted setup. In March 2023 Polygon zkEVM's mainnet beta wraps its final proof in fflonk, a PLONK
  variant by Gabizon and Williamson.
- **2023–2025.** Aztec moves to Honk, which keeps PLONK-style circuits but replaces the univariate quotient with sum-check.
  UltraPlonk is removed from Aztec's barretenberg prover on 20 May 2025.

## Strengths

- **One setup for all circuits.** A reference string made once — by a ceremony with hundreds of participants, if you like —
  serves every circuit up to its size, and can be strengthened later.
- **Flexible circuits.** New gate types and lookup tables slot into the same table, which is how TurboPlonk and UltraPlonk
  got fast hashes and range checks.
- **Small, cheap-to-verify proofs.** A constant size and two pairings, independent of the circuit.

## Weaknesses

- **Still a trusted setup.** Universal is not none: if every contributor to the reference string kept their secret,
  proofs can be forged.
- **Bigger and slower than Groth16.** A PLONK proof carries several times more elements than Groth16's three, and the
  prover's polynomial arithmetic costs more than Groth16's for the same R1CS-shaped circuit.
- **Not post-quantum.** Its security rests on pairings over elliptic curves.

## How next-generation it is

The protocol itself is no longer where the frontier is. Its arithmetization is: nearly every modern circuit language
writes PLONK-style tables. The provers behind them have moved on — to sum-check over the boolean hypercube (HyperPlonk,
Aztec's Honk), to IPA commitments without a trusted setup (Halo 2), or to folding for client-side proving (Aztec's CHONK).

## Status today (as of September 2026)

**Active as a family, rarely as plain PLONK.** The original paper was last revised in July 2025. Aztec, where it was
born, now proves with UltraHonk and CHONK over the same Ignition reference string, and removed UltraPlonk in 2025. PLONK
variants remain in production elsewhere: ZKsync verifies fflonk proofs, and Mina runs on Kimchi.

## How this repository runs it

`crates/sys-plonk` uses the PLONK prover from lambdaworks, an Apache-2.0 cryptography library by LambdaClass, over
BLS12-381. The museum's circuits are already PLONK tables: each row becomes lambdaworks' selector values, and the copy
classes become its permutation. Two things differ from the paper.
- lambdaworks splits the quotient polynomial differently and writes curve points uncompressed. Its proofs are therefore
  1,620 bytes for every example rather than the paper's more compact encoding.
- Its decoder accepts several byte strings as the same proof. The museum accepts only the exact bytes the prover wrote.

The setup draws `τ` from your operating system and builds a fresh reference string for each statement: a one-person
ceremony, so you are trusting your own computer.

Try `/run plonk pool-spend`, then `/run groth16 pool-spend` to see the price of a universal setup.

## Sources

- A. Gabizon, Z. J. Williamson, O. Ciobotaru, *PLONK: Permutations over Lagrange-bases for Oecumenical Noninteractive arguments of Knowledge* — https://eprint.iacr.org/2019/953
- J. Groth, M. Kohlweiss, M. Maller, S. Meiklejohn, I. Miers, *Updatable and Universal Common Reference Strings with Applications to zk-SNARKs*, CRYPTO 2018 — https://eprint.iacr.org/2018/280
- M. Maller, S. Bowe, M. Kohlweiss, S. Meiklejohn, *Sonic*, CCS 2019 — https://eprint.iacr.org/2019/099
- A. Gabizon, Z. J. Williamson, *plookup* — https://eprint.iacr.org/2020/315
- Aztec Ignition ceremony verification — https://github.com/AztecProtocol/ignition-verification
- Aztec, *Launching Aztec 2.0 Rollup* — https://aztec.network/blog/launching-aztec-2-0-rollup
- UltraPlonk removed from barretenberg — https://github.com/AztecProtocol/aztec-packages/pull/14205
- NU5 activation (Orchard on Halo 2) — https://electriccoin.co/blog/nu5-activates-on-mainnet-eliminating-trusted-setup-and-launching-a-new-era-for-zcash/
- L2BEAT ZK catalog, Polygon zkEVM prover — https://l2beat.com/zk-catalog/zkprover
- A. Gabizon, Z. J. Williamson, *fflonk* — https://eprint.iacr.org/2021/1167
- ZKsync protocol v27, Plonk and fflonk verifier — https://github.com/zkSync-Community-Hub/zksync-developers/discussions/980
- o1Labs, *Reintroducing Kimchi* — https://www.o1labs.org/blog/reintroducing-kimchi
- lambdaworks — https://github.com/lambdaclass/lambdaworks
