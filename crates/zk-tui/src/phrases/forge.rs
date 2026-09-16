//! Words for the BCTV14 forgery, CVE-2019-7167.

zk_i18n::messages! {
    Title { en: "Forging BCTV14: CVE-2019-7167", ko: "BCTV14 위조: CVE-2019-7167" },
    Intro { en: "BCTV14 is the proof system Zcash Sprout ran. Its setup, as the paper describes it, publishes a few extra points. In 2018 Ariel Gabizon found that with those points anyone can turn a proof of a true claim into a proof of a false one; the flaw was disclosed in 2019.", ko: "BCTV14는 Zcash Sprout가 쓴 증명 시스템입니다. 논문에 적힌 대로의 설정은 점 몇 개를 더 공개합니다. 2018년 Ariel Gabizon이 그 점들로 누구든 참인 주장의 증명을 거짓 주장의 증명으로 바꿀 수 있음을 찾았고, 결함은 2019년에 공개됐습니다." },
    Keys { en: "Keys for one-plus-one from the flawed generator. They publish one extra point for each public input, {count} in all.", ko: "결함 있는 생성기로 만든 one-plus-one용 키. 공개 입력마다 점을 하나씩 더 공개해 모두 {count}개입니다." },
    Honest { en: "An honest proof of the true claim, the envelope that holds 2: accepted.", ko: "참인 주장, 곧 2가 든 봉투에 대한 정직한 증명: 받아들여짐." },
    HonestRejected { en: "The honest proof of the true claim was rejected.", ko: "참인 주장에 대한 정직한 증명이 거절됐습니다." },
    HonestForFalse { en: "The same proof offered for the false claim, an envelope that holds 3: rejected.", ko: "같은 증명을 거짓 주장, 곧 3이 든 봉투에 내밀면: 거절됨." },
    HonestForFalseAccepted { en: "The same proof offered for the false claim was accepted.", ko: "같은 증명을 거짓 주장에 내밀었더니 받아들여졌습니다." },
    Rewritten { en: "The proof rewritten with the extra points: the ordinary verifier accepts it for the false claim.", ko: "더 공개된 점으로 고쳐 쓴 증명: 평범한 검증자가 거짓 주장에 대해 받아들입니다." },
    RewrittenRejected { en: "The rewritten proof was rejected for the false claim.", ko: "고쳐 쓴 증명이 거짓 주장에 대해 거절됐습니다." },
    Corrected { en: "The same rewrite with corrected keys, which leave the extra points out: rejected.", ko: "더 공개된 점을 뺀 고친 키에 같은 고쳐 쓰기를 하면: 거절됨." },
    CorrectedAccepted { en: "The same rewrite with corrected keys was accepted.", ko: "고친 키에 같은 고쳐 쓰기를 했더니 받아들여졌습니다." },
    Lesson { en: "Leaving the extra points out of the setup closes the attack. The verifier stays the same.", ko: "설정에서 그 점들을 빼면 공격이 막힙니다. 검증자는 그대로입니다." },
    Unexpected { en: "The forgery did not come out the way its design says it must. This is a fault in the program.", ko: "위조가 설계대로 나오지 않았습니다. 프로그램의 결함입니다." },
    Stopped { en: "Stopped after {done} of {total} steps.", ko: "{total}단계 중 {done}단계 뒤에 멈췄습니다." },
    WorkingKeys { en: "Making keys with the flawed generator", ko: "결함 있는 생성기로 키 만드는 중" },
    WorkingHonest { en: "Proving the true claim", ko: "참인 주장 증명 중" },
    WorkingFalse { en: "Offering the proof for the false claim", ko: "증명을 거짓 주장에 내미는 중" },
    WorkingRewrite { en: "Rewriting the proof", ko: "증명 고쳐 쓰는 중" },
    WorkingCorrected { en: "Rewriting against corrected keys", ko: "고친 키에 대해 고쳐 쓰는 중" },
    Failed { en: "The forgery could not run: {why}", ko: "위조를 진행하지 못했습니다: {why}" },
}
