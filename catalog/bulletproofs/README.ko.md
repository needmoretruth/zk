# Bulletproofs

신뢰 설정 없이, 이산 로그의 어려움 하나에만 기대서 만든 짧은 증명입니다. 암호화폐 거래의 금액을 감추려고 만들어졌고,
2018년에 가동됐을 때 Monero의 거래 크기를 80% 넘게 줄였습니다.

> **선반:** Others · **처음 발표:** 2017 · **신뢰 설정:** 없음 · **영지식:** 예 ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** `bulletproofs`, dalek 구현을 zkcrypto가 포크한 것

## 무엇인가

Pedersen 약속 `v·G + r·H`는 수 `v`를 무작위 `r` 뒤에 감추고, 생성원을 여러 개 쓰면 수 여러 개를 함께 약속할 수 있습니다.
Bulletproofs는 약속된 벡터에 대한 사실을 **내적 논증**으로 증명합니다. 약속된 두 벡터의 내적이 주어진 값임을 보이려고,
증명자는 무작위 챌린지로 두 벡터를 반으로 접고, 군 원소 두 개를 보내고, 이를 되풀이합니다. `log n` 라운드가 지나면 수
하나가 남으므로 증명에는 원소가 `2·log n` 개 정도뿐입니다.

가장 잘 알려진 쓰임은 **범위 증명**입니다. 「이 약속된 금액은 0과 2^64 사이다」를 「64개 비트가 각각 0 또는 1이고, 그
합이 금액이다」로 적고 내적 논증 하나로 증명합니다. 64비트 범위 증명은 약 670바이트이고, 여러 금액의 증명을 원소 몇
개만 더해 하나로 모을 수 있습니다. 같은 장치로 곱셈 게이트와 선형 제약으로 주어진 어떤 산술 회로든 증명합니다.

대가는 검증자 쪽에 있습니다. 접힌 생성원을 다시 계산해야 해서 문장 크기에 비례하는 시간이 듭니다. 증명 여러 개를 함께
검증하면 그 비용을 나눌 수 있습니다.

## 역사

- **2016년.** Bootle, Cerulli, Chaidos, Groth, Petit가 통신량이 로그인 산술 회로용 이산 로그 논증을 내놓습니다.
  Bulletproofs가 그 위에 지어집니다.
- **2017년 11월.** Bünz, Bootle, Boneh, Poelstra, Wuille, Maxwell이 Bulletproofs를 공개하고, IEEE S&P 2018에 실립니다.
- **2018년 10월 18일.** Monero가 거래 금액에 Bulletproofs를 도입해 거래 크기를 80% 넘게 줄입니다.
- **2020년 6월.** Bulletproofs+가 영지식 가중 내적 논증을 더해 64비트 범위 증명을 576바이트로 줄입니다.
- **2022년 4월 15일.** Trail of Bits의 「Frozen Heart」 공개. 논문이 제안한 챌린지가 증명 대상인 약속을 빠뜨려서, 그것을
  따른 구현 여럿에서 증명을 위조할 수 있었습니다.
- **2022년 8월 13일.** Monero가 Bulletproofs+로 옮깁니다.

## 장점

- **신뢰 설정이 없습니다.** 생성원은 해시로 만들고, 비밀을 가진 사람이 없습니다.
- **짧습니다.** 문장 크기의 로그에 비례하고, 64비트 범위 증명은 1킬로바이트가 안 됩니다.
- **묶기와 일괄 검증.** 범위 증명 여러 개를 하나로, 증명 여러 개를 한꺼번에 확인합니다.
- **이산 로그 가정 하나뿐**이고, 연산이 빠르고 잘 감사된 곡선 위에서 돕니다.

## 단점

- **검증 시간이 선형입니다.** 증명 확인에 문장 크기에 비례하는 시간이 듭니다. Bulletproofs식 약속 위에서 재귀를 하려면
  증명 안에서 증명을 검증하는 대신 누적(accumulation)이 필요한 이유이기도 합니다.
- **큰 회로에는 느립니다.** 일반 회로의 증명과 검증이 페어링 SNARK나 STARK보다 훨씬 느립니다.
- **Fiat–Shamir 함정.** 논문 자신이 고른 챌린지 때문에 구현들이 위조 가능해졌습니다.
- **양자 내성이 없습니다.** 이산 로그는 Shor 알고리즘에 풀립니다.

