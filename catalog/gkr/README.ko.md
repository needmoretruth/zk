# GKR

층으로 나뉜 산술 회로를 층마다 sumcheck 한 번씩으로 확인해서, 검증자가 회로를 직접 계산하는 것보다 훨씬 적게 일하게 하는 대화형
증명입니다. 그 자체로는 아무것도 가리지 않습니다. Hyrax, Libra, Virgo가 약속을 더해 영지식 논증으로 바꿨고, 오늘은 다른 증명자
안에서 LogUp-GKR로 돕니다.

> **선반:** Others · **처음 발표:** 2008 · **신뢰 설정:** 없음 · **영지식:** 예(Hyrax와 함께) ·
> **양자 내성:** 약속에 따라 다름 · Hyrax로는 아니오 · **이 저장소에서 돌리는 코드:** Worldcoin의 Remainder(GKR + Hyrax)

## 무엇인가

**층을 이룬 회로**는 게이트가 층으로 배치돼, 게이트마다 바로 아래 층의 와이어 둘을 읽는 회로입니다. GKR은 출력 층에 대한 주장 —
예를 들어 모든 출력이 0이라는 주장 — 에서 시작합니다. **sumcheck 프로토콜**로 그 주장을 아래 층에 대한 주장으로 바꿉니다.
sumcheck는 증명자가 변수마다 작은 다항식 하나를 보내고 검증자가 그것을 앞의 것과 대조하는 대화입니다. 층마다 sumcheck를 한 번
하고 나면 입력에 대한 주장이 남고, 검증자가 그것을 직접 확인합니다.

검증자의 일은 회로의 크기가 아니라 깊이와 너비의 로그에 따라 늘어납니다. 증명자는 회로에 비례하는 일을 하고, 뒤의 논문들이 그것을
선형 시간으로 줄였습니다.

GKR 그대로는 **위임한 계산**의 증명입니다. 검증자가 입력을 전부 알아야 합니다. 비밀 입력을 안다는 것을 증명하려면 입력 대신 입력에
대한 약속을 두고, 마지막 확인을 그 약속의 열기로 합니다. Hyrax는 Pedersen 약속으로 이것을 하고 영지식을 더했고, Libra는 가리는
다항식을 더했으며, Virgo는 해시 기반 약속을 썼습니다.

## 역사

- **2008년.** Shafi Goldwasser, Yael Tauman Kalai, Guy Rothblum이 STOC에서 *Delegating Computation: Interactive Proofs for
  Muggles*를 발표합니다. 학술지 버전은 2015년에 나옵니다.
- **2013년.** Justin Thaler의 CRYPTO 논문이 규칙적인 회로에서 증명자를 선형 시간으로 줄입니다.
- **2018년.** Hyrax가 이산 로그 약속으로 GKR을 영지식 논증으로 바꿉니다.
- **2019~2020년.** Libra가 한 번의 설정으로 선형 시간 증명자에 이르고, Virgo가 해시 기반 약속으로 설정을 없앱니다.
- **2023년 8월.** Papini와 Haböck이 룩업을 GKR로 증명하는 LogUp-GKR을 발표합니다.
- **2025년 1월.** Khovratovich, Rothblum, Soukhanov가 *How to Prove False Statements*에서, GKR 회로가 자기 Fiat–Shamir 해시를 계산할
  수 있으면 비대화형 버전이 거짓 문장을 받아들이게 만들 수 있음을 보입니다. 배포된 시스템으로 이름이 나온 Polyhedra의 Expander는
  먼저 Fiat–Shamir를 고칩니다.
- **2026년 3월.** OtterSec이 GKR·sumcheck 기반 증명자에서 주장한 값이 묶이지 않은 문제를 보고합니다. Expander는 1월에, Scroll의
  Ceno는 3월에 고쳤습니다.

## 장점

- **선형 시간 증명.** 중간 층 전부가 아니라 증인에만 약속이 필요합니다.
- **얕고 규칙적인 회로에서 검증자가 작습니다.**
- **기본 형태에는 신뢰 설정이 없습니다.**

## 단점

