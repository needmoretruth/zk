//! Words for the BCTV14 forgery, CVE-2019-7167.

zk_i18n::messages! {
    Title { en: "Forging BCTV14: CVE-2019-7167" },
    Intro { en: "BCTV14 is the proof system Zcash Sprout ran. Its setup, as the paper describes it, publishes a few extra points. In 2019 Ariel Gabizon showed that with those points anyone can turn a proof of a true claim into a proof of a false one." },
    Keys { en: "Keys for one-plus-one from the flawed generator. They publish one extra point for each public input, {count} in all." },
    Honest { en: "An honest proof of the true claim, the envelope that holds 2: accepted." },
    HonestRejected { en: "The honest proof of the true claim was rejected." },
    HonestForFalse { en: "The same proof offered for the false claim, an envelope that holds 3: rejected." },
    HonestForFalseAccepted { en: "The same proof offered for the false claim was accepted." },
    Rewritten { en: "The proof rewritten with the extra points: the ordinary verifier accepts it for the false claim." },
    RewrittenRejected { en: "The rewritten proof was rejected for the false claim." },
    Corrected { en: "The same rewrite with corrected keys, which leave the extra points out: rejected." },
    CorrectedAccepted { en: "The same rewrite with corrected keys was accepted." },
    Lesson { en: "Leaving the extra points out of the setup closes the attack. The verifier stays the same." },
    Unexpected { en: "The forgery did not come out the way its design says it must. This is a fault in the program." },
    Stopped { en: "Stopped after {done} of {total} steps." },
    WorkingKeys { en: "Making keys with the flawed generator" },
    WorkingHonest { en: "Proving the true claim" },
    WorkingFalse { en: "Offering the proof for the false claim" },
    WorkingRewrite { en: "Rewriting the proof" },
    WorkingCorrected { en: "Rewriting against corrected keys" },
    Failed { en: "The forgery could not run: {why}" },
}
