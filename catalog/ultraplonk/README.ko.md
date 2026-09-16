# UltraPlonk

PLONK에 Aztec이 두 가지를 더한 것입니다. 행 하나가 더 많은 일을 하는 사용자 정의 게이트, 그리고 값을 비트 단위로
분해하는 대신 미리 계산한 목록과 대조하는 조회 테이블입니다. 2025년 Aztec이 UltraHonk로 바꿀 때까지 Noir 프로그램이
이것으로 증명됐습니다.

> **선반:** Aztec · **처음 발표:** 2020 · **신뢰 설정:** 범용 · 갱신 가능 · **영지식:** 예 ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** Espresso Systems의 jellyfish에 들어 있는 `jf-plonk`

## 무엇인가

평범한 [PLONK](../plonk/README.ko.md)의 게이트 모양은 하나입니다. 와이어 세 개 위의 `q_L·a + q_R·b + q_O·c + q_M·a·b + q_C = 0`.
**TurboPlonk**는 행을 넓히고(Aztec 구현에서는 와이어 네 개) 게이트가 다음 행도 읽게 해서, 전에는 여러 행이 하던 일을 한
행이 합니다. Aztec이 든 예는 고정 기저 스칼라 곱셈을 두 비트마다 게이트 하나로 하는 것이었습니다.

**plookup**은 다른 종류의 검사를 더합니다. 산술로는 비싸지만 표로는 쉬운 사실이 있습니다. 「이 값은 256보다 작다」, 「이
세 바이트는 XOR 관계다」 같은 것입니다. plookup은 약속된 열의 모든 값이 공개 표에 있다는 것을, 열과 표를 함께 정렬하고
PLONK의 순열 논증과 비슷한 grand-product 논증으로 둘을 비교해서 증명합니다.

**UltraPlonk**는 TurboPlonk에 plookup을 더하고 BN254 위의 KZG로 약속한 것입니다. 증명 크기는 그대로 일정하고, 범용 참조
문자열 하나가 모든 회로에 쓰입니다.

## 역사

- **2020년 2월.** Aztec이 Pedersen 해시에서 Groth16보다 증명자가 다섯 배 빠르다는 TurboPlonk 벤치마크를 올립니다.
- **2020년 3월 15일.** Ariel Gabizon과 Zachary J. Williamson이 plookup을 공개합니다.
- **2020년 5월.** Gabizon이 ZKProof 워크숍에서 TurboPlonk 프로그램 문법을 발표합니다.
- **2020년 9월.** Aztec이 UltraPlonk를 plookup 게이트가 있는 PLONK로 소개합니다.
- **2021년 3월.** Aztec 2.0의 비공개 롤업이 TurboPlonk로 출범하고, Aztec은 UltraPlonk가 배포에 가깝다고 말합니다.
- **2021년 9월.** Aztec이 Aztec 2.0에서 고친 버그를 공개합니다. 회로의 버그 여럿, 그리고 비밀에 암호학용이 아닌 난수
  생성기를 쓴 것을 포함한 증명 시스템 구현 버그 둘입니다. Aztec은 잃은 자금이 없다고 밝힙니다.
- **2023–2025년.** Aztec이 Honk로 옮겨 갑니다. 2025년 3월 Noir가 예제를 UltraHonk로 바꾸고, 2025년 5월 20일 Aztec의
  barretenberg 증명자에서 UltraPlonk가 삭제됩니다.

## 장점

- **게이트가 적습니다.** 사용자 정의 게이트와 조회 덕분에 해시·범위 검사·비트 연산이 평범한 PLONK나 R1CS보다 훨씬
  쌉니다.
- **모든 회로에 설정 한 번**, PLONK와 같습니다. 증명 크기도 일정합니다.
- **필드 밖의 알고리즘도 현실적입니다.** 조회 덕분에 SHA-256이나 AES식 연산을 회로 안에서 감당할 수 있습니다.

## 단점

- **FFT 기반 증명자.** Aztec이 쓴 후속 설계 문서는 sum-check로 옮기면 FFT가 사라져 증명자 시간과 메모리가 줄고, 대신
  증명이 길어진다고 적습니다.
