# Nova

폴딩을 처음 내놓은 증명 시스템입니다. 다음 증명 안에서 이전 증명 전체를 검사하는 대신 주장 둘을 하나로 접어서, 긴 계산을
거의 비용 없이 한 단계씩 증명하고 — 마지막에 짧은 영지식 증명으로 압축할 수 있습니다.

> **선반:** Others · **처음 발표:** 2021 · **신뢰 설정:** 없음(IPA 약속일 때) · **영지식:** 예(압축한 증명) ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** Microsoft의 `nova-snark`

## 무엇인가

**점진적으로 검증 가능한 계산**(IVC)은 긴 계산을 한 단계씩 증명합니다. 단계마다 지금까지의 모든 단계가 옳았다는 증명이
있습니다. 고전적인 방식은 이전 증명의 검증자를 다음 단계의 회로 안에 넣는데, 이것이 비쌉니다.

Nova는 단계 사이에 넘기는 것을 바꿉니다. 오차 항과 스칼라를 더해 두 인스턴스를 더할 수 있게 한 R1CS, 곧 **완화된 R1CS**를
씁니다. **폴딩 방식**은 쌓여 온 인스턴스와 새 인스턴스를 무작위 챌린지로 합쳐 하나로 만들고, 합친 인스턴스는 둘이 모두
만족될 때만 만족됩니다. 단계마다 회로는 접기만 확인하면 되고, 그 비용은 단계가 아무리 커도 스칼라 곱셈 두 번 — 제약 약
1만 개 — 입니다.

마지막에 접힌 인스턴스를 Spartan식 SNARK로 증명해 증명을 몇 킬로바이트로 줄입니다. 구현은 압축 직전에 무작위 인스턴스를
하나 더 접어서, 압축한 증명이 영지식이 되게 합니다.

## 역사

- **2021년 3월.** Abhiram Kothapalli, Srinath Setty, Ioanna Tzialla가 Nova를 공개하고, CRYPTO 2022에 실립니다.
- **2022년 12월.** SuperNova가 단계가 모두 같지는 않은 프로그램으로 폴딩을 넓힙니다.
- **2023년 6월.** Nguyen, Boneh, Setty가 곡선 두 개의 순환 위에 지은 원래 구현이 건전하지 않았음을 보입니다. MinRoot 지연
  함수 2⁷⁵ 라운드의 증명을 1.46초에 위조합니다. 구현은 안전성 증명이 있는 변형으로 옮겨 갑니다.
- **2023년 8월.** CycleFold가 폴딩이 두 번째 곡선을 쓰는 방식을 단순하게 만듭니다.
- **2024년 10월.** NeutronNova가 SHA-256 회로를 Nova보다 약 열 배 빠르게 접습니다. `nova-snark`는 이것을 실험 기능으로 넣습니다.
- **2026년 9월.** `nova-snark` 0.76.0.

## 장점

- **발표 당시 가장 작은 재귀 부담.** 폴딩은 단계마다 일정한 수의 연산만 듭니다.
- **FFT도, 신뢰 설정도 없습니다.** 기본 내적 약속을 쓸 때입니다.
- **압축한 증명이 짧습니다.** 몇 킬로바이트이고, 영지식입니다.

## 단점

- **구현이 까다롭습니다.** 2023년 공격은 폴딩 아이디어 자체가 아니라, 증명 시스템이 곡선 순환을 쓰는 방식에서 나왔습니다.
- **압축하지 않은 증명은 큽니다.** 단계 하나만큼 크고, 무언가를 감춘다고 주장하지 않습니다.
- **양자 내성이 없습니다.** 이산 로그에 기댑니다.

## 얼마나 차세대인가

아주 그렇습니다. Nova가 시작한 폴딩이 HyperNova, ProtoGalaxy, Aztec의 클라이언트 쪽 증명 뒤에 있고, NeutronNova 같은 더
빠른 변형으로 연구가 이어집니다. Nova 자체는 뒤의 방식들이 자기를 재는 기준점입니다.

## 지금의 상태 (2026년 9월 기준)

**실험 중.** `nova-snark`는 꾸준히 릴리스되지만, Nova 폴딩의 프로덕션 배포는 확인되지 않았습니다. 이 크레이트에 기대는 한
프로젝트는 HyperKZG 약속만 씁니다.

## 이 저장소에서 돌리는 방법

`crates/sys-nova`는 Pallas–Vesta 곡선 순환 위에서 `nova-snark`를 씁니다. 박물관의 문장마다 두 단계짜리 계산의 단계 함수가
됩니다. 공개 입력이 단계에서 단계로 넘어가는 상태이고, 단계는 비밀을 비공개 조언으로 받아 문장을 확인합니다. 두 단계를 접고,
결과를 SNARK로 압축하고, 검증자는 그 압축한 증명을 공개 입력에 대해 확인합니다.

문장 자체는 작습니다. one-plus-one은 제약 34개, pool-spend는 707개입니다. 단계마다 Nova의 증강 회로 안에서 돌고, 이 회로가
넘기는 상태를 해시하고 접기를 확인하기 때문에 모든 문장이 대략 제약 1만~1만 3천 개가 됩니다. 논문이 말하는 재귀의 일정한 부담이
문장보다 훨씬 큰 것입니다. 설정은 25~35 MB의 공개 매개변수와 키를 만듭니다. 압축한 증명은 모든 문장에서 약 10.6 KB입니다. Nova의
증명자는 아무것도 확인하지 않습니다. 거짓 주장도 참인 주장처럼 접히고 압축되며, 검증자가 거절합니다.

`/run nova one-plus-one`을 친 다음 `/run spartan one-plus-one`과 비교해 보세요. Nova는 Spartan식 SNARK로 압축합니다.

## 출처

- A. Kothapalli, S. Setty, I. Tzialla, *Nova: Recursive Zero-Knowledge Arguments from Folding Schemes*, CRYPTO 2022 — https://eprint.iacr.org/2021/370
- Nova 저장소와 README — https://github.com/microsoft/Nova
- A. Kothapalli, S. Setty, *SuperNova* — https://eprint.iacr.org/2022/1758
- W. Nguyen, D. Boneh, S. Setty, *Revisiting the Nova Proof System on a Cycle of Curves* — https://eprint.iacr.org/2023/969
- A. Kothapalli, S. Setty, *CycleFold* — https://eprint.iacr.org/2023/1192
- A. Kothapalli, S. Setty, *NeutronNova* — https://eprint.iacr.org/2024/1606
- crates.io의 nova-snark — https://crates.io/crates/nova-snark
