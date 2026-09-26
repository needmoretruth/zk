# nmtzk — a working museum of zero-knowledge proof systems

English · [한국어](README.ko.md)

`nmtzk` is one program with 21 zero-knowledge proof systems built in, from Schnorr's 1989 sigma protocol to
2024's Circle STARKs. Each one has a page telling its history, strengths and weaknesses, and each one runs on your
machine: it proves the same seven statements, the program checks the honest proof, then attacks it three ways and
reports whether the attacks held. The numbers you see — setup, proving and verifying times, proof sizes — are
measured there and then, on your computer.

## The shelves

Every system stands on exactly one shelf, the first that fits:

- **Zcash** — used or built by Zcash.
- **Aztec** — built or used by Aztec.
- **Polygon** — built or used by Polygon.
- **Others** — everything else worth running.
- **Homemade** — designed for this repository: Ali Baba's cave and Trio.

Commands name them `zcash`, `aztec`, `polygon`, `others` and `homemade` (`nmtzk list homemade`).

## The systems

The table is generated from what each system declares about itself in code (`crates/zk-core/src/catalog.rs`);
a test fails when it drifts. Each name links to its page.

<!-- systems:begin -->

| System | ID | Shelf | Year | Trusted setup | Zero-knowledge | Proof size | Post-quantum | Code | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Schnorr and sigma protocols](catalog/schnorr/README.md) | `schnorr` | Zcash | 1989 | none | yes | linear | no | teaching | in use (as of 2026-09) |
| [BCTV14 (Pinocchio)](catalog/bctv14/README.md) | `bctv14` | Zcash | 2013 | per circuit | yes | constant | no | teaching | superseded by Groth16 since 2018-10-28 (as of 2026-09) |
| [Groth16](catalog/groth16/README.md) | `groth16` | Zcash | 2016 | per circuit | yes | constant | no | upstream | in use (as of 2026-09) |
| [Halo 2](catalog/halo2/README.md) | `halo2` | Zcash | 2020 | none | yes | logarithmic | no | upstream | in use (as of 2026-09) |
| [PLONK](catalog/plonk/README.md) | `plonk` | Aztec | 2019 | universal | yes | constant | no | upstream | in use (as of 2026-09) |
| [UltraPlonk](catalog/ultraplonk/README.md) | `ultraplonk` | Aztec | 2020 | universal | yes | constant | no | upstream | superseded by UltraHonk since 2025-05-20 (as of 2026-09) |
| [Winterfell](catalog/winterfell/README.md) | `winterfell` | Polygon | 2021 | none | no | polylogarithmic | yes | upstream | superseded by Plonky3 since 2026-02-14 (as of 2026-09) |
| [Miden VM](catalog/miden/README.md) | `miden` | Polygon | 2021 | none | no | polylogarithmic | yes | upstream | experimental (as of 2026-09) |
| [Plonky3 (uni-STARK)](catalog/plonky3/README.md) | `plonky3` | Polygon | 2024 | none | optional | polylogarithmic | yes | upstream | in use (as of 2026-09) |
| [Circle STARK](catalog/circle-stark/README.md) | `circle-stark` | Polygon | 2024 | none | no | polylogarithmic | yes | upstream | in use (as of 2026-09) |
| [GKR](catalog/gkr/README.md) | `gkr` | Others | 2008 | none | yes | logarithmic | no | upstream | in use (as of 2026-09) |
| [ZKBoo](catalog/zkboo/README.md) | `zkboo` | Others | 2016 | none | yes | linear | yes | teaching | historical (as of 2026-09) |
| [GM17](catalog/gm17/README.md) | `gm17` | Others | 2017 | per circuit | yes | constant | no | upstream | historical (as of 2026-09) |
| [Bulletproofs](catalog/bulletproofs/README.md) | `bulletproofs` | Others | 2017 | none | yes | logarithmic | no | upstream | superseded by Bulletproofs+ since 2022-08-13 (as of 2026-09) |
| [Ligero](catalog/ligero/README.md) | `ligero` | Others | 2017 | none | yes | square root | yes | teaching | in use (as of 2026-09) |
| [STARK](catalog/stark/README.md) | `stark` | Others | 2018 | none | no | polylogarithmic | yes | upstream | superseded by S-two (Circle STARK) since 2025-11-03 (as of 2026-09) |
| [Spartan](catalog/spartan/README.md) | `spartan` | Others | 2019 | none | yes | square root | no | upstream | in use (as of 2026-09) |
| [Marlin](catalog/marlin/README.md) | `marlin` | Others | 2019 | universal | yes | constant | no | upstream | in use (as of 2026-09) |
| [Nova](catalog/nova/README.md) | `nova` | Others | 2021 | none | yes | logarithmic | no | upstream | experimental (as of 2026-09) |
| [Ali Baba's cave](catalog/cave/README.md) | `cave` | Homemade | 1989 | trusted component | yes | linear | no | homemade | historical (as of 2026-09) |
| [Trio](catalog/trio/README.md) | `trio` | Homemade | 2026 | none | yes | linear | yes | homemade | experimental (as of 2026-09) |

<!-- systems:end -->

**Code:** *upstream* runs the crate a real project ships; *teaching* is an implementation written here from the
papers because no permissively licensed Rust one exists; *homemade* is a system designed for this repository.

## The seven examples

Every system proves the same circuits, written once over any field (`crates/zk-examples`):

| ID | Statement |
|---|---|
| `one-plus-one` | The answer sealed in this envelope is 1 + 1. |
| `password` | I know the PIN behind this digest. |
| `sudoku` | I solved this 4×4 sudoku. |
| `age` | I am at least 18. |
| `membership` | I am one of 16 members, without saying which. |
| `factoring` | I know two factors of N, neither of them 1. |
| `pool-spend` | A spend in the Toy Shielded Pool. |

Envelopes and digests use ToyHash, a toy for teaching and not a real hash.

## Install

### Prebuilt

Each [GitHub Release](https://github.com/needmoretruth/zk/releases) has an archive per platform, named
`nmtzk-<version>-<target>.tar.gz`, for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`x86_64-apple-darwin` and `aarch64-apple-darwin`, where `<version>` is the tag without its `v` (`0.1.0` for
`v0.1.0`). Next to each is a `.sha256` file. The archive holds the binary, the licence, both READMEs and completion
scripts for bash, zsh and fish.

```sh
shasum -a 256 -c nmtzk-<version>-<target>.tar.gz.sha256   # or: sha256sum -c …
tar -xzf nmtzk-<version>-<target>.tar.gz
./nmtzk-<version>-<target>/nmtzk --version
```

- **Linux:** the binaries are built on Ubuntu 24.04 and need glibc 2.39 or newer (Ubuntu 24.04, Debian 13, Fedora 40
  or later). On an older system, build from source.
- **macOS:** the binaries are not signed. A browser marks the download, and Gatekeeper then refuses to open it; clear
  the mark with `xattr -d com.apple.quarantine nmtzk-<version>-<target>/nmtzk`, or download with `curl -LO`, which
  leaves no mark.

### From source

The compiler is pinned in `rust-toolchain.toml` (Rust 1.98.1); `rustup` picks it up by itself.

```sh
git clone https://github.com/needmoretruth/zk.git && cd zk
cargo install --locked --path crates/nmtzk   # puts nmtzk on your PATH
# or
cargo build --release -p nmtzk               # target/release/nmtzk
```

### Shell completions

`nmtzk completions <bash|zsh|fish|elvish|powershell>` prints a completion script to standard output. The release
archives carry the first three in `completions/`.

```sh
nmtzk completions bash > ~/.local/share/bash-completion/completions/nmtzk
nmtzk completions zsh > ~/.zfunc/_nmtzk             # with fpath+=(~/.zfunc) before compinit in ~/.zshrc
nmtzk completions fish > ~/.config/fish/completions/nmtzk.fish
```

## Usage

Run `nmtzk` with no command in a terminal for the full-screen program. With a command, it prints the same text the
screen would show and exits.

```text
nmtzk list [SHELF]                          the systems, on every shelf or on one
nmtzk about <SYSTEM>                        what one system declares about itself, then its page
nmtzk examples                              the seven statements
nmtzk run <SYSTEM|all> <EXAMPLE> [--seed HEX] [--no-attacks]
nmtzk cave [MODE] [EXAMPLE] [--scenes N]    Ali Baba's cave, filmed scene by scene
nmtzk trio [play | cheat KIND | simulate] [EXAMPLE] [--rounds N]
nmtzk pool ACTION [ARGUMENTS] [--yes]       the Toy Shielded Pool, proved with Groth16 or Halo 2
nmtzk ceremony [toxic | tau | collude] [--participants N]
nmtzk forge bctv14                          replay CVE-2019-7167
nmtzk completions <SHELL>
```

Global options, before or after the command: `--lang en|ko`, `--ascii` (no box-drawing glyphs), `--json` (JSON
lines instead of text), `--data-dir <DIR>`.

```sh
nmtzk list polygon
nmtzk about halo2
nmtzk run groth16 age
nmtzk run all sudoku --no-attacks
nmtzk --lang ko run plonk membership
nmtzk --json run all one-plus-one > runs.jsonl
nmtzk pool wallet new alice && nmtzk pool faucet alice 10 && nmtzk pool shield alice 5
```

`nmtzk run groth16 age` prints, for example:

```text
• ✓ Groth16 accepted the honest proof of age
  Circuit
  ├ R1CS over BLS12-381
  └ constraints 75 · variables 78
  Timings · measured on this machine, now
  ├ setup   43.7 ms
  ├ prove   21.5 ms
  └ verify  3.39 ms
  Sizes
  ├ setup material  38.1 KiB
  └ proof           192 B
  First bytes · 48 of 192 bytes
  └ b05ad298 fff97c26 ab7d38f5 15f890dc 5eb52a7a fdfb331e 05dc6b69 3137de5e 450d16d2 ca52d475 e9aadf1c 9e6788fe
  Attacks · 3 of 3 held
  ├ ✓ flip proof byte 47: rejected, could not be decoded: bellman Proof::read: invalid G1
  ├ ✓ add 1 to a public input: rejected
  └ ✓ prove a false claim: rejected
  Secret scan
  ├ ✓ no secret appears byte for byte in the proof
  └ A smoke detector, not a proof of zero knowledge.
```

The exit code is 1 when an honest proof was rejected, an attack was accepted, a run failed or the pool's ledger
refused a transaction you asked for, and 2 for a command line that does not parse, so `run all` works in scripts.
`--seed` takes 64 hex digits to make a run's secrets repeatable; without it they come from the OS.

Most systems finish an example in well under a second. GKR is the slow one: its zero-knowledge prover takes about a
minute on `pool-spend` and half a minute on `membership`, so `run all` on those takes a minute or two.

## The full-screen program

A transcript of results above a composer where slash commands are typed. The commands are the ones above plus
`/help`, `/lang <en|ko>`, `/clear` and `/quit`; a command name typed without its slash works too.
`/about <system>` opens the system's page, and ctrl+o opens it again. Esc stops a run, and ctrl+c twice leaves.
Colour follows `NO_COLOR`; `TERM=linux` switches to ASCII as `--ascii` does, and `TERM=dumb` gets plain text and no
full-screen program.

## Data directory

Only the Toy Shielded Pool writes anything: its ledgers and wallets go to `$XDG_DATA_HOME/nmtzk`, or
`~/.local/share/nmtzk`, or `~/Library/Application Support/nmtzk` on macOS. `--data-dir` puts them elsewhere. Any
number of `nmtzk` processes can use one pool: each takes its turn. `nmtzk pool reset --yes` deletes the ledger and
wallets of the system the pool is using, and starts over a pool whose files were damaged.

## Not for real value

This is a museum. The teaching and homemade implementations are written for this repository and have not been
audited, and the upstream crates run here on toy circuits, with any trusted setup a one-person ceremony on your
machine. Nothing in this repository is meant to protect anything of value.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check
```

CI (`.github/workflows/check.yml`) runs the first three on Linux and macOS, and `cargo deny` for licences, sources
and advisories. After a change to a system's metadata, regenerate the tables in both READMEs with
`NMTZK_BLESS=1 cargo test -p nmtzk --test readme`. By default `nmtzk` silences what upstream crates print on
standard output and error; set `NMTZK_UPSTREAM_OUTPUT=1` to see it on standard error.

## License

[Apache-2.0](LICENSE).
