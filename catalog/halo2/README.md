# Halo 2

A proving system with no trusted setup: PLONK-style circuits proved with an inner-product-argument commitment over
the Pasta curves. It replaced Groth16 in Zcash's newest pools and made "PLONKish" the circuit language a whole
generation of projects adopted.

> **Shelf:** Zcash · **First published:** 2020 · **Trusted setup:** none · **Zero-knowledge:** yes ·
> **Post-quantum:** no · **Runs here with:** `halo2_proofs`, Zcash's own crate

## What it is

Halo 2 has two halves.

- **The circuit language (arithmetization).** A circuit is a table: columns of values, rows of cells, and
  polynomial "gates" that must hold on every row where they are switched on. Cells can be tied together by copy
  constraints, and a lookup argument can require a value to appear in a table. This style, inherited from PLONK and
  extended, is what people now call *PLONKish*.
- **The commitment scheme.** The table's columns become polynomials, and the prover commits to them with an
  *inner product argument* (IPA) in the style of Bulletproofs. The IPA needs only a group where discrete logarithms
  are hard — no pairings and no secret setup. Anyone can generate the public parameters from nothing.

The curves are Pallas and Vesta ("Pasta"): each curve's group order is the other's field size. That cycle is what lets
one proof efficiently check another, the recursion idea Halo introduced in 2019.

## History

- **September 2019.** Sean Bowe, Jack Grigg and Daira Hopwood publish *Halo*, the first practical recursive proof
  composition without a trusted setup. It is research code and never ships.
- **1 September 2020.** The Electric Coin Company announces Halo 2, a new implementation combining PLONK-style
  arithmetization with Halo's commitment and accumulation ideas. It is released under the Bootstrap Open Source License.
- **7 April 2022.** Halo 2 is relicensed under MIT or Apache-2.0, under an agreement with the Filecoin Foundation.
  The Ethereum Foundation's zkEVM work was already using part of it, and forks later swapped the commitment scheme for
  KZG to get smaller proofs with a universal setup.
- **31 May 2022.** Zcash's NU5 upgrade activates the Orchard pool at block 1,687,104. For the first time, Zcash
  shielded transactions need no trusted setup at all.
- **29 May 2026.** An audit finds a missing copy constraint in a circuit gadget (`halo2_gadgets`, variable-base scalar
  multiplication) used by Orchard; it had allowed the same note to be spent twice under different nullifiers since
  2022. Zcash switches Orchard off by an emergency soft fork on 2 June and back on with a corrected circuit on 3 June
  (NU6.2). No exploitation was found. The flaw was in a circuit built *with* Halo 2, not in the proving system.
- **28 July 2026.** NU6.3 opens the Ironwood pool, which reuses Orchard's circuit and the Halo 2 proof system, and seals
  Orchard.

## Strengths

- **No trusted setup.** Nothing secret is generated, so there is nothing to destroy or to trust.
- **Expressive circuits.** Custom gates and lookups let a circuit designer make hashes, range checks and elliptic-curve
  arithmetic much cheaper than in plain R1CS.
- **Built for recursion.** The Pasta cycle and accumulation let proofs verify proofs without a pairing-friendly curve.
- **Proven in production.** It has protected Zcash's shielded pools since 2022, in Rust, under a permissive license.

## Weaknesses

- **Verification is not constant-time.** Checking an IPA opening takes time linear in the circuit size unless the work
  is accumulated and deferred, so a single Halo 2 proof is slower to verify than a Groth16 proof.
- **Bigger proofs.** Kilobytes rather than Groth16's 192 bytes.
- **Not post-quantum.** Security rests on discrete logarithms.
- **Circuits are easy to get subtly wrong.** The expressiveness that makes circuits cheap also makes a missing
  constraint easy to miss; the 2026 Orchard flaw went unnoticed for four years.

## How next-generation it is

In 2020 it was the next generation: no setup, recursion, and a circuit language flexible enough that the Ethereum
zkEVM efforts, Scroll, Axiom and others built on PLONKish tables. The frontier has since moved again — to STARKs and
zkVMs over small fields, to folding schemes, to sumcheck-based variants such as HyperPlonk and Aztec's Honk, and to
post-quantum constructions. Zcash's own next step, Project Tachyon, is built on Ragu, a new recursion framework derived
from the original Halo construction rather than on Halo 2.

## Status today (as of September 2026)

**Active.** Halo 2 proves every transaction in Zcash's Ironwood pool, which held about 3.84 million ZEC at the end of
August 2026, and Orchard's remaining withdrawals. Zcash's crate `halo2_proofs` 0.3.5 was released in August 2026. The
privacy-scaling-explorations KZG fork was archived; the Axiom fork is maintained.

## How this repository runs it

`crates/sys-halo2` uses `halo2_proofs` from the zcash/halo2 repository with its Pasta curves. Every example is lowered to
PLONKish rows and laid out in three advice columns with one generic gate
`q_l·a + q_r·b + q_o·c + q_m·a·b + q_c − instance = 0`, copy constraints for shared wires, and an instance column
that carries the public inputs.
Orchard's real circuit uses many custom gates and lookups instead; one generic gate keeps the circuit identical to what
every other system here proves, at the cost of more rows than a hand-tuned Halo 2 circuit would need.

Try `/run halo2 one-plus-one`, and compare its proof size and verification time with `/run groth16 one-plus-one`.

This repository is not affiliated with Zcash, the Electric Coin Company or the Zcash Foundation.

## Sources

- S. Bowe, J. Grigg, D. Hopwood, *Recursive Proof Composition without a Trusted Setup* (Halo) — https://eprint.iacr.org/2019/1021
- Announcing Halo 2, Zcash Community Forum, 2020-09-01 — https://forum.zcashcommunity.com/t/announcing-halo-2/37215
- Halo now licensed under MIT or Apache 2.0, Electric Coin Company, 2022-04-07 — https://electriccoin.co/blog/zero-knowledge-proving-system-halo-now-licensed-under-mit-making-it-available-for-anyone-to-use/
- NU5 activates on mainnet, Electric Coin Company — https://electriccoin.co/blog/nu5-activates-on-mainnet-eliminating-trusted-setup-and-launching-a-new-era-for-zcash/
- ZIP 224, Orchard Shielded Protocol — https://zips.z.cash/zip-0224
- ZIP 257 and the emergency soft fork and NU6.2 activation — https://zips.z.cash/zip-0257 · https://zfnd.org/zebra-4-5-3-and-5-0-0-emergency-soft-fork-and-nu6-2-activation/
- ZIP 258 and Zebra 6.0.0 (Ironwood) — https://zips.z.cash/zip-0258 · https://zfnd.org/zebra-6-0-0-release/
- Pine Analytics, Zcash quarterly report Q3 2026 — https://pineanalytics.substack.com/p/zcash-quarterly-report-q3-2026
- The halo2 book — https://zcash.github.io/halo2/
