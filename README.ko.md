# nmtzk — 돌아가는 영지식 증명 시스템 박물관

[English](README.md) · 한국어

`nmtzk`는 영지식 증명 시스템 21개가 들어 있는 프로그램 하나입니다. 1989년 Schnorr의 시그마 프로토콜부터 2024년의
Circle STARK까지 있습니다. 시스템마다 역사와 장점, 단점을 적은 페이지가 있고, 모두 여러분의 컴퓨터에서 돌아갑니다.
같은 일곱 문장을 증명하고, 정직한 증명을 검증한 다음, 그 증명을 세 가지 방법으로 공격해 공격이 막혔는지 알려 줍니다.
화면에 나오는 숫자 — 설정, 증명, 검증 시간과 증명 크기 — 는 그 자리에서 여러분의 컴퓨터로 잰 값입니다.

## 선반

시스템은 모두 선반 하나에 놓입니다. 아래 순서대로 처음 맞는 선반입니다.

- **Zcash** — Zcash가 쓰거나 만든 것.
- **Aztec** — Aztec이 만들거나 쓰는 것.
- **Polygon** — Polygon이 만들거나 쓰는 것.
- **기타** — 그 밖에 돌려 볼 만한 모든 것.
- **자작** — 이 저장소를 위해 설계한 것: 알리바바의 동굴과 Trio.

명령에서는 선반을 `zcash`, `aztec`, `polygon`, `others`, `homemade`로 적습니다(`nmtzk list homemade`).

## 시스템

이 표는 각 시스템이 코드(`crates/zk-core/src/catalog.rs`)에서 스스로 밝힌 값으로 만듭니다. 표가 그 값과 어긋나면
테스트가 실패합니다. 이름을 누르면 그 시스템의 페이지로 갑니다.

<!-- systems:begin -->

