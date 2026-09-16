# Halo 2

신뢰 설정이 없는 증명 시스템입니다. PLONK 방식의 회로를 Pasta 곡선 위의 내적 논증(IPA) 약속으로 증명합니다. Zcash의 가장
새 풀에서 Groth16을 대신했고, 한 세대의 프로젝트가 받아들인 회로 언어 「PLONKish」를 만들었습니다.

> **선반:** Zcash · **처음 공개:** 2020 · **신뢰 설정:** 없음 · **영지식:** 예 ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** Zcash의 크레이트 `halo2_proofs`

## 무엇인가

Halo 2는 두 부분으로 되어 있습니다.

- **회로 언어(산술화, arithmetization).** 회로는 표입니다. 값의 열, 칸의 행, 그리고 켜진 행마다 성립해야 하는 다항식
  「게이트」가 있습니다. 복사 제약으로 칸끼리 묶을 수 있고, lookup 논증으로 어떤 값이 표에 있어야 한다고 요구할 수 있습니다.
  PLONK에서 물려받아 넓힌 이 방식을 지금은 *PLONKish*라고 부릅니다.
- **약속(commitment) 방식.** 표의 열은 다항식이 되고, 증명자는 Bulletproofs 방식의 *내적 논증*(IPA)으로 그 다항식에 약속합니다.
  IPA는 이산 로그가 어려운 그룹만 있으면 됩니다. 페어링도 비밀 설정도 필요 없어서, 공개 파라미터를 누구나 아무것도 없이
  만들 수 있습니다.

곡선은 Pallas와 Vesta(「Pasta」)입니다. 한 곡선의 그룹 위수가 다른 곡선의 체 크기와 같습니다. 이 순환 덕분에 한 증명이 다른
증명을 효율적으로 확인할 수 있고, 이것이 2019년 Halo가 내놓은 재귀 아이디어입니다.

## 역사

- **2019년 9월.** Sean Bowe, Jack Grigg, Daira Hopwood가 신뢰 설정 없이 증명을 재귀로 합성하는 첫 실용적 방법인 *Halo*를
  발표합니다. 연구용 코드였고 실제로 쓰이지는 않았습니다.
- **2020년 9월 1일.** Electric Coin Company가 PLONK 방식의 산술화와 Halo의 약속·누적 아이디어를 합친 새 구현 Halo 2를
  발표합니다. 처음 라이선스는 Bootstrap Open Source License였습니다.
- **2022년 4월 7일.** Filecoin Foundation과의 합의로 Halo 2가 MIT 또는 Apache-2.0으로 바뀝니다. 이더리움 재단의 zkEVM 작업이
  이미 그 일부를 쓰고 있었고, 뒤에 나온 포크들은 약속 방식을 KZG로 바꿔 범용 설정을 대가로 더 작은 증명을 얻었습니다.
- **2022년 5월 31일.** Zcash NU5 업그레이드가 블록 1,687,104에서 Orchard 풀을 켭니다. Zcash의 shielded 거래가 처음으로
  신뢰 설정 없이 돌아갑니다.
- **2026년 5월 29일.** 감사에서 Orchard가 쓰는 회로 가젯(`halo2_gadgets`의 가변 기저 스칼라 곱셈)에 빠진 복사 제약이 발견됩니다.
  2022년부터 같은 노트를 다른 nullifier로 두 번 쓸 수 있었습니다. Zcash는 6월 2일 긴급 소프트 포크로 Orchard를 끄고, 6월 3일
  고친 회로로 다시 켭니다(NU6.2). 악용 흔적은 발견되지 않았습니다. 결함은 Halo 2 증명 시스템이 아니라 Halo 2 *로 만든* 회로에 있었습니다.
- **2026년 7월 28일.** NU6.3이 Orchard의 회로와 Halo 2 증명 시스템을 그대로 쓰는 Ironwood 풀을 열고 Orchard를 봉인합니다.

## 장점

- **신뢰 설정이 없음.** 비밀을 만들지 않으니 지울 것도 믿을 것도 없습니다.
- **표현력 있는 회로.** 사용자 정의 게이트와 lookup으로 해시·범위 검사·타원곡선 연산을 R1CS보다 훨씬 싸게 만들 수 있습니다.
- **재귀를 위해 설계됨.** Pasta 순환과 누적 덕분에 페어링 곡선 없이도 증명이 증명을 확인합니다.
- **실전 검증.** 2022년부터 Zcash의 shielded 풀을 Rust로, 허용형 라이선스로 지켜 왔습니다.

