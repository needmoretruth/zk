# BCTV14 (Pinocchio)

Zcash가 2016년에 출범하며 쓴 증명 시스템입니다. 저자들이 「거의 실용적(nearly practical)」이라고 부른 페어링 기반 SNARK인
Pinocchio의 변형입니다. Zcash의 원래 Sprout 풀을 2년 동안 지켰고, 그러다 공격자가 없는 돈을 만들 수 있었던 결함이 발견됐습니다 —
그 결함은 발표 없이, 증명 시스템 자체를 내려놓는 방식으로 제거됐습니다.

> **선반:** Zcash · **처음 공개:** 2013–2014 · **신뢰 설정:** 회로마다 · **영지식:** 예 ·
> **양자 내성:** 아니오 · **이 저장소에서 돌리는 코드:** 이 저장소가 쓴 교육용 구현

## 무엇인가

[Groth16](../groth16/README.ko.md)처럼 BCTV14는 R1CS의 증인을 안다는 것을, 제약을 QAP(quadratic arithmetic program)로 바꾸고 페어링으로
「지수 위에서」 다항식 등식을 확인해 증명합니다. 다른 점은 검증자가 따로따로 확신해야 하는 것이 얼마나 많으냐입니다. BCTV14 증명은
그룹 원소 여덟 개를 담습니다. 각 약속이 지어낸 값이 아니라 증명 키로 만든 값임을 보이는 *지식 약속* 세 쌍 (A, A′), (B, B′), (C, C′),
A·B·C가 같은 계수를 썼음을 보이는 원소 K 하나, 제약 다항식이 나누어떨어짐을 보이는 원소 H 하나입니다. 검증자는 이 사실들을 각자의
페어링 등식으로 확인합니다.

회로마다 신뢰 설정이 필요하고, 그 설정의 비밀 — Groth16보다 개수가 많습니다 — 은 반드시 지워야 합니다.

## 역사

- **2012–2013년.** Gennaro, Gentry, Parno, Raykova가 QAP를 내놓습니다(*GGPR*, EUROCRYPT 2013). Parno, Howell, Gentry, Raykova가 그 위에
  *Pinocchio*를 짓고(IEEE S&P 2013) 「거의 실용적」이라고 부릅니다.
- **2013–2014년.** Ben-Sasson, Chiesa, Tromer, Virza가 「Succinct Non-Interactive Zero Knowledge for a von Neumann Architecture」(USENIX
  Security 2014)의 일부로 Pinocchio 변형을 발표하고 C++ 라이브러리 libsnark에 구현합니다. Zcash 명세는 이 변형을 BCTV14라고 부릅니다.
- **2015년.** Bryan Parno가 BCTV14가 Pinocchio를 옮기는 과정의 건전성 문제를 찾고, libsnark에서 고쳐집니다.
- **2016년 10월 22–23일.** 여섯 사람이 다자간 의식으로 Zcash Sprout의 파라미터를 만듭니다. 여섯 명 모두가 부정직했거나 뚫리지 않은 한 안전합니다.
- **2016년 10월 28일.** Zcash가 출범합니다. Sprout의 JoinSplit마다 BN254 곡선 위의 296바이트 BCTV14 증명이 붙습니다.
- **2018년 3월 1일.** Electric Coin Company의 Ariel Gabizon이, BCTV14 논문이 적은 방식의 키 생성 — Zcash의 파라미터 의식이 그대로 따른 방식 — 이 증명자에게
  필요 없는 원소를 더 공개했다는 것을 찾아냅니다(libsnark 자체의 키 생성기는 마침 그 원소를 빼고 있었습니다). 그 원소로
  속이는 증명자가 일관성 검사 하나를 피해 거짓 문장의 증명을 위조할 수 있었고, Zcash에서는 shielded 돈을 없는 데서 만들 수 있었습니다(뒤에
  CVE-2019-7167).
- **2018년 10월 28일.** Sapling 업그레이드가 Sprout을 포함한 모든 shielded 증명을 Groth16으로 옮기면서, 알리지 않고 결함을 없앱니다. Zcash 코드
  위에 지은 다른 체인들에는 11월에 비공개로 알립니다.
- **2019년 2월 5일.** 결함이 공개됩니다. Electric Coin Company는 악용 흔적을 찾지 못했다고 밝혔습니다.

## 장점

- **대규모로, 먼저 돌아갔음.** 대부분의 증명 시스템이 연구실을 나서기 몇 해 전에 일정한 크기의 증명과 빠른 검증으로 실제 가치를 지켰고, libsnark는
  한 세대의 엔지니어를 길러 냈습니다.