| 시스템 | ID | 선반 | 연도 | 신뢰 설정 | 영지식 | 증명 크기 | 양자 내성 | 코드 | 상태 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Schnorr and sigma protocols](catalog/schnorr/README.ko.md) | `schnorr` | Zcash | 1989 | 없음 | 예 | 선형 | 아니오 | 교육용 | 쓰이는 중 (2026-09 기준) |
| [BCTV14 (Pinocchio)](catalog/bctv14/README.ko.md) | `bctv14` | Zcash | 2013 | 회로마다 | 예 | 일정 | 아니오 | 교육용 | 대체됨 · Groth16 · 2018-10-28부터 (2026-09 기준) |
| [Groth16](catalog/groth16/README.ko.md) | `groth16` | Zcash | 2016 | 회로마다 | 예 | 일정 | 아니오 | 원조 | 쓰이는 중 (2026-09 기준) |
| [Halo 2](catalog/halo2/README.ko.md) | `halo2` | Zcash | 2020 | 없음 | 예 | 로그 | 아니오 | 원조 | 쓰이는 중 (2026-09 기준) |
| [PLONK](catalog/plonk/README.ko.md) | `plonk` | Aztec | 2019 | 범용 | 예 | 일정 | 아니오 | 원조 | 쓰이는 중 (2026-09 기준) |
| [UltraPlonk](catalog/ultraplonk/README.ko.md) | `ultraplonk` | Aztec | 2020 | 범용 | 예 | 일정 | 아니오 | 원조 | 대체됨 · UltraHonk · 2025-05-20부터 (2026-09 기준) |
| [Winterfell](catalog/winterfell/README.ko.md) | `winterfell` | Polygon | 2021 | 없음 | 아니오 | 다항 로그 | 예 | 원조 | 대체됨 · Plonky3 · 2026-02-14부터 (2026-09 기준) |
| [Miden VM](catalog/miden/README.ko.md) | `miden` | Polygon | 2021 | 없음 | 아니오 | 다항 로그 | 예 | 원조 | 실험 중 (2026-09 기준) |
| [Plonky3 (uni-STARK)](catalog/plonky3/README.ko.md) | `plonky3` | Polygon | 2024 | 없음 | 선택 | 다항 로그 | 예 | 원조 | 쓰이는 중 (2026-09 기준) |
| [Circle STARK](catalog/circle-stark/README.ko.md) | `circle-stark` | Polygon | 2024 | 없음 | 아니오 | 다항 로그 | 예 | 원조 | 쓰이는 중 (2026-09 기준) |
| [GKR](catalog/gkr/README.ko.md) | `gkr` | 기타 | 2008 | 없음 | 예 | 로그 | 아니오 | 원조 | 쓰이는 중 (2026-09 기준) |
| [ZKBoo](catalog/zkboo/README.ko.md) | `zkboo` | 기타 | 2016 | 없음 | 예 | 선형 | 예 | 교육용 | 역사 (2026-09 기준) |
| [GM17](catalog/gm17/README.ko.md) | `gm17` | 기타 | 2017 | 회로마다 | 예 | 일정 | 아니오 | 원조 | 역사 (2026-09 기준) |
| [Bulletproofs](catalog/bulletproofs/README.ko.md) | `bulletproofs` | 기타 | 2017 | 없음 | 예 | 로그 | 아니오 | 원조 | 대체됨 · Bulletproofs+ · 2022-08-13부터 (2026-09 기준) |
| [Ligero](catalog/ligero/README.ko.md) | `ligero` | 기타 | 2017 | 없음 | 예 | 제곱근 | 예 | 교육용 | 쓰이는 중 (2026-09 기준) |
| [STARK](catalog/stark/README.ko.md) | `stark` | 기타 | 2018 | 없음 | 아니오 | 다항 로그 | 예 | 원조 | 대체됨 · S-two (Circle STARK) · 2025-11-03부터 (2026-09 기준) |
| [Spartan](catalog/spartan/README.ko.md) | `spartan` | 기타 | 2019 | 없음 | 예 | 제곱근 | 아니오 | 원조 | 쓰이는 중 (2026-09 기준) |
| [Marlin](catalog/marlin/README.ko.md) | `marlin` | 기타 | 2019 | 범용 | 예 | 일정 | 아니오 | 원조 | 쓰이는 중 (2026-09 기준) |
| [Nova](catalog/nova/README.ko.md) | `nova` | 기타 | 2021 | 없음 | 예 | 로그 | 아니오 | 원조 | 실험 중 (2026-09 기준) |
| [Ali Baba's cave](catalog/cave/README.ko.md) | `cave` | 자작 | 1989 | 믿어야 하는 부품 | 예 | 선형 | 아니오 | 자작 | 역사 (2026-09 기준) |
| [Trio](catalog/trio/README.ko.md) | `trio` | 자작 | 2026 | 없음 | 예 | 선형 | 예 | 자작 | 실험 중 (2026-09 기준) |

<!-- systems:end -->

**코드:** *원조*는 실제 프로젝트가 내놓은 크레이트를 그대로 돌립니다. *교육용*은 허용 라이선스 Rust 구현이 없어 이
저장소가 논문을 보고 쓴 구현입니다. *자작*은 이 저장소를 위해 설계한 시스템입니다.

## 일곱 가지 예제

모든 시스템이 같은 회로를 증명합니다. 회로는 어느 필드에서든 돌도록 한 번만 적었습니다(`crates/zk-examples`).

| ID | 문장 |
|---|---|
| `one-plus-one` | 이 봉투에 봉인한 답은 1 + 1입니다. |
| `password` | 나는 이 요약값 뒤의 PIN을 압니다. |
| `sudoku` | 나는 이 4×4 스도쿠를 풀었습니다. |
| `age` | 나는 18세 이상입니다. |
| `membership` | 나는 회원 16명 중 한 명입니다. 누구인지는 말하지 않습니다. |
| `factoring` | 나는 N의 두 인수를 압니다. 둘 다 1이 아닙니다. |
| `pool-spend` | 장난감 비공개 송금의 지출 하나. |

봉투와 요약값은 ToyHash를 씁니다. 가르치기 위한 장난감이고 진짜 해시가 아닙니다.

## 설치

### 미리 빌드된 파일

[GitHub Releases](https://github.com/needmoretruth/zk/releases)마다 플랫폼별 압축 파일이
`nmtzk-<version>-<target>.tar.gz`라는 이름으로 올라옵니다. 대상은 `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`이고, 파일마다 `.sha256` 파일이 함께
있습니다. 압축 파일에는 실행 파일, 라이선스, 두 README, bash·zsh·fish용 자동 완성 스크립트가 들어 있습니다.

```sh
shasum -a 256 -c nmtzk-<version>-<target>.tar.gz.sha256
tar -xzf nmtzk-<version>-<target>.tar.gz
./nmtzk-<version>-<target>/nmtzk --version
```

### 소스에서

컴파일러는 `rust-toolchain.toml`에 고정되어 있습니다(Rust 1.98.1). `rustup`이 알아서 그 버전을 씁니다.

```sh
git clone https://github.com/needmoretruth/zk.git && cd zk
cargo install --locked --path crates/nmtzk   # PATH에 nmtzk를 설치합니다
# 또는
cargo build --release -p nmtzk               # target/release/nmtzk
```

### 셸 자동 완성

`nmtzk completions <bash|zsh|fish|elvish|powershell>`은 자동 완성 스크립트를 표준 출력으로 내보냅니다.

```sh
nmtzk completions bash > ~/.local/share/bash-completion/completions/nmtzk
```

## 쓰는 법

터미널에서 명령 없이 `nmtzk`를 실행하면 전체 화면 프로그램이 열립니다. 명령을 주면 화면에 나올 내용을 그대로 글로
출력하고 끝납니다.

```text
nmtzk list [SHELF]                          시스템 목록, 모든 선반 또는 선반 하나
nmtzk about <SYSTEM>                        시스템이 스스로 밝힌 내용과 그 페이지
nmtzk examples                              일곱 문장
nmtzk run <SYSTEM|all> <EXAMPLE> [--seed HEX] [--no-attacks]
nmtzk cave [MODE] [EXAMPLE] [--scenes N]    알리바바의 동굴, 장면마다 촬영
nmtzk trio [play | cheat KIND | simulate] [EXAMPLE] [--rounds N]
nmtzk pool ACTION [ARGUMENTS] [--yes]       장난감 비공개 송금, Groth16 또는 Halo 2로 증명
nmtzk ceremony [toxic | tau | collude] [--participants N]
nmtzk forge bctv14                          CVE-2019-7167 재현
nmtzk completions <SHELL>
```

전역 옵션은 명령 앞이나 뒤 어디에나 둘 수 있습니다. `--lang en|ko`, `--ascii`(상자 그리기 문자를 쓰지 않음),
`--json`(글 대신 JSON 줄), `--data-dir <DIR>`입니다.

```sh
nmtzk --lang ko list polygon
nmtzk --lang ko about halo2
nmtzk --lang ko run groth16 age
nmtzk run all sudoku --no-attacks
nmtzk --json run all one-plus-one > runs.jsonl
nmtzk pool wallet new alice && nmtzk pool faucet alice 10 && nmtzk pool shield alice 5
```

정직한 증명이 거부되거나, 공격이 받아들여지거나, 실행이 실패하면 종료 코드가 0이 아니므로 `run all`을 스크립트에서
쓸 수 있습니다. `--seed`에 16진수 64자리를 주면 실행의 비밀값을 똑같이 되풀이할 수 있습니다. 주지 않으면 운영체제에서
뽑습니다.

## 전체 화면 프로그램

위에는 결과 기록이, 아래에는 슬래시 명령을 치는 입력창이 있습니다. 명령은 위의 것들에 `/help`, `/lang <en|ko>`,
`/clear`, `/quit`이 더해집니다. 슬래시 없이 명령 이름만 쳐도 됩니다. `/about <system>`은 그 시스템의 페이지를 열고,
ctrl+o로 다시 엽니다. `NO_COLOR`를 설정하면 색을 쓰지 않고, `TERM=linux`에서는 `--ascii`처럼 ASCII로 그립니다.

## 데이터 디렉터리

무언가를 저장하는 것은 장난감 비공개 송금뿐입니다. 원장과 지갑은 `$XDG_DATA_HOME/nmtzk`나
`~/.local/share/nmtzk`에, macOS에서는 `~/Library/Application Support/nmtzk`에 둡니다. `--data-dir`로 다른 곳을
지정할 수 있고, `nmtzk pool reset`은 지금 풀이 쓰는 시스템의 원장과 지갑을 지웁니다.

## 실제 가치를 지키는 데 쓰지 마세요

이곳은 박물관입니다. 교육용 구현과 자작 구현은 이 저장소가 직접 썼고 감사받지 않았습니다. 원조 크레이트도 여기서는
장난감 회로를 증명하고, 신뢰 설정이 있다면 여러분의 컴퓨터에서 도는 한 사람짜리 의식입니다. 이 저장소의 어떤 것도 가치 있는 무언가를
지키려고 만든 것이 아닙니다.

## 개발

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check
```

CI(`.github/workflows/check.yml`)는 앞의 셋을 Linux와 macOS에서 돌리고, 라이선스·출처·보안 권고는 `cargo deny`로
확인합니다. 시스템의 메타데이터를 바꿨다면 `NMTZK_BLESS=1 cargo test -p nmtzk --test readme`로 두 README의 표를 다시
만드세요. `nmtzk`는 원조 크레이트가 표준 출력과 표준 오류에 찍는 내용을 기본으로 감춥니다. 보고 싶다면
`NMTZK_UPSTREAM_OUTPUT=1`을 설정하세요.

## 라이선스

[Apache-2.0](LICENSE).
