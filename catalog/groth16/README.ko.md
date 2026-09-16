# Groth16

증명 하나가 타원곡선 점 세 개로 끝나는 페어링 기반 zk-SNARK입니다. Zcash Sapling 풀이 쓰는 증명 시스템이고,
발표된 지 10년이 지난 지금도 널리 쓰이는 증명 가운데 가장 작고 가장 빨리 검증됩니다.

> **선반:** Zcash · **처음 공개:** 2016 · **신뢰 설정:** 회로마다 · **영지식:** 예 ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** Zcash가 직접 쓰는 `bellman`

## 무엇인가

산술 회로의 증인(witness)을 아는 증명자는 타원곡선 점 정확히 세 개로 검증자를 설득합니다. 두 개는 G1 그룹, 하나는
G2 그룹의 점이고, BLS12-381 곡선에서는 192바이트입니다. 검증자는 페어링 사이의 등식 하나만 확인하므로, 회로의
제약이 열 개든 천만 개든 검증 시간이 같습니다.

회로는 R1CS(rank-1 constraint system)로 적고, 이것을 다항식(QAP)으로 바꿉니다. 누구든 증명하기 전에 **그 회로만을 위한
설정(setup)**이 비밀 난수로 증명 키와 검증 키를 만듭니다. 이 난수(흔히 「독성 폐기물」이라 부릅니다)는 반드시
지워야 합니다. 그것을 가진 사람은 거짓 문장의 증명을 위조할 수 있기 때문입니다. 어느 경우든 증명 자체는 증인에 대해
아무것도 드러내지 않습니다.

## 역사

- **2016년.** Jens Groth가 「On the Size of Pairing-based Non-interactive Arguments」(EUROCRYPT 2016)를 발표합니다.
  당시의 페어링 기반 SNARK — Pinocchio와 그 변형인 BCTV14는 그룹 원소 여덟 개를 썼습니다 — 를 세 개로 줄였습니다.
- **2017–2018년.** 독성 폐기물을 한 사람에게 맡기지 않으려고 Zcash 공동체가 두 단계의 다자간 의식을 엽니다. 첫 단계
  *Powers of Tau*는 2017년 11월부터 2018년 4월까지 87번의 기여를 모았고, 둘째 단계가 Sapling 회로의 키를 만들었습니다.
  참가자 중 한 명만 자기 비밀을 지웠어도 키는 안전합니다.
- **2018년 10월 28일.** Zcash가 블록 419,200에서 Sapling을 켭니다. shielded 송금의 spend·output 증명이 BLS12-381 위의 Groth16으로,
  Rust 크레이트 `bellman`으로 증명됩니다. 같은 순간 Sprout의 옛 JoinSplit 증명도 Groth16으로 옮겨 갔고, 그 덕에 이전 증명
  시스템의 위조 결함도 함께 닫혔습니다([BCTV14](../bctv14/README.ko.md) 참고).
- **그 뒤로** 새 시스템들의 「마지막 단계」가 됐습니다. 예를 들어 RISC Zero는 STARK 증명을 BN254 위의 Groth16 증명으로
  한 번 더 감싸, 블록체인이 점 세 개만 확인하면 되게 합니다.

## 장점

- **가장 작은 증명.** 회로가 무엇이든 그룹 원소 세 개입니다.
- **가장 빠른 검증.** 페어링 몇 번과 공개 입력에 대한 짧은 합이면 끝납니다.
- **성숙함.** 실제 가치를 지키며 여러 해 동안 운영됐고, 독립 구현이 여럿 있고, 보안 증명(일반 그룹 모델)이 잘 연구됐습니다.
- **좋은 래퍼(wrapper).** 증명이 큰 시스템도 「그 증명을 확인했다」를 Groth16 회로 안에서 증명하고 작은 증명을 대신 내보낼 수 있습니다.

## 단점

- **회로마다 새 의식.** 제약 하나만 바꿔도 키를 못 씁니다. Groth16 시스템을 업그레이드하려면 신뢰 설정을 다시 열어야 합니다.
- **독성 폐기물.** 의식의 모든 참가자가 비밀을 남겼거나 한 사람짜리 설정이 뚫렸다면 거짓 증명이 가능해지고, 증명만 봐서는
  아무도 알 수 없습니다.
- **양자 내성이 없음.** 페어링은 큰 양자 컴퓨터가 깨뜨릴 이산 로그 계열 가정에 기댑니다.
- **무거운 증명자.** 증명 시간은 큰 다중 스칼라 곱셈이 차지하고, 큰 회로는 메모리를 많이 씁니다.
- **증명의 가변성(malleability).** 증인 없이도 누구나 올바른 증명을 모양만 다른 올바른 증명으로 다시 무작위화할 수 있습니다. 증명이
  하나뿐이어야 하는 시스템은 증명을 다른 무언가에 묶어야 합니다.

## 얼마나 차세대인가

설계로 보면 차세대가 아닙니다. 회로를 키 안에 고정한 2016년의 구성입니다. 새 시스템들은 회로별 설정을 없애거나(범용 설정의
PLONK, 설정이 없는 Halo 2와 STARK) 양자 내성을 노립니다(해시 기반·격자 기반 증명). 그래도 Groth16의 작은 증명은 많은
차세대 파이프라인의 끝에서, 블록체인이 실제로 검증하는 형식으로 남아 있습니다. 앞날은 양자 내성 때문에 페어링을 얼마나
빨리 내려놓아야 하느냐에 달려 있습니다.

## 지금의 상태 (2026년 9월 기준)

**쓰이는 중.** Zcash Sapling 풀에는 2026년 8월 말 약 525,000 ZEC가 있었고, Sapling을 폐지하는 Zcash 개선 제안(ZIP)은 찾지
못했습니다. Zcash의 가장 새 풀(Orchard, 2026년 7월부터는 Ironwood)은 Halo 2를 씁니다. Zcash 밖에서는 RISC Zero 증명의
마지막 래퍼이자 iden3 신원 프로토콜의 증명 시스템입니다.

## 이 저장소에서 돌리는 방법

`crates/sys-groth16`은 Zcash의 Sapling 증명자가 쓰는 `bellman`과 `bls12_381` 크레이트를 씁니다. 예제 회로를 R1CS로 내리고
bellman의 제약 API로 합성합니다. 설정은 여러분의 컴퓨터에서 새 비밀 난수로 돌고, 끝나면 그 난수를 버립니다. 한 사람짜리
의식이므로 Zcash의 참가자들이 아니라 여러분의 컴퓨터를 믿는 것입니다.

`/run groth16 one-plus-one`을 친 다음, `/run all one-plus-one`으로 나머지 전부와 비교해 보세요.

이 저장소는 Zcash, Electric Coin Company, Zcash Foundation과 관계가 없습니다.

## 출처

- J. Groth, *On the Size of Pairing-based Non-interactive Arguments*, EUROCRYPT 2016 — https://eprint.iacr.org/2016/260
- Zcash Protocol Specification, §5.4 — https://zips.z.cash/protocol/protocol.pdf
- Powers of Tau 의식 종료 발표 — https://zfnd.org/conclusion-of-the-powers-of-tau-ceremony/
- ZIP 205, Sapling 네트워크 업그레이드 배포 — https://zips.z.cash/zip-0205
- Pine Analytics, Zcash 분기 보고서 2026년 3분기(풀 규모) — https://pineanalytics.substack.com/p/zcash-quarterly-report-q3-2026
- L2BEAT ZK 카탈로그, RISC Zero — https://l2beat.com/zk-catalog/risc0
- bellman — https://github.com/zkcrypto/bellman
