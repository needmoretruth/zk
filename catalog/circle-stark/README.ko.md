# Circle STARK

곱셈 부분군 대신 원 위에서 일해서, 컴퓨터가 가장 싸게 계산하는 필드 중 하나인 메르센 소수 2³¹ − 1 위에서 도는 STARK입니다.
2024년 Polygon과 StarkWare의 암호학자들이 설계했고, StarkWare의 S-two 증명자가 이것으로 Starknet을 증명합니다.

> **선반:** Polygon · **처음 발표:** 2024 · **신뢰 설정:** 없음 · **영지식:** 아니오 ·
> **양자 내성:** 그렇게 여겨짐(해시 기반) · **이 저장소에서 돌리는 코드:** Plonky3의 `p3-circle`

## 무엇인가

STARK에는 필드 안에 크기가 큰 2의 거듭제곱인 점들의 군이 필요합니다. 트레이스의 다항식이 거기 살고, 고속 푸리에 변환이
거기서 돕니다. 소수 `p = 2³¹ − 1`(Mersenne31)은 32비트 하드웨어에서 곱셈이 아주 싸지만, `p − 1`에는 2가 한 번밖에 들어
있지 않아 흔한 곱셈 군을 쓸 수 없습니다.

Circle STARK는 다른 군을 씁니다. 필드 위에서 `x² + y² = 1`을 만족하는 점 `(x, y)`들입니다. 이 원에는 점이 `p + 1`개
있고, `p + 1 = 2³¹`은 딱 떨어지는 2의 거듭제곱입니다. 논문은 STARK에 필요한 모든 것 — 원 FFT, 원 위의 Reed–Solomon
부호, FRI 저차수 시험 — 을 그 위에 다시 짓고, 결과가 기존 STARK만큼 효율적임을 보입니다. 31비트만으로는 안전성이
모자라서 챌린지는 확장체에서 뽑습니다.

다른 STARK처럼 핵심 프로토콜은 트레이스 값을 검증자에게 엽니다. 따로 가리지 않으면 영지식이 아닙니다.

## 역사

- **2023년 6월.** Haböck, Lubarov, Nabaglo가 원 군 위의 Reed–Solomon 부호를 발표합니다. 바탕이 되는 작업입니다.
- **2023년 7월.** StarkWare가 S-two(Stwo) 저장소를 만듭니다.
- **2024년 2월.** Ulrich Haböck(Polygon), David Levit, Shahar Papini(StarkWare)가 *Circle STARKs*를 공개합니다. ETHDenver에서
  StarkWare는 Polygon의 암호학자들과 함께 개발한 S-two를 오픈소스로 발표합니다.
- **2024년 3월.** Plonky3에 원 FFT와 약속 방식이 들어옵니다.
- **2025년 7월.** S-two 1.0.
- **2025년 11월 3일.** S-two가 Stone을 대신해 Starknet의 증명자가 됩니다.
- **2026년 6월.** S-two의 Cairo AIR 인코딩이 건전하다는 Lean 4 증명이 공개됩니다.

## 장점

- **연산이 아주 빠릅니다.** Mersenne31 원소는 32비트 SIMD 칸에 들어가고, `2³¹ − 1`로 나눈 나머지는 시프트와 덧셈 한 번입니다.
- **신뢰 설정이 없고** 안전성이 해시에 기댑니다.
- **프로덕션에서 돕니다.** S-two는 증명을 페어링 SNARK로 감싸지 않고 Starknet과 여러 애플리케이션을 증명합니다.

## 단점

- **기본값으로는 영지식이 아닙니다.** 비밀을 지키려면 별도의 블라인딩이 필요하고, StarkWare는 따로 된 파이프라인에서만 그것을
  더합니다.
- **추측에 기댄 안전성.** 다른 FRI 기반 시스템처럼 근접성 간격 추측과 작업 증명(grinding)에 일부 기댑니다.
- **특별한 필드가 필요합니다.** `p + 1`이 2로 많이 나누어떨어져야 하는데, 그런 소수는 드뭅니다.
- **증명이 큽니다.** 다른 STARK와 같습니다.

## 얼마나 차세대인가

맨 앞에 있습니다. 빠른 작은 필드 STARK의 가장 새로운 설계 중 하나이고, 큰 롤업에서 프로덕션으로 돌며, Cairo용 AIR에는
기계로 확인한 건전성 증명이 있습니다. 주된 대가 — 추측에 기댄 안전성과 기본값으로 없는 비밀 보호 — 는 STARK 계열 전체의
대가입니다.

## 지금의 상태 (2026년 9월 기준)

**쓰이는 중.** L2BEAT는 S-two 검증자가 Starknet, Paradex, Sorare, edgeX, tanX를 받친다고 적습니다. Plonky3도 이 전시품이
쓰는 원 약속 방식을 따로 관리합니다.

## 이 저장소에서 돌리는 방법

`crates/sys-circle-stark`는 Plonky3의 원 크레이트를 Mersenne31 위에서, [Plonky3](../plonky3/README.ko.md) 전시품과 같은 넓은 행
하나짜리 모양으로 씁니다. 와이어 하나에 열 하나, 게이트 하나에 규칙 하나입니다. S-two 자체는 nightly 컴파일러가 필요하고,
Plonky3의 원 약속은 이 저장소가 쓰는 안정판 컴파일러로 빌드됩니다.

트레이스는 회로의 행을 네 번 되풀이합니다. `CirclePcs`가 커밋하는 가장 적은 행 수입니다. 그러면 모든 열이 상수라서
검증자가 고른 무작위 점에서의 값이 곧 와이어 값이고, 증명은 그 값을 그대로 싣습니다. 박물관의 검사는 65,536 이상인 비밀을
증명 바이트에서 모두 찾아냅니다. one-plus-one의 솔트, password의 PIN, pool-spend의 지출 키와 머클 형제 노드입니다. 증명
크기는 one-plus-one의 약 29 KB부터 pool-spend의 약 420 KB까지입니다. 약속에 무작위 입력이 없어서 같은 주장은 늘 같은
증명을 냅니다.

`/run circle-stark one-plus-one`을 친 다음 `/run plonky3 one-plus-one`과 비교해 보세요. 두 필드 위의 같은 종류의 증명인데,
하나는 비밀을 감추고 하나는 감추지 않습니다.

## 출처

- U. Haböck, D. Levit, S. Papini, *Circle STARKs* — https://eprint.iacr.org/2024/278
- U. Haböck, D. Lubarov, J. Nabaglo, *Reed-Solomon Codes over the Circle Group* — https://eprint.iacr.org/2023/824
- S-two 저장소 — https://github.com/starkware-libs/stwo
- The Block, StarkWare의 Stwo 오픈소스 공개(2024) — https://www.theblock.co/post/279907/starkware-open-source-zero-knowledge-prover-stwo
- Starknet, *S-two is live on Starknet mainnet* (2025) — https://www.starknet.io/blog/s-two-is-live-on-starknet-mainnet-the-fastest-prover-for-a-more-private-future/
- S-two Cairo AIR의 Lean 4 건전성 증명(2026) — https://arxiv.org/abs/2606.04311
- 영지식 블라인딩이 들어간 StarkWare 증명 파이프라인 — https://github.com/starkware-libs/proving/blob/main/crates/circuit_common/src/finalize.rs
- L2BEAT ZK 카탈로그, Stwo — https://l2beat.com/zk-catalog/stwo
- Plonky3 원 FFT와 PCS — https://github.com/Plonky3/Plonky3/pull/278