## 단점

- **검증 시간이 일정하지 않음.** IPA 열기를 확인하는 데 회로 크기에 비례하는 시간이 들고, 그 일을 누적해 미루지 않으면 Halo 2
  증명 하나는 Groth16 증명보다 검증이 느립니다.
- **더 큰 증명.** Groth16의 192바이트가 아니라 킬로바이트 단위입니다.
- **양자 내성이 없음.** 보안이 이산 로그에 기댑니다.
- **회로를 미묘하게 틀리기 쉬움.** 회로를 싸게 만드는 표현력이 빠진 제약도 놓치기 쉽게 만듭니다. 2026년의 Orchard 결함은 4년
  동안 아무도 몰랐습니다.

## 얼마나 차세대인가

2020년에는 이것이 차세대였습니다. 설정이 없고, 재귀가 되고, 이더리움 zkEVM 작업·Scroll·Axiom 등이 PLONKish 표 위에 지을 만큼
유연한 회로 언어였습니다. 그 뒤 최전선은 다시 옮겨 갔습니다 — 작은 체 위의 STARK와 zkVM, 폴딩, HyperPlonk와 Aztec의 Honk 같은
sumcheck 기반 변형, 그리고 양자 내성 구성으로. Zcash 자신의 다음 단계인 Project Tachyon은 Halo 2가 아니라, 원래 Halo 구성에서
갈라져 나온 새 재귀 프레임워크 Ragu 위에 짓고 있습니다.

## 지금의 상태 (2026년 9월 기준)

**쓰이는 중.** Halo 2는 2026년 8월 말 약 384만 ZEC가 있던 Zcash Ironwood 풀의 모든 거래와 Orchard의 남은 인출을 증명합니다.
Zcash의 크레이트 `halo2_proofs` 0.3.5는 2026년 8월에 나왔습니다. privacy-scaling-explorations의 KZG 포크는 보관(archive)됐고,
Axiom 포크는 관리되고 있습니다.

## 이 저장소에서 돌리는 방법

`crates/sys-halo2`는 zcash/halo2 저장소의 `halo2_proofs`와 그 Pasta 곡선을 씁니다. 모든 예제를 PLONKish 행으로 내리고, advice 열
셋과 범용 게이트 하나 `q_l·a + q_r·b + q_o·c + q_m·a·b + q_c − instance = 0`, 공유 전선을 위한 복사 제약, 공개 입력을 싣는 instance 열로
배치합니다. 실제 Orchard 회로는 많은 사용자 정의 게이트와 lookup을 씁니다. 범용 게이트 하나로 두면 여기의 모든 시스템이 증명하는
회로와 똑같아지는 대신, 손으로 다듬은 Halo 2 회로보다 행이 많아집니다.

`/run halo2 one-plus-one`을 쳐 보고, 증명 크기와 검증 시간을 `/run groth16 one-plus-one`과 비교해 보세요.

이 저장소는 Zcash, Electric Coin Company, Zcash Foundation과 관계가 없습니다.

## 출처

- S. Bowe, J. Grigg, D. Hopwood, *Recursive Proof Composition without a Trusted Setup* (Halo) — https://eprint.iacr.org/2019/1021
- Announcing Halo 2, Zcash 커뮤니티 포럼, 2020-09-01 — https://forum.zcashcommunity.com/t/announcing-halo-2/37215
- Halo의 MIT/Apache 2.0 라이선스 전환, Electric Coin Company, 2022-04-07 — https://electriccoin.co/blog/zero-knowledge-proving-system-halo-now-licensed-under-mit-making-it-available-for-anyone-to-use/
- NU5 메인넷 활성화, Electric Coin Company — https://electriccoin.co/blog/nu5-activates-on-mainnet-eliminating-trusted-setup-and-launching-a-new-era-for-zcash/
- ZIP 224, Orchard Shielded Protocol — https://zips.z.cash/zip-0224
- ZIP 257, 긴급 소프트 포크와 NU6.2 활성화 — https://zips.z.cash/zip-0257 · https://zfnd.org/zebra-4-5-3-and-5-0-0-emergency-soft-fork-and-nu6-2-activation/
- ZIP 258과 Zebra 6.0.0(Ironwood) — https://zips.z.cash/zip-0258 · https://zfnd.org/zebra-6-0-0-release/
- Pine Analytics, Zcash 분기 보고서 2026년 3분기 — https://pineanalytics.substack.com/p/zcash-quarterly-report-q3-2026
- halo2 책 — https://zcash.github.io/halo2/
