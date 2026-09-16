# Spartan

sum-check 프로토콜로 만든, 신뢰 설정 없는 산술 회로용 zkSNARK입니다. 2019년 Microsoft Research의 Srinath Setty가
발표했고, 그 방식은 지금 World의 ProveKit부터 Jolt와 Binius64까지 여러 증명자 안에 들어 있습니다.

> **선반:** Others · **처음 발표:** 2019 · **신뢰 설정:** 없음 · **영지식:** 예 ·
> **양자 내성:** 여기서 쓰는 구현은 아니오(이산 로그) · **이 저장소에서 돌리는 코드:** Microsoft의 `spartan` 크레이트

## 무엇인가

R1CS 인스턴스는 이렇게 말합니다. 행렬 `A, B, C`와 할당 `z`에 대해 모든 행이 `(A·z) × (B·z) = (C·z)`를 만족한다.
Spartan은 행들의 오차를 불 초입방체 위의 다중선형 다항식으로 적고 무작위 다항식을 곱해서, 모든 행이 만족될 때(무시할 만한
확률을 빼면) 그리고 그때만 초입방체 위의 합 하나가 0이 되게 합니다. 그다음 **sum-check 프로토콜**이 「이 합은 0이다」를
무작위 점 하나에서 다항식 몇 개의 값을 구하는 문제로 줄이고, 증명자는 그 값을 다항식 약속으로 증명합니다.

간결하고 빠르게 만드는 아이디어가 둘 더 있습니다. **계산 약속**은 행렬을 한 번 전처리해 두어서, 검증자가 증명마다 회로
전체를 읽지 않게 합니다. **SPARK**는 조밀한 다중선형 다항식용 약속 방식을 희소한 다항식용으로 바꿔서, 증명자의 작업이 0이
아닌 항목 수에 비례하게 유지합니다.

다항식 약속은 바꿔 끼울 수 있습니다. 원래 라이브러리는 ristretto255 위의 Hyrax식 약속을 쓰는데, 설정이 필요 없고 이산
로그에 기댑니다. 뒤의 시스템들은 KZG나 WHIR 같은 해시 기반 약속을 끼웁니다.

## 역사

- **2019년 5월 23일.** Srinath Setty가 Spartan을 공개합니다. 신뢰 설정이 없고, 검증이 준선형이고, 증명자 시간이 회로에
  선형인 zkSNARK 계열입니다. CRYPTO 2020에 실립니다.
- **2023년 7월.** 새 구현 Spartan2가 시작되고, 저장소는 뒤에 Vega로 이름이 바뀝니다.
- **2025년 1월.** Setty와 Thaler의 Twist and Shout이 PLONK 모양 제약용 SpeedySpartan과, Spartan보다 증명이 약 여섯 배
  빠른 Spartan++를 내놓습니다.
- **2026년.** Kaviani와 Setty의 Vega가 모바일 운전면허증에서 나이에 관한 사실을 일반 클라이언트 기기에서 약 92 ms에
  증명합니다.
- **2026년 9월 2일.** World가 ProveKit — Noir 회로를 Spartan과 WHIR로 증명 — 을 프로덕션 준비 완료로 소개합니다.

## 장점

- **신뢰 설정이 없습니다.**
- **증명자가 빠릅니다.** 회로 크기에 선형이고 FFT가 없습니다.
- **유연합니다.** sum-check 핵심은 이산 로그·페어링·해시 기반 등 어떤 다중선형 약속과도 맞물려서, 양자 내성이 있을 법한
  형태로도 만들 수 있습니다.

## 단점

- **증명이 큽니다.** 페어링 SNARK의 수백 바이트 대신 수십~수백 킬로바이트입니다. 제약 백만 개에서 원래 라이브러리의
  증명은 48 KB(NIZK)와 142 KB(SNARK)이고, 같은 비교에서 Groth16은 128바이트입니다.
- **감사받지 않은 참조 코드.** `spartan` 크레이트의 README는 보안 검토나 감사를 받지 않았다고 적습니다.
- **여기서 쓰는 구현은 양자 내성이 없습니다.** Hyrax식 약속이 이산 로그에 기댑니다.

## 얼마나 차세대인가

지금의 흐름 한가운데에 있습니다. sum-check 기반 증명은 2020년대의 주된 방향 중 하나이고, Spartan은 그것이 일반 회로에
실용적일 수 있음을 보인 설계입니다. 그 아이디어가 Jolt의 영지식 모드, Binius64의 영지식 감싸기, ProveKit에 쓰입니다.

## 지금의 상태 (2026년 9월 기준)

**후손을 통해 쓰이는 중.** 원래 `spartan` 크레이트는 지금도 관리되고, Vega와 ProveKit가 이 설계를 프로덕션을 겨냥한
시스템으로 가져갑니다.

## 이 저장소에서 돌리는 방법

`crates/sys-spartan`은 Microsoft의 `spartan` 크레이트 0.9.0을 ristretto255 위에서 씁니다. 박물관의 R1CS를 희소 행렬
`A, B, C`로 넘기고, 공개 입력은 Spartan의 입력 칸에 넣어 검증자가 그 값에 기대게 하며, Spartan이 요구하는 2의 거듭제곱
크기로 채웁니다.

여기서 증명은 `one-plus-one`의 23,488바이트부터 `pool-spend`의 50,520바이트까지입니다. 이 전시품은 Spartan의 `SNARK`
모드를 씁니다. 행렬을 한 번 약속해 두어 검증자가 다시 읽지 않는 모드입니다. `NIZK` 모드도 증인을 똑같이 감추지만,
검증자가 회로 전체를 읽어야 합니다.

`/run spartan pool-spend`를 친 다음 `/run bulletproofs pool-spend`와 비교해 보세요. 둘 다 설정 없는 이산 로그 시스템인데
비용은 아주 다릅니다.

## 출처

- S. Setty, *Spartan: Efficient and general-purpose zkSNARKs without trusted setup*, CRYPTO 2020 — https://eprint.iacr.org/2019/550
- Spartan 라이브러리 — https://github.com/microsoft/Spartan
- Spartan2, 지금의 Vega — https://github.com/microsoft/vega-prover
- S. Setty, J. Thaler, *Twist and Shout* — https://eprint.iacr.org/2025/105
- D. Kaviani, S. Setty, *Vega* — https://eprint.iacr.org/2025/2094
- World, *ProveKit: privacy for the real world* — https://world.org/blog/engineering/provekit-privacy-for-the-real-world
- a16z crypto, *Jolt zero-knowledge* — https://a16zcrypto.com/posts/article/zkvm-jolt-zero-knowledge/
- Binius64 영지식 증명자 설정 — https://github.com/binius-zk/binius64/blob/main/crates/prover/src/zk_config.rs
