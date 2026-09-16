# Schnorr and sigma protocols

The oldest proof system in the museum that still signs real transactions. A Schnorr proof shows that you know the secret
behind a public key in three messages; hashed into one, it becomes the kind of signature that authorizes shielded spends
in Zcash and Taproot spends in Bitcoin.

> **Shelf:** Zcash · **First published:** 1989 · **Trusted setup:** none · **Zero-knowledge:** yes (honest verifier;
> statistical once non-interactive) · **Post-quantum:** no · **Runs here with:** `sigma-proofs`, plus a circuit compiler
> written for this repository

## What it is

Fix a group where discrete logarithms are hard, with generator `G`. You claim to know `x` with `X = x·G`.

1. **Commit.** You pick a random `r` and send `R = r·G`.
2. **Challenge.** The verifier sends a random number `c`.
3. **Respond.** You send `s = r + c·x`, and the verifier checks `s·G = R + c·X`.

Answering two different challenges for the same `R` would reveal `x`, so someone who does not know `x` can answer at most
one of them: a cheater survives with probability one in the size of the challenge space. And the conversation `(R, c, s)`
can be produced without `x` — pick `c` and `s` first, then set `R = s·G − c·X` — so it teaches the verifier nothing.

Three moves shaped like this — commit, challenge, respond — are called **sigma protocols**. They compose: AND of two
statements, OR of two statements, or any statement of the form "these group elements are these linear combinations of
secrets". Replacing the verifier's challenge with a hash of the statement and the commitment (the **Fiat–Shamir**
transform) turns the conversation into a proof anyone can check later; hash a message in too, and it is a signature.

Sigma protocols are not succinct. A proof about a big computation is as big as the computation, which is why Zcash uses
them to sign spends and uses SNARKs to prove them.

## History

- **1986.** Fiat and Shamir show how to turn an interactive identification scheme into a signature with a hash function.
- **1989.** Claus-Peter Schnorr presents efficient identification and signatures for smart cards at CRYPTO '89, based on
  discrete logarithms; the journal version follows in 1991. His US patent on the scheme runs until 2008.
- **1994.** Cramer, Damgård and Schoenmakers show how to combine such proofs into proofs of "one of these statements is
  true", and the three-move shape comes to be studied as a family: sigma protocols.
- **2012.** "How not to Prove Yourself" shows that Fiat–Shamir done carelessly, without hashing the whole statement, broke
  the Helios voting system.
- **28 October 2018.** Zcash's Sapling upgrade introduces RedJubjub, a Schnorr-based signature with re-randomizable keys,
  for spend authorization and for the binding signature that balances each transaction.
- **November 2021.** Bitcoin's Taproot upgrade activates BIP340 Schnorr signatures.
- **April 2022.** Trail of Bits discloses "Frozen Heart": weak Fiat–Shamir in several proof-system implementations.
- **31 May 2022.** Zcash's Orchard pool launches with RedPallas, the same scheme on the Pallas curve.
- **June 2024.** RFC 9591 standardizes FROST, two-round threshold Schnorr signatures.
- **2026.** The IRTF's crypto research group works on drafts for sigma proofs of linear relations and for the Fiat–Shamir
  transform; the Zcash Foundation releases FROST 3.0.0.

## Strengths

- **Tiny and simple.** A signature is 64 bytes; verification is two scalar multiplications, and many signatures can be
  checked together.
- **No trusted setup** and nothing to keep secret except your own key.
- **Composable.** AND, OR and linear relations over any number of secrets, each with a short, well-understood proof.
- **Re-randomizable keys.** Zcash authorizes a spend with a fresh-looking key each time, so two spends by the same owner
  cannot be linked by their signatures.

## Weaknesses

- **Not succinct.** Proving a general computation this way costs work and bytes in proportion to its size.
- **Only honest-verifier zero-knowledge when interactive.** The non-interactive version is zero-knowledge in the random
  oracle model.
