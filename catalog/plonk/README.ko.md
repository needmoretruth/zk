# PLONK

Aztec이 2019년에 발표한 zk-SNARK로, 신뢰 설정 한 번이 모든 회로에 쓰이게 만들었습니다. 회로를 적는 방식 — 셀렉터가
켜고 끄는 와이어의 행들, 그 행들을 잇는 복사 제약 — 은 Halo 2, Kimchi, 그리고 Aztec의 후속 시스템이 함께 쓰는 공용어가
됐습니다.

> **선반:** Aztec · **처음 발표:** 2019 · **신뢰 설정:** 범용 · 갱신 가능 · **영지식:** 예 ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** lambdaworks의 PLONK 증명자

## 무엇인가

PLONK 회로는 표입니다. 행마다 게이트 하나이고, 와이어 `a, b, c` 셋과 그 게이트가 무엇을 하는지 정하는 셀렉터 다섯이
있습니다: `q_L·a + q_R·b + q_O·c + q_M·a·b + q_C = 0`. 덧셈·곱셈·상수·공개 입력이 모두 셀렉터만 다른 같은 식입니다.
복사 제약은 어느 칸들이 같은 값을 갖는지 말하고, PLONK는 순열 논증 하나로 그것을 한꺼번에 증명합니다.

증명자는 열마다 곱셈 부분군 위의 다항식으로 바꾸고 — 이름의 「Lagrange 기저」 — KZG 다항식 약속으로 커밋합니다.
그다음 무작위 점 하나에서 다항식들을 열어, 게이트 식과 순열이 부분군 전체에서 성립한다는 것을 보입니다. 증명은 곡선 점과
필드 원소 몇 개로 크기가 일정하고, 검증자는 회로 크기와 상관없이 페어링 두 번을 합니다.

설정은 **범용**입니다. Groth16은 회로마다 의식을 새로 해야 합니다. PLONK에는 비밀 `τ`의 거듭제곱 목록 하나가 필요하고,
그 목록이 그 크기까지의 모든 회로에 쓰입니다. 또 **갱신 가능**해서 나중에 누구든 무작위성을 더 섞을 수 있고, 기여자 중
한 명만 정직했으면 목록은 안전합니다.

## 역사

- **2018년.** Groth, Kohlweiss, Maller, Meiklejohn, Miers가 SNARK가 범용·갱신 가능한 참조 문자열을 쓸 수 있음을 보입니다.
  Sonic(2019)이 처음으로 실용 가능성이 있는 방식이었지만, 검증이 완전히 간결한 모드에서는 증명이 느렸습니다.
- **2019년 8월 21일.** Aztec의 Ariel Gabizon, Zachary J. Williamson, Oana Ciobotaru가 PLONK를 공개합니다. 계수 대신 부분군
  위의 값으로 다루어 순열 논증과 산술화를 모두 단순하게 만들었고, Sonic보다 군 거듭제곱 연산이 훨씬 적습니다.
- **2019년 10월 25일 – 2020년 1월 2일.** Aztec의 Ignition 의식. 176명이 BN254 위의 KZG 참조 문자열을 만들고, Aztec은 지금도
  그것을 씁니다.
- **2020년.** TurboPlonk가 사용자 정의 게이트를, plookup(Gabizon, Williamson)이 조회 테이블을 더합니다. 둘을 합친 것이
  UltraPlonk입니다.
- **2021년 3월.** Aztec 2.0의 비공개 롤업이 TurboPlonk로 출범합니다.
- **2022–2023년.** PLONK 계열이 퍼집니다. 2022년 5월 Zcash의 Orchard가 Halo 2로 가동되는데, Halo 2는 신뢰 설정 없이
  PLONK 모양의 회로를 씁니다. 2023년 3월에는 Polygon zkEVM 메인넷 베타가 마지막 증명을 Gabizon과 Williamson의 PLONK
  변형인 fflonk로 감쌉니다.
- **2023–2025년.** Aztec이 Honk로 옮겨 갑니다. Honk는 PLONK 모양의 회로는 그대로 두고 단변수 몫 다항식을 sum-check로
  바꿨습니다. 2025년 5월 20일 Aztec의 barretenberg 증명자에서 UltraPlonk가 삭제됩니다.

## 장점

- **모든 회로에 설정 한 번.** 한 번 만든 참조 문자열이 — 원한다면 수백 명이 참여한 의식으로 — 그 크기까지의 모든 회로에
  쓰이고, 나중에 더 튼튼하게 만들 수도 있습니다.
- **유연한 회로.** 새 게이트 종류와 조회 테이블이 같은 표에 들어갑니다. TurboPlonk와 UltraPlonk가 빠른 해시와 범위 검사를
  얻은 방법이 이것입니다.
- **작고 검증이 싼 증명.** 크기가 일정하고 페어링 두 번이면 되며, 회로와 상관이 없습니다.