## 얼마나 차세대인가

새롭다기보다 성숙했고, 신뢰 설정 없는 범위 증명의 기준점으로 남아 있습니다. 내적 논증은 계속 살아 있습니다. Zcash의
Halo 2 안에 있는 IPA 다항식 약속이 같은 아이디어로 만들어졌습니다.

## 지금의 상태 (2026년 9월 기준)

**주 사용처에서는 자기 후속 방식으로 대체됨.** Monero는 2022년 8월 Bulletproofs를 Bulletproofs+로 바꿨고 0.18.5.x
릴리스에서도 그것을 씁니다. 다음으로 제안된 업그레이드 FCMP++는 아직 활성화되지 않았습니다. Tari가 Rust용 Bulletproofs+
라이브러리를 관리합니다.

## 이 저장소에서 돌리는 방법

`crates/sys-bulletproofs`는 `bulletproofs` 크레이트 — dalek-cryptography의 구현을 zkcrypto가 포크해 crates.io에 올린 것 —
를 커밋 하나에 고정해 ristretto255 위의 R1CS 증명자와 함께 씁니다. crates.io에 올라간 크레이트는 R1CS 증명자를 꺼 두어서, 이
전시품은 저장소에서 직접 빌드합니다. 박물관 회로의 제약마다 곱셈 게이트 하나와 선형 제약 하나가 되고, 비밀 값은 증명 안에
할당되며, 공개 입력은 검증자가 제약을 만들 때 쓰는 상수로 들어갑니다.

여기서 증명은 `one-plus-one`이 801바이트, `pool-spend`가 1,057바이트입니다. 곱셈 게이트 수가 두 배가 될 때마다 64바이트씩
늘어납니다. 이 전시품이 업스트림 크레이트 바깥에 더한 것이 둘 있습니다.
- 챌린지를 뽑기 전에 공개 입력을 트랜스크립트에 해시합니다. 업스트림은 약속만 해시하는데, 해시에서 빠진 상수가 바로
  Frozen Heart의 실수입니다.
- 업스트림은 같은 증명을 더 긴 배치로 다시 쓴 것도 받아들입니다. 박물관은 증명자가 쓴 바이트만 받습니다.

`/run bulletproofs membership`을 친 다음 `/run halo2 membership`과 비교해 보세요. 둘 다 신뢰 설정이 없고, 하나는 다른 하나의
후손입니다.

## 출처

- B. Bünz, J. Bootle, D. Boneh, A. Poelstra, P. Wuille, G. Maxwell, *Bulletproofs: Short Proofs for Confidential Transactions and More*, IEEE S&P 2018 — https://eprint.iacr.org/2017/1066
- J. Bootle, A. Cerulli, P. Chaidos, J. Groth, C. Petit, *Efficient Zero-Knowledge Arguments for Arithmetic Circuits in the Discrete Log Setting*, EUROCRYPT 2016 — https://eprint.iacr.org/2016/263
- Monero 0.13.0 릴리스 — https://www.getmonero.org/2018/10/11/monero-0.13.0-released.html
- H. Chung, K. Han, C. Ju, M. Kim, J. H. Seo, *Bulletproofs+* — https://eprint.iacr.org/2020/735
- Trail of Bits, *The Frozen Heart vulnerability in Bulletproofs* — https://blog.trailofbits.com/2022/04/15/the-frozen-heart-vulnerability-in-bulletproofs/
- Monero 네트워크 업그레이드, 2022년 7월 — https://web.getmonero.org/2022/04/20/network-upgrade-july-2022.html
- Monero, unlock time 폐기 공지(FCMP++ 아직 활성화 전) — https://www.getmonero.org/2026/05/10/deprecating-unlock-time.html
- Ragu 문서, Bulletproofs와 누적 — https://github.com/tachyon-zcash/ragu/blob/main/book/src/protocol/prelim/bulletproofs.md
- bulletproofs (zkcrypto 포크) — https://github.com/zkcrypto/bulletproofs · 원본 — https://github.com/dalek-cryptography/bulletproofs
