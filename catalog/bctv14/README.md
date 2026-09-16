# BCTV14 (Pinocchio)

The proof system Zcash launched with in 2016: a variant of Pinocchio, the pairing-based SNARK whose authors called it "nearly
practical". It protected Zcash's original Sprout pool for two years, until a flaw that would have let an attacker create
money from nothing was found — and removed, without announcement, by retiring the proof system.

> **Shelf:** Zcash · **First published:** 2013–2014 · **Trusted setup:** per circuit · **Zero-knowledge:** yes ·
> **Post-quantum:** no · **Runs here with:** a teaching implementation written for this repository

## What it is

Like [Groth16](../groth16/README.md), BCTV14 proves that someone knows a witness for a rank-1 constraint system by
turning the constraints into a quadratic arithmetic program and checking polynomial identities "in the exponent" with
pairings. The difference is how much the verifier has to be convinced of separately. A BCTV14 proof carries eight group
elements: three pairs of *knowledge commitments* (A, A′), (B, B′), (C, C′) that show each commitment was built from the
proving key rather than made up, one element K showing that A, B and C used the same coefficients, and one element H
that proves the divisibility of the constraint polynomial. The verifier checks each of those facts with its own
pairing equation.

It needs a per-circuit trusted setup, and the secrets of that setup — more of them than Groth16's — must be destroyed.

## History

- **2012–2013.** Gennaro, Gentry, Parno and Raykova introduce quadratic arithmetic programs (*GGPR*, EUROCRYPT 2013).
  Parno, Howell, Gentry and Raykova build *Pinocchio* on them (IEEE S&P 2013), a pairing SNARK its authors called
  "nearly practical".
- **2013–2014.** Ben-Sasson, Chiesa, Tromer and Virza publish a Pinocchio variant as part of *Succinct Non-Interactive
  Zero Knowledge for a von Neumann Architecture* (USENIX Security 2014) and implement it in the C++ library libsnark.
  Zcash's specification calls this variant BCTV14.
- **2015.** Bryan Parno finds a soundness problem in how BCTV14 translated Pinocchio; it is fixed in libsnark.
- **22–23 October 2016.** Six people generate Zcash's Sprout parameters in a multi-party ceremony. The parameters are
  safe unless all six were dishonest or compromised.
- **28 October 2016.** Zcash launches. Every Sprout JoinSplit carries a 296-byte BCTV14 proof over the BN254 curve.
- **1 March 2018.** Ariel Gabizon of the Electric Coin Company finds that key generation as the BCTV14 paper
  describes it — and as Zcash's parameter ceremony carried it out — published extra elements the prover never needed
  (libsnark's own key generator happened to leave them out). With them, a cheating prover could get past one of the
  consistency checks and forge proofs
  of false statements — in Zcash, creating shielded money from nothing (later CVE-2019-7167).
- **28 October 2018.** The Sapling upgrade moves all shielded proofs, including Sprout's, to Groth16, which removes the
  flaw without announcing it. Other chains built on Zcash's code are told privately in November.
- **5 February 2019.** The flaw is disclosed publicly. The Electric Coin Company reported finding no evidence that it had
  been exploited.

## Strengths

- **It worked, at scale, first.** Constant-size proofs and fast verification protected real value years before most
  proof systems left the lab, and libsnark trained a generation of engineers.
- **Small, constant-size proofs.** 296 bytes in Zcash's encoding, whatever the circuit.
- **Fast verification.** A fixed number of pairings.

## Weaknesses

- **Larger and slower than Groth16.** Eight group elements and more pairing checks, for the same job.
- **A per-circuit ceremony**, with more toxic waste to destroy.
- **Fragile setup.** The 2018 flaw came not from the proofs themselves but from key generation publishing a few
  elements too many. Nothing about a valid-looking proof would have revealed a forgery.
- **Not post-quantum.**

## How next-generation it is

Not at all today. It is the direct ancestor of Groth16, which kept its structure and cut the proof to three elements.
Its lasting lessons are historical: that SNARKs could secure real money, and that a trusted setup has to be audited as
carefully as the proof system itself.

## Status today (as of September 2026)

**Superseded** by Groth16 since 28 October 2018. We found no production system still using it. Zcash's Sprout pool, which it
once protected, still held about 22,600 ZEC at the end of August 2026 and has been closed to new deposits since the
Canopy upgrade; in a poll that closed on 14 September 2026 Zcash coinholders voted to disable Sprout transactions — a signal,
not yet a network rule.

## How this repository runs it

No permissively licensed Rust implementation exists — libsnark is C++ — so `crates/sys-bctv14` is a **teaching
implementation**, written from the papers and the Zcash protocol specification on top of the arkworks libraries for the
BN254 curve. It has not been audited. It generates the proving and verifying keys, the eight-element proof and the three
kinds of checks the verifier runs, each named after the fact it establishes. Its proofs use arkworks' compressed
encoding: 288 bytes, against Zcash's 296, because libff spends a separate tag byte on each of the eight points.

It also reproduces **CVE-2019-7167** as described in Gabizon's paper. A key generator that publishes the extra
elements lets anyone turn a valid proof for one public input into a proof for a different, false one, and the ordinary
verifier accepts it. From corrected keys, which lack those elements, the same rewrite is rejected.

Try `/run bctv14 one-plus-one`, then compare with `/run groth16 one-plus-one`: same statement, same kind of setup,
different proof sizes.

This repository is not affiliated with Zcash, the Electric Coin Company or the Zcash Foundation.

## Sources

- R. Gennaro, C. Gentry, B. Parno, M. Raykova, *Quadratic Span Programs and Succinct NIZKs without PCPs* — https://eprint.iacr.org/2012/215
- B. Parno, J. Howell, C. Gentry, M. Raykova, *Pinocchio: Nearly Practical Verifiable Computation* — https://eprint.iacr.org/2013/279
- E. Ben-Sasson, A. Chiesa, E. Tromer, M. Virza, *Succinct Non-Interactive Zero Knowledge for a von Neumann Architecture* — https://eprint.iacr.org/2013/879
- Zcash Protocol Specification, §5.4 — https://zips.z.cash/protocol/protocol.pdf
- Zcash Sprout parameter ceremony — https://github.com/zcash/mpc
- Zcash counterfeiting vulnerability successfully remediated, Electric Coin Company, 2019-02-05 — https://electriccoin.co/blog/zcash-counterfeiting-vulnerability-successfully-remediated/
- A. Gabizon, *On the security of the BCTV Pinocchio zk-SNARK variant* — https://eprint.iacr.org/2019/119
- ZIP 211, Disabling Addition of New Value to the Sprout Chain Value Pool — https://zips.z.cash/zip-0211
- Zcash community poll on NU7 — https://zfnd.org/zcap-poll-now-open-nu7/
