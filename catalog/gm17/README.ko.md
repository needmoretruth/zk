# GM17

이듬해에 나온 Groth16의 형제입니다. 증명은 똑같이 작은데, 누구도 그것을 다른 유효한 증명으로 바꿀 수 없습니다. 그 성질
덕분에 지식의 서명이 되고, 대가로 증명자가 느려지고 설정이 커집니다.

> **선반:** Others · **처음 발표:** 2017 · **신뢰 설정:** 회로마다 · **영지식:** 예 ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** arkworks의 `ark-gm17`

## 무엇인가

[Groth16](../groth16/README.ko.md)처럼 GM17도 그 회로를 위한 신뢰 설정 뒤에 타원곡선 점 셋 — G1에 둘, G2에 하나 — 으로 회로를
증명합니다. 다른 점은 공격자가 본 증명으로 무엇을 할 수 있느냐입니다.

Groth16 증명은 **다시 무작위화**할 수 있습니다. 누구나 유효한 증명을 같은 문장에 대한, 달라 보이는 다른 유효한 증명으로 바꿀
수 있습니다. 많은 경우 해가 없지만, 증명의 바이트를 식별자로 쓰는 경우 같은 곳에서는 문제가 됩니다. GM17은
**시뮬레이션 추출 가능**합니다. 공격자가 고른 문장의 증명을 포함해 증명을 아무리 많이 봐도, 증인을 모르면 새 유효한 증명을
만들 수 없습니다. 그래서 문장에 메시지를 접어 넣은 GM17 증명은 짧은 지식의 서명으로 쓰입니다.

대가는 연산에 있습니다. GM17은 회로를 제곱 산술 프로그램으로 적어 곱셈 하나를 제곱 둘로 세므로, 증명자가 Groth16보다 일을
더 하고 설정에 원소가 더 들어갑니다. 검증자는 페어링 식 둘, 모두 페어링 다섯 번을 확인합니다. Groth16은 세 번입니다.

## 역사

- **2017년 6월.** Jens Groth와 Mary Maller가 *Snarky Signatures*를 공개하고, CRYPTO 2017에 실립니다. 이 논문은 이런 종류의
  페어링 기반 방식에서 군 원소 셋과 검증 식 둘이 최소라는 것도 증명합니다.
- **2023년 9월.** arkworks GM17 저장소의 마지막 커밋.
- **2023년 11월.** ZoKrates의 마지막 릴리스. ZoKrates는 GM17을 증명 방식으로 제공하고, Groth16의 가변성이 문제가 되는 곳에
  GM17 같은 가변성 없는 방식을 권합니다.

## 장점

- **가변성이 없습니다.** 본 증명을 새 증명으로 바꿀 수 없습니다.
- **Groth16만큼 작습니다.** 군 원소 셋입니다.
- **지식의 서명.** 문장에 메시지를 넣어 증명으로 서명할 수 있습니다.

## 단점

- **Groth16보다 느립니다.** 증명도 검증도 느리고, 설정도 큽니다.
- **회로마다 신뢰 설정이 필요합니다.** Groth16과 같습니다.
- **양자 내성이 없습니다.** 페어링에 기댑니다.

## 얼마나 차세대인가

별로 아닙니다. GM17은 「가변성 없는 SNARK는 얼마나 작을 수 있나」라는 정확한 질문에 잘 답했지만, 그 뒤로 흐름은 범용
설정·투명한 시스템·폴딩으로 옮겨 갔습니다. 남은 기여는 최적성 결과와, SNARK로 만드는 지식의 서명이라는 아이디어입니다.

## 지금의 상태 (2026년 9월 기준)

**역사적 의미.** 확인된 프로덕션 사용처가 없습니다. 구현은 libsnark(`r1cs_se_ppzksnark`, 프로토콜을 고친 버전)와 arkworks에
남아 있고, 이것을 제공하는 ZoKrates는 스스로를 개념 증명이라고 하며 2023년 이후 릴리스가 없습니다.

## 이 저장소에서 돌리는 방법

`crates/sys-gm17`은 arkworks의 `ark-gm17` 0.3.0을 공개된 버전 그대로 BLS12-381 위에서 씁니다. 박물관의 R1CS를 arkworks의
제약 API로 한 행씩 넘기고, 공개 입력을 검증자의 입력으로 둡니다. 증명은 이 곡선 위의 Groth16과 같은 192바이트입니다.

GM17을 규정하는 차이는 이 전시품이 아직 보여 주지 못합니다. 박물관의 바이트 뒤집기 공격은 점의 인코딩을 깨뜨리므로 GM17도
Groth16도 결과를 형식 오류로 거절합니다. 둘을 가르는 공격은 증명의 점 두 개에 음수를 취하는 것입니다. `e(−A, −B) = e(A, B)`
이므로 Groth16은 바뀐 증명을 받아들이고 GM17은 받아들이지 않습니다.

`/run gm17 one-plus-one`을 친 다음 `/run groth16 one-plus-one`과 비교해 보세요.

## 출처

- J. Groth, M. Maller, *Snarky Signatures: Minimal Signatures of Knowledge from Simulation-Extractable SNARKs*, CRYPTO 2017 — https://eprint.iacr.org/2017/540
- libsnark `r1cs_se_ppzksnark` — https://github.com/scipr-lab/libsnark/blob/master/libsnark/zk_proof_systems/ppzksnark/r1cs_se_ppzksnark/r1cs_se_ppzksnark.hpp
- arkworks GM17 — https://github.com/arkworks-rs/gm17
- ZoKrates 증명 방식 — https://zokrates.github.io/toolbox/proving_schemes.html
- ZoKrates 저장소 — https://github.com/Zokrates/ZoKrates
