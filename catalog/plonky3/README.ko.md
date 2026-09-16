# Plonky3

STARK 증명자를 조립하는 Polygon의 오픈소스 도구 모음입니다. 필드·해시·다항식 약속·증명자를 골라 필요한 증명 시스템을
만듭니다. 지금의 zkVM 여럿 — SP1, OpenVM, Miden 등 — 이 이 위에 지어졌습니다.

> **선반:** Polygon · **첫 릴리스:** 2024 · **신뢰 설정:** 없음 · **영지식:** 가리는 약속을 골랐을 때만 ·
> **양자 내성:** 그렇게 여겨짐(해시 기반) · **이 저장소에서 돌리는 코드:** Plonky3 자체 크레이트 `p3-uni-stark`

## 무엇인가

STARK는 계산의 단계들을 표, 곧 **실행 트레이스**에 적고, 모든 행(그리고 이웃한 두 행)이 지켜야 하는 다항식 규칙,
곧 **AIR**를 정해서 계산이 올바르게 이루어졌음을 증명합니다. 증명자는 열마다 작고 빠른 필드 위의 다항식으로 바꾸고
해시 기반 약속으로 커밋합니다. 검증자는 표 바깥의 무작위 점에서 규칙을 확인하고, 커밋된 데이터가 정말 낮은 차수의
다항식인지 시험합니다.

Plonky3는 그런 시스템 하나를 정해 두지 않고 부품을 줍니다. 필드(BabyBear, KoalaBear, Mersenne31, Goldilocks, 이진 필드),
해시(Poseidon2, Keccak, BLAKE3 등), 약속 방식(FRI, STIR, WHIR, Mersenne31용 원 약속), 증명자(표 하나는 `uni-stark`, 여럿은
`batch-stark`와 `multi-stark`)를 고릅니다. 작은 필드와 SIMD 명령어 덕분에 빠릅니다.

Plonky3는 기본값으로는 영지식이 아닙니다. 표준 FRI 약속은 트레이스 값을 그대로 엽니다. 증명이 계산이 옳았음을 보이면서
계산의 일부도 함께 보여 주는 것입니다. 비밀을 지키려면 가리는 약속을 골라야 하고, 그 약속은 커밋하는 내용에 무작위
행과 솔트를 섞습니다.

## 역사

- **2023년.** Plonky2를 만든 Polygon Zero 팀이 MIT와 Apache-2.0 이중 라이선스로 Plonky3를 공개 개발하기 시작합니다.
- **2024년 3월.** Mersenne31 필드용 원 FFT와 약속이 들어옵니다. Circle STARK의 바탕입니다.
- **2024년 7월 4일.** crates.io에 첫 릴리스.
- **2024년 7월 16일.** Polygon이 Plonky3를 프로덕션 준비 완료로 발표하고, 사용처로 Valida와 SP1을 듭니다. 7월 31일
  Least Authority의 감사 보고서가 나옵니다.
- **2024년 11월 18일.** 가리는 FRI 약속이 추가되어 영지식 증명이 가능해집니다.
- **2025년.** FRI 검증자와 트랜스크립트에서 심각도 높은 권고 셋이 고쳐집니다. 열린 값이 트랜스크립트에 묶이지 않았고,
  크기 검사가 빠졌고, 마지막 차수 검사가 빠져서 악의적인 증명자가 속일 수 있었습니다.
- **2026년 2월.** Miden VM이 Winterfell을 Plonky3로 바꿉니다.
- **2026년 3월.** WHIR가 통합됩니다.
- **2026년 9월.** 버전 0.7.0.

## 장점

- **빠릅니다.** 31비트짜리 작은 필드가 기계 워드에 들어가고, 연산에 AVX2·AVX-512·NEON을 씁니다.
- **신뢰 설정이 없고** 안전성이 해시에 기대서, 양자 컴퓨터에도 버틴다고 여겨집니다.
- **모듈식입니다.** 나머지를 다시 쓰지 않고 필드·해시·약속을 바꿀 수 있고, 같은 코드가 아주 다른 증명자들을 받칩니다.
- **안정판 Rust, 감사를 받았고, 널리 쓰입니다.** 많은 zkVM이 기대고 있어서 버그가 발견됩니다.

## 단점

- **요청하지 않으면 비밀을 지키지 않습니다.** 기본 약속은 트레이스를 흘립니다. 영지식에는 가리는 변형이 필요하고,
  그것도 완전한 영지식이 아니라 통계적 영지식입니다.
