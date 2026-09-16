# STARK

해시에 기댄 계산 무결성 증명의 원조입니다. 신뢰 설정이 없고, 안전성이 해시 함수에서 오며, 검증이 계산보다 지수적으로 빠릅니다.
이것 위에 지은 StarkWare의 Stone 증명자가 2020년부터 StarkEx를, 2025년 11월 Circle STARK로 바뀔 때까지 Starknet을 증명했습니다.

> **선반:** Others · **처음 발표:** 2018 · **신뢰 설정:** 없음 · **영지식:** 아니오 ·
> **양자 내성:** 그렇게 여겨짐(해시 기반) · **이 저장소에서 돌리는 코드:** lambdaworks의 STARK 증명자

## 무엇인가

STARK는 계산을 **실행 트레이스** — 단계마다 한 행인 표 — 와 **AIR** — 각 행과 이웃한 두 행이 따라야 하는 다항식 규칙 — 로
적습니다. 각 열을 큰 소수 필드의 곱셈 부분군 위에서 보간하고 몇 배 큰 영역으로 늘려서, 틀린 트레이스가 낮은 차수에서 멀리 떨어진
다항식이 되게 합니다. 증명자는 늘린 열에 머클 트리로 커밋합니다.

그다음 검증자는 트레이스 바깥의 무작위 점에서 규칙을 확인하게 하고(**DEEP** 방식), 커밋한 것이 정말 낮은 차수의 다항식에
가깝다는 증명을 요구합니다(**FRI** 근접성 시험). 대화 기록을 해시하면 비대화형이 됩니다. 결과는 계산이 아무리 길어도 수백
킬로바이트이고 몇 밀리초에 확인되는 증명입니다.

2018년 논문과 StarkWare의 ethSTARK 설명은 이 전시품이 쓰는 252비트 Stark 필드 같은 큰 필드를 씁니다. 뒤의 STARK는 속도를 위해 더
작은 필드로 옮겨 갔습니다. [Winterfell](../winterfell/README.ko.md)은 62비트와 64비트 필드를 주고,
[Plonky3](../plonky3/README.ko.md)와 [Circle STARK](../circle-stark/README.ko.md)는 31비트 필드를 씁니다.

## 역사

- **2018년 1월.** Eli Ben-Sasson, Iddo Bentov, Yinon Horesh, Michael Riabzev가 *Scalable, transparent, and post-quantum secure
  computational integrity*를 공개합니다. 시연은 어떤 DNA 프로필이 경찰 데이터베이스에 없다는 것을 증명합니다.
- **2018년 7월.** Ethereum 재단이 프로덕션급 STARK인 ethSTARK에 자금을 댑니다.
- **2020년 6월.** StarkEx가 Ethereum 메인넷에서 프로덕션에 들어갑니다.
- **2021년 5월.** ethSTARK 문서가 공개됩니다.
- **2023년 8월.** StarkWare가 StarkEx와 Starknet 뒤의 증명자 Stone을 Apache-2.0으로 공개합니다.
- **2024년 9월.** Stone 저장소의 마지막 커밋.
- **2025년.** 논문 둘이 FRI와 DEEP-FRI 매개변수가 기대던 Reed–Solomon 근접성 간격 추측을 반증합니다.
- **2025년 11월 3일.** Circle STARK 증명자 S-two가 Starknet 메인넷에서 Stone을 대신합니다.

## 장점

- **신뢰 설정이 없습니다.** 안전성이 해시 함수에 기댑니다.
- **양자 컴퓨터에 견딘다고 여겨집니다.** 타원곡선을 쓰지 않습니다.
- **아주 긴 계산도 검증이 아주 빠릅니다.**
- **StarkWare에서의 긴 프로덕션 기록.**

## 단점

- **증명이 큽니다.** 100킬로바이트 안팎이거나 그 이상입니다.
- **증명자가 무겁습니다.** 큰 필드 산술과 큰 트레이스에 메모리가 많이 듭니다.
- **추측에 기댄 매개변수.** 2025년의 반증이 추측 보안 매개변수가 가정한 추측을 건드렸습니다.
- **설명된 그대로는 영지식이 아닙니다.** ethSTARK는 가린다고 주장하지 않고, Stone에는 가리는 코드가 없습니다.

## 얼마나 차세대인가

이제는 아닙니다. STARK는 오늘 가장 빠른 증명자들이 속한 해시 기반 계보를 시작했지만, 고전적인 큰 필드 버전은 StarkWare에서도
작은 필드와 원 구성으로 바뀌었습니다.

## 지금의 상태 (2026년 9월 기준)

**Starknet에서는 S-two로 대체됨.** 2025년 11월부터입니다. Stone은 멈춰 있고, StarkEx가 아직 Stone으로 증명하는지는 확인되지
않았습니다.

## 이 저장소에서 돌리는 방법

`crates/sys-stark`는 ethSTARK 설명을 따르는 Rust 구현인 lambdaworks의 STARK 증명자를 커밋 하나에 고정해 씁니다. StarkWare 자체의
Stone 증명자는 C++이라 이 저장소가 빌드하지 않습니다. 문장마다 회로의 와이어를 열로 가진 트레이스가 되고, 게이트마다 규칙
하나가 붙으며, 공개 입력은 첫 행에서 확인합니다.

트레이스는 회로의 행을 네 번 되풀이한 것으로, lambdaworks의 FRI가 커밋하는 가장 적은 행 수입니다. 그러면 모든 열이 상수라서,
증명이 트레이스 바깥의 점에서 싣는 값이 곧 와이어 값입니다. lambdaworks는 필드 원소를 Montgomery 형태로 적고, 박물관의 검사도 그
형태를 찾습니다. one-plus-one과 age에서 솔트를, password에서 PIN을, pool-spend에서 지출 키와 머클 형제 노드를 찾아냅니다.
lambdaworks의 128비트 추측 보안 설정에서 증명은 one-plus-one의 약 230 KB부터 pool-spend의 약 3.7 MB까지이고, 대부분은 열 전부를
싣는 FRI 질의 열기 55개입니다. 증명에는 20비트 작업 증명 탐색이 들어 있어 시간이 실행마다 달라집니다. 증명자는 증인을 확인하지
않습니다. 거짓 주장도 증명이 되고, 검증자가 거절합니다.

`/run stark age`를 친 다음 `/run winterfell age`, `/run circle-stark age`와 비교해 보세요. STARK 필드의 세 세대입니다.

## 출처

- E. Ben-Sasson, I. Bentov, Y. Horesh, M. Riabzev, *Scalable, transparent, and post-quantum secure computational integrity* — https://eprint.iacr.org/2018/046
- StarkWare, *ethSTARK Documentation* — https://eprint.iacr.org/2021/582
- StarkWare, *Open-sourcing the battle-tested Stone prover* (2023) — https://starkware.co/blog/open-sourcing-the-battle-tested-stone-prover/
- Stone 증명자 저장소 — https://github.com/starkware-libs/stone-prover
- StarkEx — https://starkware.co/starkex/
- Starknet, *S-two is live on Starknet mainnet* (2025) — https://www.starknet.io/blog/s-two-is-live-on-starknet-mainnet-the-fastest-prover-for-a-more-private-future/
- 근접성 간격 반례(2025) — https://eprint.iacr.org/2025/2046 · https://eprint.iacr.org/2025/2010
- lambdaworks — https://github.com/lambdaclass/lambdaworks
