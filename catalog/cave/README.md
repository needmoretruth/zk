# Ali Baba's cave

The story that has explained zero-knowledge to more people than any paper: a man proves he knows the words that open a
secret wall inside a cave, forty times over, without ever saying them — and a forged videotape shows why his proof
convinces the person who watched it and nobody else. This exhibit follows the original 1989 story, not the popular
retelling.

> **Shelf:** Homemade · **Story first published:** 1989 · **Trusted setup:** a trusted component (the wall) ·
> **Zero-knowledge:** yes · **Post-quantum:** not a cryptographic question · **Runs here with:** code written for this repository

## What it is

In the original story the cave is **not a ring**. Its entrance forks into two winding passages, one to the left and
one to the right, and both end in a dead end. What nobody can see is that the two dead ends are separated by a wall
that slides open when someone whispers the magic words. Whoever knows the words can walk in down one passage and come
out of the other.

A demonstration goes like this. The prover walks alone into one passage of his choosing. A reporter follows only as
far as the fork, where the reporter cannot see which way he went, flips a coin, and calls out which side he must come back
from. If he knows the words he can always obey. If he does not, he can obey only when he happened to pick that side
already — half the time. After forty calls, someone without the words survives with probability one in 2^40, about
one in a trillion.

The reporter learns nothing about the words: all the reporter sees is a man appearing on the side that was called. And the
recording of it proves nothing to anyone else, as the second half of the story shows.

## History

The story is *How to Explain Zero-Knowledge Protocols to Your Children* by Jean-Jacques Quisquater and Louis Guillou,
written with their families and with Tom Berson for the English version, published at CRYPTO '89. Retold briefly:

- **The thieves.** In old Baghdad, a thief snatches Ali Baba's purse and runs into the forked cave. Ali Baba searches
  both passages and finds nobody. The same happens with a different thief every day for forty days. Forty escapes by
  luck would be absurdly unlikely, so he hides at the end of the right-hand passage, hears a thief whisper
  "Open sesame", and watches the wall slide open. He changes the words, catches the next thief, and records the story
  in a manuscript with only hints about the new words.
- **Mick Ali.** Centuries later researchers decode the hints and the cave is found again. Mick Ali, perhaps a
  descendant, wants to show on television that he knows the words without revealing them. The crew films the empty
  passages, everyone leaves, Mick goes in alone, and the reporter at the fork flips a coin and calls a side — forty
  times, in memory of the forty thieves. He never fails.
- **The edited tape.** A jealous reporter from another channel hires a look-alike actor who does not know the words,
  films the same routine, throws away every scene where the actor came out of the wrong side — about half — and keeps
  cutting until forty successes remain. Both tapes air; in court nobody can tell the real one from the fake. Since the
  fake plainly contains no knowledge of the words, neither does the real one.
- **Afterwards.** The paper goes on to caves on every floor of an apartment building tested at once, to a reporter and
  an actor agreeing the calls in advance, and to a cave with more passages.

Popular versions draw a ring-shaped cave, call the characters Peggy and Victor, and often have the prover toss the
coin. The paper does none of these.

## Strengths

- **No mathematics needed.** It shows all three properties of a zero-knowledge proof: an honest prover always
  convinces (completeness), a cheater is caught with overwhelming probability (soundness), and the verifier learns
  nothing but the fact (zero knowledge).
- **It explains the definition, not just the idea.** The edited tape is exactly the *simulator* that formal definitions of
  zero knowledge use: if a transcript can be produced without the secret, the transcript cannot contain the secret.
- **It explains non-transferability.** A live, interactive proof convinces the verifier who chose the challenges and
  nobody who merely watches a recording.

## Weaknesses

- **Everything rests on a magic wall.** The wall checks the words and tells no one. Software has no such wall: in this
  exhibit it is a component everyone has to trust, which is precisely what real proof systems exist to avoid. See
  [Trio](../trio/README.md) for the same idea with the wall replaced by a hash function.
- **Interactive.** The verifier must be there and must flip honest coins.
- **Slow soundness.** Each scene halves a cheater's chances; forty scenes are needed for one in a trillion.

## How next-generation it is

It is a story from 1989 and makes no claim to be a proof system. Its central device — a simulated transcript that is
indistinguishable from the real one — is at the core of every security proof in this museum, from Schnorr
signatures to zkVMs.

## Status today (as of September 2026)

**Historical.** Still the most widely used explanation of zero knowledge, usually in the altered popular form.

## How this repository runs it

`crates/zk-cave` implements the cave as the story describes it. The wall's lock is one of the museum's seven example
circuits: the words Mick whispers are that example's secret inputs, and the wall opens only if they satisfy the
circuit. The verifier-side code cannot read the wall's lock — Rust's module privacy enforces that — which is the
program's way of saying where the trust lives. The coin is your operating system's randomness.

`/cave` plays the story: Mick's forty scenes, the look-alike caught early, the jealous reporter's edit and how many takes
were thrown away, the courtroom comparison of the two tapes, the prior-agreement fake and the apartment building.
`/run cave one-plus-one` runs the same cave through the museum's standard harness: the words that open the wall are
the answer to 1 + 1 and its salt, and the reporter never hears them.

The story is retold here in our own words; read the original for its charm.

## Sources

- J.-J. Quisquater, L. Guillou et al., *How to Explain Zero-Knowledge Protocols to Your Children*, CRYPTO '89, LNCS 435,
  pp. 628–631 — https://doi.org/10.1007/0-387-34805-0_60
- S. Goldwasser, S. Micali, C. Rackoff, *The Knowledge Complexity of Interactive Proof-Systems*, STOC '85 (the simulator
  definition of zero knowledge) — https://doi.org/10.1145/22145.22178
