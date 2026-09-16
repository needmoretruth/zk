# Ligero

오류 정정 부호와 해시 함수로 지은 영지식 논증입니다. 증명이 회로 크기의 제곱근쯤이고 신뢰 설정이 없습니다. 오랫동안 논문에만
있다가, 2025년부터 구글의 Longfellow ZK의 핵심으로 구글 월렛에서 디지털 신분증으로 나이를 증명하고 있습니다.

> **선반:** Others · **처음 발표:** 2017 · **신뢰 설정:** 없음 · **영지식:** 예 ·
> **양자 내성:** 그렇게 여겨짐(해시만 씀) · **이 저장소에서 돌리는 코드:** 이 저장소가 쓴 교육용 구현

## 무엇인가

Ligero는 [ZKBoo](../zkboo/README.ko.md)와 같은 **머릿속 MPC**에서 나왔지만, 참여자를 많이 두고 오류 정정 부호를 씁니다. 그래서
증명이 회로와 함께가 아니라 회로의 제곱근과 함께 커집니다.

증명자는 증인을 무작위 채움과 함께 행렬의 행으로 적습니다. 행마다 **Reed–Solomon 부호**로 인코딩하는데, 이 부호는 행을 늘여서
행의 어떤 변화도 인코딩의 대부분을 바꾸게 합니다. 증명자는 인코딩한 행렬의 열에 머클 트리로 약속합니다. 그다음 검증자는 행의
무작위 조합 셋 — 모든 행이 정말 부호어인지 시험하는 것, 일차 제약을 시험하는 것, 곱셈을 시험하는 것 — 과, 무작위로 고른 열 여러 개의
열기를 요구합니다. 속이려면 어느 행의 인코딩을 크게 바꿔야 하므로 연 열에서 높은 확률로 들키고, 무작위 채움 덕분에 열린
열은 증인을 드러내지 않습니다.

## 역사

- **2017년 10월.** Scott Ames, Carmit Hazay, Yuval Ishai, Muthuramakrishnan Venkitasubramaniam이 CCS에서 Ligero를 발표합니다.
- **2023년 7월.** 확장한 학술지 버전이 Designs, Codes and Cryptography에 실립니다.
- **2024년 12월.** 구글의 Matteo Frigo와 abhi shelat가 *Anonymous credentials from ECDSA*를 공개합니다. sumcheck와 Ligero를 합쳐,
  이미 ECDSA로 서명된 신분증에 대한 사실을 증명합니다. 모바일 운전면허증으로 「18세 이상」을 증명하는 데 휴대폰에서 1초가 안
  걸립니다.
- **2025년 4월.** 구글 월렛이 영지식 나이 확인에 쓰기 시작하고, 7월에 라이브러리가 Longfellow ZK로 공개됩니다.
- **2025년 8~12월.** Trail of Bits, ISRG, 학계 패널이 보안 검토를 공개합니다.
- **2026년 7월.** Longfellow ZK를 설명하는 구글의 IETF 초안이 새 버전으로 나옵니다.

## 장점

- **해시 함수 하나뿐.** 신뢰 설정도 곡선도 없습니다.
- **이미 있는 신분증으로 됩니다.** Longfellow는 신분증 발급 방식을 바꾸지 않고 ECDSA 서명과 SHA-256에 대한 사실을 증명합니다.
- **증명자가 빠릅니다.** 휴대폰에서 쓸 만큼입니다.

## 단점

- **증명이 회로의 제곱근과 함께 커집니다.** 실제 신분증에서는 수백 킬로바이트입니다.
- **검증 시간이 간결하지 않습니다.** 검증자의 일이 회로와 함께 늘어납니다.
- **아직 표준이 아닙니다.** IETF 작업은 어떤 프로토콜이 필요로 할 때까지 미뤄져 있습니다.

## 얼마나 차세대인가

오래된 아이디어의 새 삶입니다. Ligero는 이 선반의 대부분의 시스템보다 먼저 나왔지만 증명자가 단순하고 빠르며, sumcheck와 짝을
지어 많은 사람이 알아채지 못한 채 쓰는 영지식 증명이 됐습니다.

## 지금의 상태 (2026년 9월 기준)

**쓰이는 중.** Longfellow ZK는 구글 월렛에서 돌고 Bumble이 씁니다. 2026년 7월에 갱신된 EU 나이 확인 청사진이 이것을 고르고, 유럽의
디지털 신원 지갑들이 통합하고 있습니다.

## 이 저장소에서 돌리는 방법

Longfellow ZK와 libiop의 Ligero는 C++이라서, `crates/sys-ligero`는 논문을 보고 Goldilocks 필드와 SHA-256 위에 쓴 **교육용 구현 ·
감사받지 않음**입니다.

비율 1/4인 부호를 쓰고 열 118개를 엽니다. 라운드마다 오차가 2^-80 아래로 남도록 논문의 한계식에서 뽑은 수입니다. 증명은
one-plus-one의 약 40 KB부터 pool-spend의 약 57 KB까지입니다. 그 사이에 회로는 21배 커지는데 증명은 1.5배도 안 커집니다. 이 크기에서는
열 118개를 여는 고정 비용이 대부분이고, 제곱근으로 커지는 모습은 더 큰 회로에서야 드러나기 때문입니다. 행마다 넣은 무작위 채움이
연 열로 증인이 드러나지 않게 하고, 머클 잎마다 넣은 솔트가 나머지 열을 가립니다. 이것은 2017년 프로토콜이고, Longfellow처럼
sumcheck와 합친 것이 아닙니다.

`/run ligero pool-spend`를 친 다음 `/run zkboo pool-spend`와 비교해 보세요. 같은 아이디어인데, 증명이 회로와 함께가 아니라 회로의
제곱근과 함께 커집니다.

## 출처

- S. Ames, C. Hazay, Y. Ishai, M. Venkitasubramaniam, *Ligero: Lightweight Sublinear Arguments Without a Trusted Setup*, CCS 2017 (확장 버전) — https://eprint.iacr.org/2022/1608
- 학술지 버전, Designs, Codes and Cryptography (2023) — https://doi.org/10.1007/s10623-023-01222-8
- M. Frigo, a. shelat, *Anonymous credentials from ECDSA* — https://eprint.iacr.org/2024/2010
- Longfellow ZK — https://github.com/google/longfellow-zk
- EU 나이 확인, 영지식 부록 — https://ageverification.dev/av-doc-technical-specification/docs/annexes/annex-B/annex-B-zkp/
- IETF 초안, Longfellow ZK — https://datatracker.ietf.org/doc/draft-google-cfrg-libzk/