- **작고 크기가 일정한 증명.** 회로가 무엇이든 Zcash 인코딩으로 296바이트입니다.
- **빠른 검증.** 정해진 수의 페어링이면 됩니다.

## 단점

- **Groth16보다 크고 느림.** 같은 일에 그룹 원소 여덟 개와 더 많은 페어링 검사가 듭니다.
- **회로마다 의식**, 그리고 지워야 할 독성 폐기물이 더 많습니다.
- **깨지기 쉬운 설정.** 2018년의 결함은 증명 자체가 아니라, 키 생성이 원소 몇 개를 더 공개한 데서 나왔습니다. 올바르게 보이는 증명만 봐서는
  위조를 알아챌 방법이 없었습니다.
- **양자 내성이 없음.**

## 얼마나 차세대인가

지금은 전혀 아닙니다. 구조를 유지하면서 증명을 원소 세 개로 줄인 Groth16의 직계 조상입니다. 남은 교훈은 역사적입니다. SNARK가 실제 돈을 지킬 수
있다는 것, 그리고 신뢰 설정도 증명 시스템만큼 꼼꼼히 감사해야 한다는 것.

## 지금의 상태 (2026년 9월 기준)

2018년 10월 28일부터 Groth16으로 **대체됨**. 아직 이것을 쓰는 운영 시스템은 찾지 못했습니다. 한때 이것이 지키던 Zcash Sprout 풀에는 2026년 8월 말
약 22,600 ZEC가 남아 있었고, Canopy 업그레이드부터 새 입금이 막혀 있습니다. 2026년 9월 14일에 끝난 투표에서 Zcash 보유자들은 Sprout 거래를
끄자는 쪽을 골랐습니다 — 의사 표시이고, 아직 네트워크 규칙은 아닙니다.

## 이 저장소에서 돌리는 방법

허용형 라이선스의 Rust 구현이 없어서(libsnark는 C++입니다) `crates/sys-bctv14`는 **교육용 구현**입니다. 논문과 Zcash 프로토콜 명세를 보고, BN254
곡선을 다루는 arkworks 라이브러리 위에 새로 썼습니다. 감사받지 않았습니다. 증명 키와 검증 키, 원소 여덟 개짜리 증명, 검증자가 돌리는 세 종류의
검사를 만들고, 각 검사에는 그것이 확인하는 사실의 이름을 붙였습니다. 증명은 arkworks의 압축 인코딩으로 288바이트입니다. Zcash는 296바이트인데, libff가 점 여덟 개마다 태그 바이트를 하나씩
따로 쓰기 때문입니다.

**CVE-2019-7167**도 Gabizon의 논문대로 재현합니다. 여분의 원소를 공개하는 키 생성기로 만든 키가 있으면, 누구나 한 공개 입력에 대한 올바른
증명을 다른, 거짓인 공개 입력의 증명으로 바꿀 수 있고, 평범한 검증자가 그것을 받아들입니다. 그 원소가 없는 고친 키로는 같은 변형이
거절됩니다.

`/run bctv14 one-plus-one`을 친 다음 `/run groth16 one-plus-one`과 비교해 보세요. 같은 문장, 같은 종류의 설정, 다른 증명 크기입니다.

이 저장소는 Zcash, Electric Coin Company, Zcash Foundation과 관계가 없습니다.

## 출처

- R. Gennaro, C. Gentry, B. Parno, M. Raykova, *Quadratic Span Programs and Succinct NIZKs without PCPs* — https://eprint.iacr.org/2012/215
- B. Parno, J. Howell, C. Gentry, M. Raykova, *Pinocchio: Nearly Practical Verifiable Computation* — https://eprint.iacr.org/2013/279
- E. Ben-Sasson, A. Chiesa, E. Tromer, M. Virza, *Succinct Non-Interactive Zero Knowledge for a von Neumann Architecture* — https://eprint.iacr.org/2013/879
- Zcash Protocol Specification, §5.4 — https://zips.z.cash/protocol/protocol.pdf
- Zcash Sprout 파라미터 의식 — https://github.com/zcash/mpc
- Zcash counterfeiting vulnerability successfully remediated, Electric Coin Company, 2019-02-05 — https://electriccoin.co/blog/zcash-counterfeiting-vulnerability-successfully-remediated/
- A. Gabizon, *On the security of the BCTV Pinocchio zk-SNARK variant* — https://eprint.iacr.org/2019/119
- ZIP 211, Sprout 가치 풀에 새 가치를 더하는 것을 막기 — https://zips.z.cash/zip-0211
- Zcash 커뮤니티 NU7 투표 — https://zfnd.org/zcap-poll-now-open-nu7/