- **범용이라도 신뢰 설정이 필요하고, 양자 내성이 없습니다.** 페어링에 기댑니다.
- **움직이는 부품이 많습니다.** 사용자 정의 게이트와 조회 논증은 평범한 PLONK보다 제대로 만들어야 할 코드가 많고, Aztec의
  2021년 공개가 구현 세부가 얼마나 중요한지 보여 줍니다.

## 얼마나 차세대인가

아이디어는 현역이고, 증명자는 아닙니다. 사용자 정의 게이트와 조회는 요즘 거의 모든 증명 시스템에 들어 있고, Aztec의
UltraHonk도 같은 회로 모양을 씁니다. 바뀐 것은 그 아래의 증명 엔진입니다. 단변수 몫 다항식 대신 sum-check, 그리고
클라이언트 쪽 증명을 위한 폴딩입니다.

## 지금의 상태 (2026년 9월 기준)

**UltraHonk로 대체됨.** 2025년 5월 barretenberg에서 삭제됐고, zkVerify 같은 검증 서비스는 UltraPlonk용으로 옛 barretenberg
릴리스를 안내합니다. 이 전시품이 쓰는 것 같은 독립적인 Rust 구현은 남아 있습니다.

## 이 저장소에서 돌리는 방법

`crates/sys-ultraplonk`는 Espresso Systems의 MIT 라이선스 라이브러리 jellyfish에 들어 있는 UltraPlonk 구현 `jf-plonk`를
BN254 위에서 씁니다. 박물관의 게이트로부터 jellyfish의 회로를 직접 만들어서, 덧셈·곱셈·단언이 각각 jellyfish의 와이어
네 개짜리 게이트 하나가 됩니다.

UltraPlonk를 다르게 만드는 부분도 씁니다. 문장이 어떤 값이 몇 비트 안에 들어가는지 검사하는 곳마다, 박물관의 비트 단위
검사를 8비트 표에 대한 jellyfish의 범위 조회로 바꿉니다. 바꾼 회로가 정확히 같은 주장을 받아들이고 거절하는지는 시험이
확인합니다.

증명은 모두 1,481바이트입니다. 참조 문자열은 문장마다 운영체제의 난수로 만듭니다. 한 사람짜리 설정입니다. 여기서 거짓
주장은 검증자에게 닿지 않습니다. jellyfish의 증명자가 몫 다항식의 차수가 틀렸다는 것을 알아채고 증명을 끝내지 않으며,
박물관은 그것을 증명자의 거부로 보여 줍니다.

`/run ultraplonk pool-spend`를 친 다음 `/run plonk pool-spend`와 비교해 보세요. 조회와 넓은 게이트가 무엇을 아끼는지
보입니다.

## 출처

- A. Gabizon, Z. J. Williamson, *plookup: A simplified polynomial protocol for lookup tables* — https://eprint.iacr.org/2020/315
- A. Gabizon, Z. J. Williamson, *Proposal: The Turbo-PLONK program syntax for specifying SNARK programs*, ZKProof — https://docs.zkproof.org/pages/standards/accepted-workshop3/proposal-turbo_plonk.pdf
- Aztec, TurboPlonk 벤치마크(2020년 2월) — https://x.com/aztecnetwork/status/1230523630745980930
- Aztec, UltraPlonk 소개(2020년 9월) — https://x.com/aztecnetwork/status/1303315879392874496
- Aztec, *Aztec's ZK-ZK-Rollup, looking behind the cryptocurtain* — https://aztec.network/blog/aztecs-zk-zk-rollup-looking-behind-the-cryptocurtain
- Aztec, *Vulnerabilities patched in Aztec 2.0* — https://aztec.network/blog/vulnerabilities-patched-in-aztec-2-0
- Noir 예제를 UltraHonk로 이동 — https://github.com/noir-lang/noir/pull/7653
- barretenberg에서 UltraPlonk 삭제 — https://github.com/AztecProtocol/aztec-packages/pull/14205
- zkVerify, UltraPlonk 검증 팔레트 — https://docs.zkverify.io/architecture/verification_pallets/ultraplonk
- Aztec CHONK 설계 문서 — https://github.com/AztecProtocol/aztec-packages/blob/next/barretenberg/cpp/src/barretenberg/chonk/README.md
- jellyfish — https://github.com/EspressoSystems/jellyfish
