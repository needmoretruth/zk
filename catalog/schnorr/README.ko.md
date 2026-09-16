# Schnorr와 시그마 프로토콜

박물관에서 가장 오래됐으면서 지금도 실제 거래에 서명하는 증명 시스템입니다. Schnorr 증명은 공개 키 뒤의 비밀을 안다는
것을 메시지 세 개로 보이고, 이것을 해시로 하나로 합치면 Zcash의 shielded 송금과 Bitcoin의 Taproot 송금을 승인하는 종류의
서명이 됩니다.

> **선반:** Zcash · **처음 발표:** 1989 · **신뢰 설정:** 없음 · **영지식:** 예(정직한 검증자 · 비대화형이면 통계적) ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** `sigma-proofs`와 이 저장소가 직접 쓴 회로 컴파일러

## 무엇인가

이산 로그가 어려운 군과 생성원 `G`를 정합니다. 여러분은 `X = x·G`인 `x`를 안다고 주장합니다.

1. **커밋.** 무작위 `r`을 골라 `R = r·G`를 보냅니다.
2. **챌린지.** 검증자가 무작위 수 `c`를 보냅니다.
3. **응답.** `s = r + c·x`를 보내고, 검증자는 `s·G = R + c·X`인지 확인합니다.

같은 `R`에 대해 서로 다른 챌린지 두 개에 답하면 `x`가 드러나므로, `x`를 모르는 사람은 많아야 하나에만 답할 수 있습니다.
속이는 사람이 버틸 확률은 챌린지 공간 크기분의 1입니다. 그리고 대화 `(R, c, s)`는 `x` 없이도 만들 수 있습니다 — `c`와
`s`를 먼저 고르고 `R = s·G − c·X`로 두면 됩니다 — 그래서 검증자가 새로 배우는 것은 없습니다.

커밋·챌린지·응답의 세 단계로 된 이런 프로토콜을 **시그마 프로토콜**이라고 부릅니다. 서로 조합할 수 있습니다. 두 문장의
AND, 두 문장의 OR, 그리고 「이 군 원소들은 비밀들의 이 선형 결합이다」 꼴의 어떤 문장이든 됩니다. 검증자의 챌린지를
문장과 커밋의 해시로 바꾸면(**Fiat–Shamir** 변환) 대화가 나중에 누구나 확인할 수 있는 증명이 되고, 메시지까지 해시에
넣으면 서명이 됩니다.

시그마 프로토콜은 간결하지 않습니다. 큰 계산에 대한 증명은 계산만큼 큽니다. Zcash가 송금에 서명할 때는 시그마 프로토콜을,
송금 자체를 증명할 때는 SNARK를 쓰는 이유입니다.

## 역사

- **1986년.** Fiat와 Shamir가 해시 함수로 대화형 신원 확인 방식을 서명으로 바꾸는 방법을 보입니다.
- **1989년.** Claus-Peter Schnorr가 CRYPTO '89에서 이산 로그에 기댄, 스마트카드를 위한 효율적인 신원 확인과 서명을
  발표합니다. 학술지 버전은 1991년에 나옵니다. 이 방식에 대한 그의 미국 특허는 2008년까지 유효했습니다.
- **1994년.** Cramer, Damgård, Schoenmakers가 이런 증명을 조합해 「이 문장들 중 하나가 참이다」를 증명하는 방법을 보이고,
  세 단계 꼴의 프로토콜이 하나의 계열 — 시그마 프로토콜 — 로 연구되기 시작합니다.
- **2012년.** 「How not to Prove Yourself」가, 문장 전체를 해시하지 않은 부주의한 Fiat–Shamir 때문에 Helios 투표 시스템이
  깨졌음을 보입니다.
- **2018년 10월 28일.** Zcash의 Sapling 업그레이드가 키를 다시 무작위화할 수 있는 Schnorr 기반 서명 RedJubjub를
  도입합니다. 송금 승인과, 거래마다 금액이 맞음을 묶는 binding 서명에 씁니다.
- **2021년 11월.** Bitcoin의 Taproot 업그레이드로 BIP340 Schnorr 서명이 활성화됩니다.
- **2022년 4월.** Trail of Bits가 「Frozen Heart」를 공개합니다. 여러 증명 시스템 구현의 약한 Fiat–Shamir 문제입니다.
- **2022년 5월 31일.** Zcash의 Orchard 풀이 같은 방식을 Pallas 곡선에 올린 RedPallas와 함께 출범합니다.
- **2024년 6월.** RFC 9591이 두 라운드 임계값 Schnorr 서명 FROST를 표준으로 정합니다.
- **2026년.** IRTF의 암호 연구 그룹이 선형 관계에 대한 시그마 증명과 Fiat–Shamir 변환의 초안을 다듬고, Zcash Foundation이
  FROST 3.0.0을 냅니다.

## 장점

- **작고 단순합니다.** 서명은 64바이트이고, 검증은 스칼라 곱셈 두 번이며, 서명 여러 개를 한꺼번에 확인할 수 있습니다.
- **신뢰 설정이 없고**, 자기 키 말고는 지킬 비밀이 없습니다.
- **조합할 수 있습니다.** 비밀이 몇 개든 AND·OR·선형 관계를 짧고 잘 이해된 증명으로 증명합니다.
- **키를 다시 무작위화할 수 있습니다.** Zcash는 송금마다 새것처럼 보이는 키로 승인하므로, 같은 주인의 두 송금을 서명으로
  연결할 수 없습니다.