## 단점

- **여전히 신뢰 설정입니다.** 범용이 없음은 아닙니다. 참조 문자열의 기여자 모두가 비밀을 보관했다면 증명을 위조할 수
  있습니다.
- **Groth16보다 크고 느립니다.** PLONK 증명에는 Groth16의 세 원소보다 몇 배 많은 원소가 들어가고, 같은 R1CS 모양의
  회로라면 증명자의 다항식 연산이 Groth16보다 비쌉니다.
- **양자 내성이 없습니다.** 안전성이 타원곡선 위의 페어링에 기댑니다.

## 얼마나 차세대인가

프로토콜 자체는 더 이상 최전선이 아닙니다. 최전선에 남은 것은 산술화입니다. 요즘 회로 언어는 거의 모두 PLONK 모양의
표를 씁니다. 그 뒤의 증명자들은 다른 곳으로 옮겨 갔습니다 — 불 초입방체 위의 sum-check(HyperPlonk, Aztec의 Honk)로,
신뢰 설정 없는 IPA 약속(Halo 2)으로, 또는 클라이언트 쪽 증명을 위한 폴딩(Aztec의 CHONK)으로.

## 지금의 상태 (2026년 9월 기준)

**계열로는 쓰이는 중, 순수한 PLONK로는 드묾.** 원 논문은 2025년 7월에 마지막으로 고쳐졌습니다. PLONK가 태어난 Aztec은
지금 같은 Ignition 참조 문자열 위에서 UltraHonk와 CHONK로 증명하고, 2025년에 UltraPlonk를 지웠습니다. PLONK 변형은 다른
곳에서 계속 쓰입니다: ZKsync는 fflonk 증명을 검증하고, Mina는 Kimchi로 돌아갑니다.

## 이 저장소에서 돌리는 방법

`crates/sys-plonk`는 LambdaClass의 Apache-2.0 암호 라이브러리 lambdaworks에 들어 있는 PLONK 증명자를 BLS12-381 위에서
씁니다. 박물관의 회로는 이미 PLONK 표라서, 행마다 lambdaworks의 셀렉터 값이 되고 복사 묶음은 순열이 됩니다. 논문과
다른 점이 둘 있습니다.
- lambdaworks는 몫 다항식을 다르게 쪼개고 곡선 점을 압축하지 않고 씁니다. 그래서 증명이 논문의 더 짧은 인코딩이 아니라
  모든 예제에서 1,620바이트입니다.
- 디코더가 서로 다른 바이트열 여러 개를 같은 증명으로 받아들입니다. 박물관은 증명자가 쓴 바로 그 바이트만 받습니다.

설정은 `τ`를 운영체제의 난수에서 뽑아 문장마다 참조 문자열을 새로 만듭니다. 한 사람짜리 의식이라, 믿는 대상은 자기
컴퓨터입니다.

`/run plonk pool-spend`를 친 다음 `/run groth16 pool-spend`와 비교해 보세요. 범용 설정의 값이 보입니다.

## 출처

- A. Gabizon, Z. J. Williamson, O. Ciobotaru, *PLONK: Permutations over Lagrange-bases for Oecumenical Noninteractive arguments of Knowledge* — https://eprint.iacr.org/2019/953
- J. Groth, M. Kohlweiss, M. Maller, S. Meiklejohn, I. Miers, *Updatable and Universal Common Reference Strings with Applications to zk-SNARKs*, CRYPTO 2018 — https://eprint.iacr.org/2018/280
- M. Maller, S. Bowe, M. Kohlweiss, S. Meiklejohn, *Sonic*, CCS 2019 — https://eprint.iacr.org/2019/099
- A. Gabizon, Z. J. Williamson, *plookup* — https://eprint.iacr.org/2020/315
- Aztec Ignition 의식 검증 — https://github.com/AztecProtocol/ignition-verification
- Aztec, *Launching Aztec 2.0 Rollup* — https://aztec.network/blog/launching-aztec-2-0-rollup
- barretenberg에서 UltraPlonk 삭제 — https://github.com/AztecProtocol/aztec-packages/pull/14205
- NU5 활성화(Halo 2 위의 Orchard) — https://electriccoin.co/blog/nu5-activates-on-mainnet-eliminating-trusted-setup-and-launching-a-new-era-for-zcash/
- L2BEAT ZK 카탈로그, Polygon zkEVM 증명자 — https://l2beat.com/zk-catalog/zkprover
- A. Gabizon, Z. J. Williamson, *fflonk* — https://eprint.iacr.org/2021/1167
- ZKsync 프로토콜 v27, Plonk·fflonk 검증자 — https://github.com/zkSync-Community-Hub/zksync-developers/discussions/980
- o1Labs, *Reintroducing Kimchi* — https://www.o1labs.org/blog/reintroducing-kimchi
- lambdaworks — https://github.com/lambdaclass/lambdaworks
