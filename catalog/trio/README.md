# Trio

A zero-knowledge proof designed for this repository that needs nothing but a hash function. Where
[Ali Baba's cave](../cave/README.md) relies on a magic wall everyone must trust, Trio lets three imaginary friends
compute the statement together on scrambled pieces of the secret, and lets the verifier peek at any two of them.

> **Shelf:** Homemade · **Designed:** 2026 · **Trusted setup:** none · **Zero-knowledge:** yes ·
> **Post-quantum:** believed so (hash only; not formally analysed) · **Runs here with:** code written for this repository

## What it is

The prover splits every secret value of the circuit into three random numbers that add up to it and hands one to each
of three imaginary friends. Any two of the three pieces look like random noise; only all three together reveal the
value. The friends then run the circuit together:

- **Additions** each friend does on their own piece.
- **Multiplications** need help, so before the computation starts a **dealer** gives the friends matching pieces of
  a random multiplication card `(a, b, c = a·b)`. With a card, the friends multiply by announcing only
  `x − a` and `y − b`, which reveal nothing because `a` and `b` are random (Beaver's trick, 1991).
- **Checks** that a value is zero: each friend announces their piece, and the three must add up to zero.

The prover commits (with SHA-256) to everything the dealer handed out and everything each friend saw and said. Then
the verifier draws one card from five:

- **Two dealer cards**: open everything the dealer handed out, and check that every multiplication card really has
  `c = a·b`.
- **Three peek cards**, one per friend: hide that friend, open the other two, and redo their whole computation to
  check every announcement and every commitment — including the commitment to what the hidden friend announced.

A cheater has to lie somewhere. A bad multiplication card is only caught by a dealer card: it survives three draws out
of five. A friend who miscalculates is caught whenever they are opened: they survive the two dealer cards and the card
that hides them, again three out of five. So every round a cheater survives with probability at most **3/5**, and 109
rounds leave less than 2^-80. Two dealer cards rather than one is not decoration: with a dealer card chosen with
probability `q`, the cheater's best chance is `max(1 − q, q + (1 − q)/3)`, which is smallest at `q = 2/5`.

Nothing leaks: a dealer card opens only random cards that have nothing to do with the secret, and a peek card opens
two pieces out of three and announcements masked by the hidden friend's random card.

## History

- **2007–2008.** Ishai, Kushilevitz, Ostrovsky and Sahai show that a zero-knowledge proof can be built by simulating a
  multi-party computation "in the head" and opening some of the parties' views.
- **2016.** *ZKBoo* makes the idea practical for real circuits: three parties, a verifier who opens two of them, 2/3 a
  round. It leads to *ZKB++* and the post-quantum signature scheme *Picnic*.
- **2018.** *KKW* (Katz, Kolesnikov and Wang) adds a preprocessing phase checked by cut-and-choose.
- **September 2026.** Trio is designed for this repository as a simple member of the preprocessing family: one dealer,
  three friends, five cards. **The first draft was broken.** It committed to what each friend announced in a way the
  verifier could not recheck for the hidden friend, so a prover who saw the card could rewrite the hidden friend's
  zero-check pieces and make any false statement pass every round. The flaw was found by review before a line of code
  was written. The fix puts a key between each friend's seed and their announcements, so the verifier can recompute
  the hidden friend's commitment too. The broken prover is kept as a cheat you can watch being caught.

## Strengths

- **Needs only a hash function.** No curves, no pairings, no setup, nothing secret generated before the first proof.
- **Proves any circuit.** Every example in this museum, unchanged.
- **Small enough to follow.** One round is a handful of additions, multiplications and hashes; the interactive
  version shows every card drawn and every check made.
- **Hash-based.** Hash functions are believed to resist quantum computers.

## Weaknesses

- **Big proofs.** Each round carries two friends' whole computation. On the machine this museum was built on, a
  non-interactive proof was about 93 KB for `one-plus-one` and about 1.2 MB for `pool-spend`. Verification is linear in
  the circuit size.
- **Many rounds.** 3/5 a round is a weak start; 109 rounds are needed for 2^-80.
- **Homemade and unaudited.** Its soundness and zero-knowledge arguments are the informal ones above; its first draft
  was wrong. Never use it to protect anything.

## How next-generation it is

Trio is a teaching design, not a contender. Its family is very much alive: the proofs "in the head" behind Picnic were
not selected by NIST in 2022, but newer members — VOLE-in-the-head schemes such as FAEST, and SDitH and MQOM — were
among the nine candidates NIST moved to the third round of its additional post-quantum signature process in May 2026.

## Status today (as of September 2026)

**Experimental.** Written for this repository; not used anywhere else.

## How this repository runs it

`crates/zk-trio` works directly on the circuit's gates over the Goldilocks field, with SHA-256 for every commitment and
the operating system's randomness for every seed.

- `/run trio one-plus-one` makes a 109-round non-interactive proof (Fiat–Shamir: the cards are drawn from a hash of the
  statement and all commitments) and runs the museum's attacks on it.
- `/trio` plays the interactive version, 55 rounds, card by card; lets you watch the three cheats get caught — a bad
  multiplication card, a friend who miscalculates, and the rewrite that broke the first draft; and runs the simulator,
  which produces accepted rounds without the secret when it knows the cards in advance, the same argument as the cave's
  edited tape.

## Sources

- Y. Ishai, E. Kushilevitz, R. Ostrovsky, A. Sahai, *Zero-Knowledge from Secure Multiparty Computation*, STOC 2007 — https://doi.org/10.1145/1250790.1250794
- I. Giacomelli, J. Madsen, C. Orlandi, *ZKBoo: Faster Zero-Knowledge for Boolean Circuits*, USENIX Security 2016 — https://eprint.iacr.org/2016/163
- M. Chase et al., *Post-Quantum Zero-Knowledge and Signatures from Symmetric-Key Primitives* (ZKB++ and Picnic), CCS 2017 — https://eprint.iacr.org/2017/279
- D. Katz, V. Kolesnikov, X. Wang, *Improved Non-Interactive Zero Knowledge with Applications to Post-Quantum Signatures*, CCS 2018 — https://eprint.iacr.org/2018/475
- D. Beaver, *Efficient Multiparty Protocols Using Circuit Randomization*, CRYPTO '91 — https://doi.org/10.1007/3-540-46766-1_34
- NIST IR 8413, status report on the third round of the PQC standardization process (2022) — https://nvlpubs.nist.gov/nistpubs/ir/2022/NIST.IR.8413.pdf
- Nine schemes advance to round 3 of NIST's additional digital signatures process (2026) — https://www.projecteleven.com/blog/nine-schemes-advance-to-round-3-of-nists-additional-digital-signatures-process