- **증명이 큽니다.** 수백 킬로바이트가 흔합니다. 블록체인에 증명을 올리는 프로젝트는 보통 마지막 STARK 증명을 페어링
  기반 SNARK로 감싸는데, 그 SNARK는 양자 내성이 없습니다.
- **추측에 기댄 안전성.** FRI 계열 약속의 안전성 수치는 근접성 간격(proximity gap) 추측에 기대고, 그중 가장 강한 추측
  몇 개가 2025년에 반증됐습니다. 주장하는 보안 비트 수는 조심해서 읽어야 합니다.
- **검증자 버그가 있었습니다.** 2025–2026년에 심각도 높은 권고 넷이 나왔고, 그중 셋이 건전성 버그였습니다.

## 얼마나 차세대인가

맨 앞에 있습니다. 작은 필드와 해시에 기댄 증명은 2026년에 잘 알려진 zkVM들 — RISC Zero, SP1, OpenVM, Miden — 의
방식이고, Plonky3는 그중 여럿이 함께 쓰는 도구 모음입니다. STIR·WHIR·다중선형·이진 필드 약속 같은 새 부품이 연구를
따라 들어옵니다.

## 지금의 상태 (2026년 9월 기준)

**쓰이는 중.** 공개 개발 중이고, 2026년 9월에 버전 0.7.0이 나왔습니다. OpenVM, SP1, Miden VM이 이 위에 지어졌고,
L2BEAT는 Scroll을 증명하는 OpenVM을 「Plonky3 기반」으로 적습니다.

## 이 저장소에서 돌리는 방법

`crates/sys-plonky3`는 BabyBear 위에서 `p3-uni-stark` 0.7.0을 **가리는 FRI 약속**과 함께 씁니다. Plonky3의 영지식 시험이
쓰는 설정이고, 무작위 시드는 전부 운영체제에서 받습니다.

박물관의 문장은 긴 반복 계산이 아니라 한 번짜리 회로입니다. 그래서 각 문장이 아주 넓은 행 하나짜리 AIR가 됩니다. 와이어
하나에 열 하나, 게이트 하나에 2차 규칙 하나입니다. STARK의 증명 크기는 열 수에 따라 커지므로 STARK에게 가장 불리한
모양입니다. 여기서 증명은 `one-plus-one`이 약 55 KB, `pool-spend`가 약 450 KB입니다. STARK가 빛나는 것은 같은 단계가 수백만
행 이어지는 표입니다.

보게 될 것 둘:
- 디버그 검사를 켜고 빌드한 프로그램에서는 Plonky3 증명자가 트레이스를 검사하므로, 거짓 주장은 증명이 만들어지기 전에
  증명자가 거부합니다. 릴리스 빌드에서는 증명이 만들어지고 검증자가 거절합니다.
- 뒤집힌 바이트는 열린 트레이스 값 안에 떨어집니다. 그 값은 약속의 작업 증명 검사 전에 트랜스크립트에 해시되므로,
  대수 검사에 닿기 전에 그 검사에서 실패합니다.

`/run plonky3 pool-spend`를 친 다음, 크기의 반대쪽 끝인 `/run groth16 pool-spend`와 비교해 보세요.

## 출처

- Plonky3 저장소와 구조 문서 — https://github.com/Plonky3/Plonky3 · https://github.com/Plonky3/Plonky3/blob/main/docs/architecture.md
- Polygon, *Plonky3, the next generation of ZK proving systems, is production ready* — https://polygon.technology/blog/polygon-plonky3-the-next-generation-of-zk-proving-systems-is-production-ready
- 원 FFT와 PCS — https://github.com/Plonky3/Plonky3/pull/278
- 가리는 FRI 약속 — https://github.com/Plonky3/Plonky3/pull/536
- 감사 — https://github.com/Plonky3/Plonky3/tree/main/audits
- 보안 권고 — https://github.com/Plonky3/Plonky3/security/advisories
- 릴리스 v0.7.0 — https://github.com/Plonky3/Plonky3/releases/tag/v0.7.0
- Miden VM 변경 기록 — https://github.com/0xMiden/miden-vm/blob/next/CHANGELOG.md
- WHIR 통합 — https://github.com/Plonky3/Plonky3/pull/1477
- D. Crites, A. Stewart, Reed–Solomon 근접성 간격 추측 반증 — https://eprint.iacr.org/2025/2046
- L2BEAT ZK 카탈로그, OpenVM — https://l2beat.com/zk-catalog/openvmprover
