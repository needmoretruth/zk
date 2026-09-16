//! The tape's byte format.

use zk_cave::{Side, Tape, TapeScene};

fn two_scenes() -> Tape {
    Tape {
        scenes: vec![
            TapeScene { call: Side::Right, exit: Side::Right },
            TapeScene { call: Side::Left, exit: Side::Right },
        ],
    }
}

#[test]
fn a_tape_is_magic_version_count_and_one_letter_per_side() {
    let bytes = two_scenes().to_bytes();
    assert_eq!(bytes, b"CAVE\x01\x02\x00\x00\x00RRLR");
    assert_eq!(Tape::from_bytes(&bytes), Some(two_scenes()));
}

#[test]
fn anything_but_a_whole_tape_is_refused() {
    let bytes = two_scenes().to_bytes();
    let mut wrong_magic = bytes.clone();
    wrong_magic[0] = b'X';
    let mut wrong_letter = bytes.clone();
    wrong_letter[9] = 0;
    assert_eq!(Tape::from_bytes(&wrong_magic), None);
    assert_eq!(Tape::from_bytes(&wrong_letter), None);
    assert_eq!(Tape::from_bytes(&bytes[..bytes.len() - 1]), None);
    assert_eq!(Tape::from_bytes(&bytes[..5]), None);
}
