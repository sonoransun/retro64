//! Storage-format parser tests.

use retro64_core::storage::{MediaKind, prg, t64::T64, d64::D64, kind_from_name};

#[test]
fn prg_parse_and_inject_basic_program() {
    let prg = [0x01u8, 0x08, 0xAA, 0xBB, 0xCC];
    let p = prg::parse(&prg).unwrap();
    assert_eq!(p.load_addr, 0x0801);
    assert_eq!(p.body, &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn kind_detection_from_filename() {
    assert_eq!(kind_from_name("game.prg"), Some(MediaKind::Prg));
    assert_eq!(kind_from_name("DISK.D64"), Some(MediaKind::D64));
    assert_eq!(kind_from_name("archive.t64"), Some(MediaKind::T64));
    assert_eq!(kind_from_name("tape.tap"), Some(MediaKind::Tap));
    assert_eq!(kind_from_name("cart.crt"), Some(MediaKind::Crt));
    assert_eq!(kind_from_name("unknown.xyz"), None);
}

#[test]
fn d64_rejects_tiny_input() {
    let res = D64::new(&[0u8; 100]);
    assert!(res.is_err());
}

#[test]
fn t64_rejects_bad_magic() {
    let data = [0u8; 64];
    let res = T64::new(&data);
    assert!(res.is_err());
}
