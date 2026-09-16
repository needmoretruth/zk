# Winterfell

Meta의 블록체인 연구 그룹이 2021년에 내놓은 범용 STARK 증명자이고, Polygon Miden이 4년 동안 이 위에서 돌았습니다. 간결하지만
README 스스로 밝히듯 영지식은 아닙니다. 그래서 박물관에서 두 성질이 서로 다르다는 것을 가장 분명하게 보여 주는 예입니다.

> **선반:** Polygon · **처음 발표:** 2021 · **신뢰 설정:** 없음 · **영지식:** 아니오 ·
> **양자 내성:** 그렇게 여겨짐(해시 기반) · **이 저장소에서 돌리는 코드:** Winterfell 자체 크레이트 `winter-prover`

## 무엇인가

계산을 **실행 트레이스** — 기계의 연속된 상태를 행으로 적은 표 — 와 **AIR** — 이웃한 두 행이 모두 만족해야 하는 규칙과,
첫 행의 입력 같은 특정 칸에 대한 단언 — 로 적습니다. Winterfell은 열마다 더 큰 영역 위의 다항식으로 늘리고, 모든 규칙을
합성 다항식 하나로 묶어 트레이스 바깥의 무작위 점에서 확인하고(DEEP 방식), 커밋된 것이 정말 낮은 차수의 다항식이라는
것을 FRI로 증명합니다.

필드는 62비트·64비트·128비트 소수 필드 셋과 챌린지용 확장체를 주고, 해시는 SHA3·BLAKE3부터 대수적 해시 Rescue Prime까지
여럿을 줍니다. 고른 매개변수에 대한 추측 보안 수준과 증명된 보안 수준도 알려 줍니다.

하지 않는 일은 트레이스를 감추는 것입니다. README는 「간결한 증명을 주지만 완전한 영지식은 아니다」라고 적습니다.
Winterfell 증명은 계산이 옳았음을 검증자에게 납득시키고, 계산의 값 일부가 증명에 실려 갑니다.

## 역사

- **2021년 4월.** Meta의 블록체인 연구 그룹 Novi Research가 저장소를 엽니다. 닷새 뒤 완전한 영지식을 요청하는 이슈가
  열리고, 지금도 열려 있습니다.
- **2021년 8월 4일.** Meta가 Irakliy Khaburzaniya, Kostas Chalkias, Harjasleen Malvai, Kevin Lewi의 Winterfell을 발표합니다.
- **2021년 11월.** Polygon이 STARK 기반 롤업 Polygon Miden을 발표합니다. Winterfell 개발을 이끈 전 Meta 연구자가 이끌고,
  Miden VM이 Winterfell로 증명합니다.
- **2022–2025년.** 무작위 AIR, 증명된 보안 추정, Lagrange 커널 제약, 한동안은 LogUp-GKR이 더해집니다. 영지식을 더하는
  풀 리퀘스트가 2024년에 열리지만 병합되지 않습니다.
- **2025년 7월 19일.** 지금까지의 마지막 릴리스인 버전 0.13.1.
- **2026년 2월 14일.** Miden VM 0.21이 Winterfell 백엔드를 Plonky3로 바꿉니다.

## 장점

- **AIR를 쓰기 쉽습니다.** 이웃한 두 행에 대한 규칙과 칸에 대한 단언이면 됩니다.
- **빠르고 이식성이 좋습니다.** 멀티스레드이고, 표준 라이브러리 없이도, WebAssembly로도 빌드됩니다.
- **신뢰 설정이 없고** 안전성이 해시에 기댑니다.
- **보안 추정이 들어 있습니다.** 고른 필드·해시·매개변수에 대해 계산해 줍니다.

## 단점

- **영지식이 아닙니다.** 증명이 트레이스의 값을 드러낼 수 있어서, 입력을 비밀로 지켜야 하는 곳에는 맞지 않습니다.
- **감사받지 않았습니다.** README가 프로덕션용이 아닌 연구 프로젝트라고 적습니다.
- **개발이 멈췄습니다.** 2025년 7월 이후 릴리스가 없고, 주 사용처가 떠났습니다.
- **증명이 큽니다.** 다른 STARK처럼, README는 96비트 보안에서 백만 단계 해시 사슬에 128 KB를 보고합니다.

## 얼마나 차세대인가

Winterfell은 62~128비트의 큰 필드 위에 지은, 실용적인 Rust STARK의 첫 세대입니다. 그 뒤로 흐름은 SIMD 연산에 맞는 31비트
필드와 이진 필드(Plonky3, S-two, Binius), 그리고 가리는 약속으로 옮겨 갔고, Winterfell의 주 사용처도 따라갔습니다. AIR
인터페이스와 DEEP-FRI 설계는 STARK가 어떻게 돌아가는지 보여 주는 분명한 예로 남아 있습니다.

## 지금의 상태 (2026년 9월 기준)

**Polygon Miden에서는 Plonky3로 대체됨.** 코드는 MIT로 공개돼 있고 몇몇 프로젝트가 기대고 있지만, 프로덕션 사용처는
찾지 못했고 개발은 멈췄습니다.

## 이 저장소에서 돌리는 방법

`crates/sys-winterfell`은 Winterfell의 크레이트를 공개된 그대로 씁니다. 박물관의 문장은 한 번짜리 회로라서, 각 문장이
회로의 와이어를 열로 가진 똑같은 행들의 트레이스가 됩니다. 게이트마다 전이 규칙 하나, 그리고 이웃한 행을 같게 묶는 규칙이
붙고, 공개 입력은 첫 행에 대한 단언입니다.

트레이스는 똑같은 행 여덟과 단계 카운터 열 하나로 됩니다. Winterfell 증명자는 트레이스에서 만든 다항식의 차수를 확인하는데,
상수 열로만 된 트레이스는 그 확인을 통과하지 못합니다. 와이어 열이 상수라서, 증명이 검증자의 무작위 점에서 여는 행은 회로의 행
그 자체와 바이트까지 같습니다. 박물관의 검사는 one-plus-one과 age에서 솔트를, password에서 PIN을 찾아냅니다. Winterfell
트레이스의 열은 255개보다 적어야 해서, membership(와이어 321개)과 pool-spend(와이어 1,008개)는 이 한 행짜리 배치에 들어가지 않아
지원하지 않음으로 표시됩니다. 약 96비트 보안으로 문서화된 Winterfell 매개변수에서 증명 크기는 one-plus-one의 약 25 KB부터
sudoku의 약 64 KB까지입니다.

`/run winterfell age`를 친 다음 `/run plonky3 age`와 비교해 보세요. 둘 다 STARK인데 하나만 비밀을 감춥니다.

## 출처

- Winterfell 저장소와 README — https://github.com/facebook/winterfell
- Meta Engineering, *Winterfell: A STARK prover and verifier* (2021) — https://engineering.fb.com/2021/08/04/open-source/winterfell/
- 이슈 #9, *Implement perfect zero-knowledge* — https://github.com/facebook/winterfell/issues/9
- 풀 리퀘스트 #293, *Adding zero-knowledge* — https://github.com/facebook/winterfell/pull/293
- Winterfell 변경 기록 — https://github.com/facebook/winterfell/blob/main/CHANGELOG.md
- Polygon, *Polygon Miden, a STARK-based Ethereum-compatible rollup* (2021) — https://polygon.technology/blog/polygon-announces-polygon-miden-a-stark-based-ethereum-compatible-rollup
- Miden VM 변경 기록 — https://github.com/0xMiden/miden-vm/blob/next/CHANGELOG.md
- winter-prover의 역의존성 — https://crates.io/crates/winter-prover/reverse_dependencies