## 단점

- **간결하지 않습니다.** 일반 계산을 이 방식으로 증명하면 작업량과 바이트가 계산 크기에 비례합니다.
- **대화형일 때는 정직한 검증자에 대해서만 영지식입니다.** 비대화형 버전은 랜덤 오라클 모델에서 영지식입니다.
- **Fiat–Shamir는 전부 해시해야 합니다.** 문장을 해시에서 빼서 실제 시스템이 깨진 적이 있습니다.
- **여럿이 쓰면 깨지기 쉽습니다.** 블라인드 서명과 다자간 변형은 아주 조심해야 하고, 2020년의 ROS 공격이 그중 여럿을
  깨뜨렸습니다.
- **양자 내성이 없습니다.** Shor 알고리즘을 돌리는 양자 컴퓨터는 이산 로그를 계산합니다.

## 얼마나 차세대인가

37년 된 아이디어이고 사라지지 않을 것입니다. 임계값 서명(FROST), 여러 익명 자격 증명 방식, 그리고 더 큰 증명 시스템
안의 많은 부품이 시그마 프로토콜입니다. 계산 전체를 증명하는 일에서는 오래전에 SNARK와 STARK에 자리를 내줬습니다.
이산 로그에 기댄 모든 것처럼, 앞날은 양자 컴퓨터가 정합니다.

## 지금의 상태 (2026년 9월 기준)

**쓰이는 중.** Zcash의 Sapling·Orchard 송금은 RedDSA 서명으로, Bitcoin의 Taproot 송금은 BIP340 서명으로 승인됩니다.
IRTF 문서는 아직 초안입니다.

## 이 저장소에서 돌리는 방법

`crates/sys-schnorr`는 두 층입니다.

- **Schnorr 지식 증명.** RedPallas 서명의 핵심이고, Pallas 곡선 위에서 `sigma-proofs`로 만듭니다.
- **일곱 문장.** Schnorr 증명 하나로는 표현할 수 없습니다. 이 저장소가 쓴 작은 컴파일러 — 교육용 구현이고 감사받지
  않았습니다 — 가 회로를 커다란 시그마 프로토콜 하나로 바꾸고, `sigma-proofs`가 그것을 증명합니다. 비밀 와이어마다
  Pedersen 약속 `v·G + r·H`가 붙는데, `H`와 `G`의 관계는 아무도 모릅니다. 약속끼리 더해지므로 덧셈에는 증명이 필요
  없습니다. 곱셈마다 약속된 값 하나가 다른 둘의 곱이라는 작은 증명이, 단언마다 약속이 0을 감춘다는 증명이 붙습니다.

간결하지 않은 값이 바로 보입니다. Schnorr 증명 하나는 64바이트이지만, 문장의 증명은 `one-plus-one`의 6,336바이트부터
`pool-spend`의 126,560바이트까지입니다. 약속·식·비밀 하나마다 32바이트씩입니다. `pool-spend`는 증명과 검증에 각각 몇 초가
걸립니다.

`/run schnorr one-plus-one`을 친 다음 `/run groth16 one-plus-one`과 비교해 보세요. 간결함이 무엇을 사 주는지 보입니다.

이 저장소는 Zcash, Electric Coin Company, Zcash Foundation과 관계가 없습니다.

## 출처

- A. Fiat, A. Shamir, *How To Prove Yourself*, CRYPTO '86 — https://doi.org/10.1007/3-540-47721-7_12
- C. P. Schnorr, *Efficient Identification and Signatures for Smart Cards*, CRYPTO '89 — https://doi.org/10.1007/0-387-34805-0_22
- C. P. Schnorr, *Efficient signature generation by smart cards*, Journal of Cryptology 1991 — https://doi.org/10.1007/BF00196725
- 미국 특허 4,995,082 — https://cr.yp.to/patents/us/4995082.html
- R. Cramer, I. Damgård, B. Schoenmakers, *Proofs of Partial Knowledge and Simplified Design of Witness Hiding Protocols*, CRYPTO '94 — https://doi.org/10.1007/3-540-48658-5_19
- D. Bernhard, O. Pereira, B. Warinschi, *How not to Prove Yourself*, ASIACRYPT 2012 — https://eprint.iacr.org/2016/771
- Zcash 프로토콜 명세 §5.4.7 (RedDSA) — https://zips.z.cash/protocol/protocol.pdf
- BIP340, Schnorr Signatures for secp256k1 — https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki
- Trail of Bits, *Frozen Heart* 공개 — https://blog.trailofbits.com/2022/04/13/part-1-coordinated-disclosure-of-vulnerabilities-affecting-girault-bulletproofs-and-plonk/
- F. Benhamouda 외, *On the (in)security of ROS* — https://eprint.iacr.org/2020/945
- RFC 9591, FROST — https://datatracker.ietf.org/doc/rfc9591/
- IRTF CFRG, *Interactive Sigma Proofs* (초안) — https://datatracker.ietf.org/doc/draft-irtf-cfrg-sigma-protocols/
- Zcash Foundation FROST 릴리스 — https://github.com/ZcashFoundation/frost/releases
- sigma-proofs — https://github.com/sigma-rs/sigma-proofs
