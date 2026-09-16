# Marlin

회로마다 의식을 치르는 대신, 한 번의 설정이 일정 크기까지의 모든 회로에 쓰이는 페어링 기반 SNARK입니다. 선형 크기의 범용
설정을 처음 가진 Sonic의 뒤를 이어 증명도 검증도 더 빨라졌고, 이것을 배치로 묶은 후손 Varuna가 Aleo의 트랜잭션을 증명합니다.

> **선반:** Others · **처음 발표:** 2019 · **신뢰 설정:** 범용 · 갱신 가능 · **영지식:** 예 ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** arkworks의 `ark-marlin`

## 무엇인가

[Groth16](../groth16/README.ko.md)은 회로마다 신뢰 설정이 필요합니다. Marlin의 설정은 그 대신 최대 크기까지 어떤 회로에든
쓰이는 **구조화된 참조 문자열**(SRS)을 만듭니다. 나중에 누구든 새 무작위 값으로 갱신할 수 있고, 참여자 중 한 명만 정직했으면
안전합니다.

Marlin은 두 층으로 지어져 있습니다. 첫째는 **대수적 홀로그래픽 증명**입니다. 증명자가 다항식을 보내고, 검증자는 회로 자체가
아니라 인덱스라고 부르는 회로의 인코딩만 읽는 프로토콜입니다. 둘째는 KZG식 **다항식 약속**으로, 그 다항식을 짧은 약속과
열기로 바꿉니다. 회로의 인덱싱은 한 번만 하고, 그 뒤로 증명마다 확인이 빠릅니다.

Marlin 증명은 G1 점 13개와 필드 원소 8개로, BLS12-381에서 880바이트입니다. Groth16은 192바이트입니다. 검증에는 페어링이 두 번
듭니다.

## 역사

- **2019년 9월.** Alessandro Chiesa, Yuncong Hu, Mary Maller, Pratyush Mishra, Psi Vesely, Nicholas Ward가 Marlin을 공개하고,
  EUROCRYPT 2020에 실립니다. 논문은 Sonic보다 증명이 약 열 배, 검증이 약 세 배 빠르다고 보고합니다.
- **2021년 6월.** 마지막 릴리스인 `ark-marlin` 0.3.0. 저장소는 스스로를 프로덕션용이 아닌 학술 프로토타입이라고 적고,
  마지막 커밋은 2022년 8월입니다.
- **2024년 9월 18일.** Aleo 메인넷이 Varuna로 시작합니다. Aleo는 Varuna를 배치를 더한 Marlin의 발전형이라고 설명합니다.
- **2025년 7월.** Provable이 배치의 안전성을 고치려고 Varuna 프로토콜에 라운드 하나를 더합니다.
- **2026년 6월.** Veria Labs가 Aleo의 구현인 snarkVM에서 증명 위조를 공개합니다. 배치 확인이 증인 값을 먼저 흡수하지 않고
  무작위 가중치를 뽑았습니다. 4월 29일에 보고되고 5월 1일에 고쳐졌으며, Aleo는 누군가 악용한 흔적을 찾지 못했습니다.

## 장점

- **설정 하나로 여러 회로.** 참조 문자열이 범용이고 누구든 갱신할 수 있습니다.
- **Sonic보다 빠릅니다.** 범용 설정을 먼저 가진 방식보다 증명도 검증도 빠릅니다.
- **증명 크기는 여전히 일정합니다.** 1킬로바이트 아래입니다.

## 단점

- **증명이 같은 곡선의 Groth16보다 약 4.5배 큽니다.**
- **여전히 신뢰 설정이 있습니다.** 회로마다가 아니라 범용일 뿐입니다.
- **양자 내성이 없습니다.** 페어링에 기댑니다.
- **코드로 옮기다 틀리기 쉽습니다.** Aleo의 2026년 위조는 논문이 아니라, 해시가 흡수하지 않은 값에서 나왔습니다.

## 얼마나 차세대인가

지금은 별로 아닙니다. Marlin은 SNARK를 다항식에 대한 프로토콜과 다항식 약속이라는 떼어 낼 수 있는 두 층으로 지은 2019년
논문 중 하나이고, 지금 대부분의 SNARK가 그렇게 설명됩니다. 그 뒤로 범용 설정의 페어링 SNARK는 대체로 사용자 정의 게이트가 있는
PLONK 계열로 옮겨 갔고, 새 연구의 상당수는 투명한 해시 기반 시스템으로 옮겨 갔습니다.

## 지금의 상태 (2026년 9월 기준)

**Varuna로 쓰이는 중.** Aleo 메인넷은 2024년 9월부터 Marlin을 배치로 묶은 변형 Varuna로 증명합니다. 이 전시품이 돌리는
arkworks의 Marlin은 관리되지 않는 프로토타입입니다.

## 이 저장소에서 돌리는 방법

`crates/sys-marlin`은 arkworks의 `ark-marlin` 0.3.0을 KZG식 약속과 함께 BLS12-381 위에서 씁니다. 설정은 실제로 두 단계입니다.
회로 크기에 맞춘 범용 참조 문자열을 만들고, 그다음 그 회로의 인덱스를 만듭니다.

증명은 one-plus-one부터 pool-spend까지 모든 문장에서 arkworks 인코딩으로 951바이트입니다. 실행마다 회로에 맞춘 참조
문자열을 만듭니다. 작은 문장은 약 80 KB, pool-spend는 약 5 MB이고, 인덱싱이 이것을 0.6 MB~40 MB의 키로 바꿉니다. 가장 큰
회로에 맞춘 문자열 하나로 일곱 문장을 모두 처리할 수도 있습니다. 박물관이 뒤집는 바이트는 증명이 주장한 평가값 하나 안에서 유효한
수로 남으므로, 증명은 그대로 해독되고 검증자 자신의 확인이 거절합니다. 디버그 단언이 켜진 빌드에서는 arkworks 증명자가 거짓 주장에서
멈추고, 릴리스 빌드에서는 증명을 만들며 검증자가 그것을 거절합니다.

`/run marlin one-plus-one`을 친 다음 `/run groth16 one-plus-one`과 비교해 보세요.

## 출처

- A. Chiesa, Y. Hu, M. Maller, P. Mishra, P. Vesely, N. Ward, *Marlin: Preprocessing zkSNARKs with Universal and Updatable SRS*, EUROCRYPT 2020 — https://eprint.iacr.org/2019/1047
- arkworks Marlin — https://github.com/arkworks-rs/marlin
- Aleo, *Announcing Aleo Mainnet* (2024) — https://aleo.org/post/announcing-aleo-mainnet/
- Provable, *Updates to Aleo records and Varuna* (2025) — https://provable.com/blog/updates-to-aleo-records-and-varuna
- Veria Labs, *Forging transactions on Aleo* (2026) — https://verialabs.com/blog/forging-transactions-on-aleo
- snarkVM — https://github.com/ProvableHQ/snarkVM