- **Fiat–Shamir must hash everything.** Leaving the statement out of the hash has broken real systems.
- **Fragile in groups.** Blind and multi-party variants need great care; the 2020 ROS attack broke several of them.
- **Not post-quantum.** A quantum computer running Shor's algorithm computes discrete logarithms.

## How next-generation it is

The idea is 37 years old and is not going anywhere: threshold signing (FROST), many anonymous-credential schemes and
many building blocks inside larger proof systems are sigma protocols. For whole computations, it was overtaken long ago by
SNARKs and STARKs. Its future is bounded by quantum computers, like everything built on discrete logarithms.

## Status today (as of September 2026)

**Active.** Zcash's Sapling and Orchard spends are authorized with RedDSA signatures, and Bitcoin's Taproot spends with
BIP340 signatures. The IRTF drafts are still drafts.

## How this repository runs it

`crates/sys-schnorr` has two layers.

- **A Schnorr proof of knowledge**, the core of a RedPallas signature, made with `sigma-proofs` on the Pallas curve.
- **The seven statements**, which a single Schnorr proof cannot express. A small compiler written for this repository —
  a teaching implementation, not audited — turns a circuit into one large sigma protocol, and `sigma-proofs` proves it.
  Every secret wire gets a Pedersen commitment `v·G + r·H`, where nobody knows how `H` relates to `G`. Additions need no
  proof, because commitments add. Each multiplication gets a small proof that one committed value is the product of two
  others, and each assertion a proof that a commitment hides zero.

The cost of not being succinct shows at once. The Schnorr proof alone is 64 bytes, but proofs of the statements run from
6,336 bytes for `one-plus-one` to 126,560 bytes for `pool-spend`: 32 bytes for every commitment, every equation and every
secret. Proving and verifying `pool-spend` each take seconds.

Try `/run schnorr one-plus-one`, then `/run groth16 one-plus-one` to see what succinctness buys.

This repository is not affiliated with Zcash, the Electric Coin Company or the Zcash Foundation.

## Sources

- A. Fiat, A. Shamir, *How To Prove Yourself*, CRYPTO '86 — https://doi.org/10.1007/3-540-47721-7_12
- C. P. Schnorr, *Efficient Identification and Signatures for Smart Cards*, CRYPTO '89 — https://doi.org/10.1007/0-387-34805-0_22
- C. P. Schnorr, *Efficient signature generation by smart cards*, Journal of Cryptology 1991 — https://doi.org/10.1007/BF00196725
- US patent 4,995,082 — https://cr.yp.to/patents/us/4995082.html
- R. Cramer, I. Damgård, B. Schoenmakers, *Proofs of Partial Knowledge and Simplified Design of Witness Hiding Protocols*, CRYPTO '94 — https://doi.org/10.1007/3-540-48658-5_19
- D. Bernhard, O. Pereira, B. Warinschi, *How not to Prove Yourself*, ASIACRYPT 2012 — https://eprint.iacr.org/2016/771
- Zcash Protocol Specification, §5.4.7 (RedDSA) — https://zips.z.cash/protocol/protocol.pdf
- BIP340, Schnorr Signatures for secp256k1 — https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki
- Trail of Bits, *Frozen Heart* disclosure — https://blog.trailofbits.com/2022/04/13/part-1-coordinated-disclosure-of-vulnerabilities-affecting-girault-bulletproofs-and-plonk/
- F. Benhamouda et al., *On the (in)security of ROS* — https://eprint.iacr.org/2020/945
- RFC 9591, FROST — https://datatracker.ietf.org/doc/rfc9591/
- IRTF CFRG, *Interactive Sigma Proofs* (draft) — https://datatracker.ietf.org/doc/draft-irtf-cfrg-sigma-protocols/
- Zcash Foundation FROST releases — https://github.com/ZcashFoundation/frost/releases
- sigma-proofs — https://github.com/sigma-rs/sigma-proofs