- **그 자체로는 영지식이 아닙니다.** 가리려면 약속과 가림을 더해야 합니다.
- **깊이가 중요합니다.** 깊거나 불규칙한 회로는 검증자와 증명을 키웁니다.
- **Fiat–Shamir가 까다롭습니다.** 2025년 결과가 흔한 해시 기반 변환이 어디서 무너지는지 보여 줍니다.

## 얼마나 차세대인가

부품으로서 아주 그렇습니다. GKR은 S-two와 SP1의 룩업 논증, 그리고 여러 sumcheck 기반 zkVM 뒤에 있고, sumcheck 기반 증명은 지금
연구의 큰 흐름 중 하나입니다.

## 지금의 상태 (2026년 9월 기준)

**부품으로 쓰이는 중.** S-two가 LogUp에 GKR을 씁니다. GKR과 Hyrax로 된 Worldcoin의 Remainder는 프로덕션에 배포돼 있지 않습니다.

## 이 저장소에서 돌리는 방법

`crates/sys-gkr`는 Worldcoin의 Remainder를 커밋 하나에 고정해 BN254 위의 Hyrax 모드로 씁니다. 공개 입력은 입력 층 하나를
이루고, 검증자가 직접 채웁니다. 비밀은 두 번째 입력 층을 이루고 Pedersen 약속으로 커밋되므로 검증자에게 가지 않습니다.

Remainder의 게이트는 앞선 층의 값 두 개를 곱하고 그 곱들을 더할 뿐, 계수가 없습니다. 그래서 전시품은 문장마다 곱의 행으로
적고, 상수와 1을 공개 값으로 넣고, 아직 필요한 값은 층마다 한 단계씩 위로 복사합니다. 마지막 층에 모든 확인이 모이고,
Remainder는 그것이 전부 0이어야 한다고 요구합니다.

그래서 회로가 깊어집니다. one-plus-one은 52층, pool-spend는 549층에 게이트 약 11만 8천 개입니다. 증명은 깊이와 함께 커져서
factoring의 5층에서 16 KB, pool-spend에서 1.4 MB이며, pool-spend는 증명에 수십 초가 걸립니다. Remainder의 증명자는
아무것도 확인하지 않습니다. 거짓 주장도 증명이 되고, 검증자가 거절합니다.

`/run gkr one-plus-one`을 친 다음 `/run spartan one-plus-one`과 비교해 보세요. sumcheck로 증명을 짓는 두 방법입니다.

## 출처

- S. Goldwasser, Y. T. Kalai, G. N. Rothblum, *Delegating Computation: Interactive Proofs for Muggles*, STOC 2008 / JACM 2015 — https://doi.org/10.1145/2699436
- J. Thaler, *Time-Optimal Interactive Proofs for Circuit Evaluation*, CRYPTO 2013 — https://eprint.iacr.org/2013/351
- R. S. Wahby, I. Tzialla, a. shelat, J. Thaler, M. Walfish, *Doubly-efficient zkSNARKs without trusted setup* (Hyrax), IEEE S&P 2018 — https://eprint.iacr.org/2017/1132
- T. Xie, J. Zhang, Y. Zhang, C. Papamanthou, D. Song, *Libra*, CRYPTO 2019 — https://eprint.iacr.org/2019/317
- J. Zhang, T. Xie, Y. Zhang, D. Song, *Virgo*, IEEE S&P 2020 — https://eprint.iacr.org/2019/1482
- S. Papini, U. Haböck, *Improving logarithmic derivative lookups using GKR* — https://eprint.iacr.org/2023/1284
- D. Khovratovich, R. D. Rothblum, L. Soukhanov, *How to Prove False Statements* — https://eprint.iacr.org/2025/118
- Polyhedra Expander, Fiat–Shamir 수정 — https://github.com/PolyhedraZK/Expander/pull/184
- OtterSec, zkVM의 묶이지 않은 주장 보고(2026) — https://osec.io/blog/zkvms-unfaithful-claims/
- S-two — https://github.com/starkware-libs/stwo
- Worldcoin Remainder — https://github.com/worldcoin/Remainder_CE
