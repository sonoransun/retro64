//! Unit tests for the MOS 6510 implementation.

use super::*;
use super::registers::Flags;

/// Tiny RAM-only bus used by unit tests.
struct RamBus {
    ram: [u8; 0x1_0000],
}

impl RamBus {
    fn new() -> Self { RamBus { ram: [0; 0x1_0000] } }
    fn load(&mut self, addr: u16, prog: &[u8]) {
        for (i, b) in prog.iter().enumerate() {
            self.ram[addr as usize + i] = *b;
        }
    }
}

impl Bus for RamBus {
    fn read(&mut self, addr: u16) -> u8 { self.ram[addr as usize] }
    fn write(&mut self, addr: u16, val: u8) { self.ram[addr as usize] = val; }
}

fn setup(prog: &[u8]) -> (Cpu, RamBus) {
    let mut bus = RamBus::new();
    bus.load(0x0200, prog);
    bus.ram[0xFFFC] = 0x00;
    bus.ram[0xFFFD] = 0x02;
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    (cpu, bus)
}

#[test]
fn lda_immediate_sets_flags() {
    let (mut cpu, mut bus) = setup(&[0xA9, 0x00]);
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert!(cpu.flag(Flags::Z));
    assert!(!cpu.flag(Flags::N));
}

#[test]
fn lda_negative() {
    let (mut cpu, mut bus) = setup(&[0xA9, 0x80]);
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert!(cpu.flag(Flags::N));
    assert!(!cpu.flag(Flags::Z));
}

#[test]
fn ldx_ldy_immediate() {
    let (mut cpu, mut bus) = setup(&[0xA2, 0x10, 0xA0, 0x20]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x10);
    assert_eq!(cpu.y, 0x20);
}

#[test]
fn sta_abs() {
    let (mut cpu, mut bus) = setup(&[0xA9, 0x55, 0x8D, 0x00, 0x04]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(bus.ram[0x0400], 0x55);
}

#[test]
fn adc_simple() {
    let (mut cpu, mut bus) = setup(&[0xA9, 0x10, 0x18, 0x69, 0x20]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x30);
    assert!(!cpu.flag(Flags::C));
}

#[test]
fn adc_overflow() {
    // 127 + 1 = -128 -> V set
    let (mut cpu, mut bus) = setup(&[0xA9, 0x7F, 0x18, 0x69, 0x01]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x80);
    assert!(cpu.flag(Flags::V));
}

#[test]
fn sbc_simple() {
    // 0x50 - 0x20 with C=1 (no borrow)
    let (mut cpu, mut bus) = setup(&[0xA9, 0x50, 0x38, 0xE9, 0x20]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x30);
    assert!(cpu.flag(Flags::C));
}

#[test]
fn and_eor_ora() {
    let (mut cpu, mut bus) = setup(&[0xA9, 0xF0, 0x29, 0x0F]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    let (mut cpu, mut bus) = setup(&[0xA9, 0xAA, 0x49, 0x55]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0xFF);
    let (mut cpu, mut bus) = setup(&[0xA9, 0x0F, 0x09, 0xF0]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0xFF);
}

#[test]
fn inx_dex_wrap() {
    let (mut cpu, mut bus) = setup(&[0xA2, 0xFF, 0xE8]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x00);
    assert!(cpu.flag(Flags::Z));
    let (mut cpu, mut bus) = setup(&[0xA2, 0x00, 0xCA]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.x, 0xFF);
    assert!(cpu.flag(Flags::N));
}

#[test]
fn branch_bne_taken() {
    // LDX #$02 ; loop: DEX ; BNE loop ; BRK
    let (mut cpu, mut bus) = setup(&[0xA2, 0x02, 0xCA, 0xD0, 0xFD, 0x00]);
    cpu.step(&mut bus); // LDX
    let start_pc = cpu.pc;
    cpu.step(&mut bus); // DEX
    cpu.step(&mut bus); // BNE, taken
    assert_eq!(cpu.pc, start_pc, "BNE should branch back to DEX");
}

#[test]
fn jsr_rts_roundtrip() {
    // JSR $0300 ; target: RTS. Then BRK.
    let (mut cpu, mut bus) = setup(&[0x20, 0x00, 0x03, 0x00]);
    bus.ram[0x0300] = 0x60; // RTS
    cpu.step(&mut bus); // JSR
    assert_eq!(cpu.pc, 0x0300);
    cpu.step(&mut bus); // RTS
    assert_eq!(cpu.pc, 0x0203);
}

#[test]
fn stack_push_pull() {
    // LDA #$42 ; PHA ; LDA #$00 ; PLA
    let (mut cpu, mut bus) = setup(&[0xA9, 0x42, 0x48, 0xA9, 0x00, 0x68]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x42);
}

#[test]
fn flag_set_clear() {
    let (mut cpu, mut bus) = setup(&[0x38, 0x18, 0xF8, 0xD8]);
    cpu.step(&mut bus); assert!(cpu.flag(Flags::C));
    cpu.step(&mut bus); assert!(!cpu.flag(Flags::C));
    cpu.step(&mut bus); assert!(cpu.flag(Flags::D));
    cpu.step(&mut bus); assert!(!cpu.flag(Flags::D));
}

#[test]
fn cmp_greater_equal() {
    // LDA #$10 ; CMP #$05
    let (mut cpu, mut bus) = setup(&[0xA9, 0x10, 0xC9, 0x05]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert!(cpu.flag(Flags::C));
    assert!(!cpu.flag(Flags::Z));
}

#[test]
fn inc_memory() {
    // INC $0400
    let (mut cpu, mut bus) = setup(&[0xEE, 0x00, 0x04]);
    bus.ram[0x0400] = 0x10;
    cpu.step(&mut bus);
    assert_eq!(bus.ram[0x0400], 0x11);
}

#[test]
fn asl_carry() {
    // LDA #$80 ; ASL A
    let (mut cpu, mut bus) = setup(&[0xA9, 0x80, 0x0A]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert!(cpu.flag(Flags::C));
    assert!(cpu.flag(Flags::Z));
}

#[test]
fn lsr_carry() {
    // LDA #$01 ; LSR A
    let (mut cpu, mut bus) = setup(&[0xA9, 0x01, 0x4A]);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x00);
    assert!(cpu.flag(Flags::C));
}

#[test]
fn rol_ror_rotate() {
    // LDA #$01 ; SEC ; ROL A ; ROR A
    let (mut cpu, mut bus) = setup(&[0xA9, 0x01, 0x38, 0x2A, 0x6A]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x03);
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x01);
}

#[test]
fn bit_sets_nv() {
    // Put $C0 at $0400 ; LDA #$FF ; BIT $0400
    let (mut cpu, mut bus) = setup(&[0xA9, 0xFF, 0x2C, 0x00, 0x04]);
    bus.ram[0x0400] = 0xC0;
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert!(cpu.flag(Flags::N));
    assert!(cpu.flag(Flags::V));
}

#[test]
fn jmp_indirect_page_wrap_bug() {
    // JMP ($03FF) reads target-low from $03FF and high from $0300 (bug)
    let (mut cpu, mut bus) = setup(&[0x6C, 0xFF, 0x03]);
    bus.ram[0x03FF] = 0x34;
    bus.ram[0x0300] = 0x12;
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0x1234);
}

#[test]
fn tax_tay_tsx_txs() {
    // LDA #$11 ; TAX ; TAY -> X=Y=$11
    let (mut cpu, mut bus) = setup(&[0xA9, 0x11, 0xAA, 0xA8]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x11);
    assert_eq!(cpu.y, 0x11);
    // TXS then TSX roundtrip
    let (mut cpu, mut bus) = setup(&[0xA2, 0x80, 0x9A, 0xA2, 0x00, 0xBA]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.x, 0x80);
}

#[test]
fn irq_disabled_when_i_set() {
    let (mut cpu, mut bus) = setup(&[0xEA]);
    cpu.set_flag(Flags::I, true);
    cpu.set_irq(true);
    let cycles = cpu.step(&mut bus);
    assert_eq!(cycles, 2); // NOP, no IRQ service
}

#[test]
fn irq_services_when_i_clear() {
    let (mut cpu, mut bus) = setup(&[0xEA]);
    bus.ram[0xFFFE] = 0x00;
    bus.ram[0xFFFF] = 0x90;
    cpu.set_flag(Flags::I, false);
    cpu.set_irq(true);
    let cycles = cpu.step(&mut bus);
    assert_eq!(cycles, 7);
    assert_eq!(cpu.pc, 0x9000);
}

#[test]
fn nmi_edge_triggered() {
    let (mut cpu, mut bus) = setup(&[0xEA, 0xEA]);
    bus.ram[0xFFFA] = 0x00;
    bus.ram[0xFFFB] = 0x80;
    cpu.trigger_nmi();
    let cycles = cpu.step(&mut bus);
    assert_eq!(cycles, 7);
    assert_eq!(cpu.pc, 0x8000);
    // Second call: NMI already cleared, should not re-fire
    cpu.pc = 0x0200;
    cpu.step(&mut bus);
    assert_ne!(cpu.pc, 0x8000);
}

#[test]
fn brk_rti_roundtrip() {
    let (mut cpu, mut bus) = setup(&[0x00, 0x00, 0xEA]);
    bus.ram[0xFFFE] = 0x00; bus.ram[0xFFFF] = 0x90;
    bus.ram[0x9000] = 0x40; // RTI
    cpu.step(&mut bus); // BRK
    cpu.step(&mut bus); // RTI
    assert_eq!(cpu.pc, 0x0202);
}

#[test]
fn undocumented_lax() {
    let (mut cpu, mut bus) = setup(&[0xA7, 0x10]);
    bus.ram[0x0010] = 0x42;
    cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x42);
    assert_eq!(cpu.x, 0x42);
}

#[test]
fn undocumented_sax() {
    let (mut cpu, mut bus) = setup(&[0xA9, 0xF0, 0xA2, 0x0F, 0x87, 0x20]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(bus.ram[0x0020], 0x00);
}

#[test]
fn undocumented_dcp() {
    // DCP zp: decrement mem, then CMP with A
    let mut bus = RamBus::new();
    bus.load(0x0200, &[0xA9, 0x05, 0xC7, 0x10]);
    bus.ram[0x0010] = 0x06;
    bus.ram[0xFFFC] = 0x00; bus.ram[0xFFFD] = 0x02;
    let mut cpu = Cpu::new(); cpu.reset(&mut bus);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(bus.ram[0x0010], 0x05);
    assert!(cpu.flag(Flags::Z));
}

#[test]
fn bcd_add_basic() {
    // SED ; LDA #$15 ; CLC ; ADC #$27 ; should give $42
    let (mut cpu, mut bus) = setup(&[0xF8, 0xA9, 0x15, 0x18, 0x69, 0x27]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x42);
}

#[test]
fn bcd_sub_basic() {
    // SED ; LDA #$42 ; SEC ; SBC #$15 ; should give $27
    let (mut cpu, mut bus) = setup(&[0xF8, 0xA9, 0x42, 0x38, 0xE9, 0x15]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x27);
}

#[test]
fn zpx_wrap() {
    // STA $zp,X wraps within zero page
    let (mut cpu, mut bus) = setup(&[0xA9, 0xAB, 0xA2, 0x05, 0x95, 0xFE]);
    cpu.step(&mut bus); cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(bus.ram[0x0003], 0xAB); // 0xFE + 5 = 0x103 wrapped to 0x03
}

#[test]
fn absx_page_cross_cycles() {
    // LDA $12F0,X with X=0x20 crosses page (target=$1310)
    let mut bus = RamBus::new();
    bus.load(0x0200, &[0xA2, 0x20, 0xBD, 0xF0, 0x12]);
    bus.ram[0x1310] = 0x99;
    bus.ram[0xFFFC] = 0x00; bus.ram[0xFFFD] = 0x02;
    let mut cpu = Cpu::new(); cpu.reset(&mut bus);
    cpu.step(&mut bus); // LDX
    let cy = cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x99);
    assert_eq!(cy, 5);
}

#[test]
fn indy_read() {
    let mut bus = RamBus::new();
    bus.load(0x0200, &[0xA0, 0x05, 0xB1, 0x10]);
    bus.ram[0x0010] = 0x00; bus.ram[0x0011] = 0x05;
    bus.ram[0x0505] = 0x77;
    bus.ram[0xFFFC] = 0x00; bus.ram[0xFFFD] = 0x02;
    let mut cpu = Cpu::new(); cpu.reset(&mut bus);
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert_eq!(cpu.a, 0x77);
}

#[test]
fn bit_zp_z_flag() {
    let (mut cpu, mut bus) = setup(&[0xA9, 0x0F, 0x24, 0x10]);
    bus.ram[0x0010] = 0xF0;
    cpu.step(&mut bus); cpu.step(&mut bus);
    assert!(cpu.flag(Flags::Z));
    assert!(cpu.flag(Flags::N));
}

#[test]
fn php_plp_roundtrip() {
    let (mut cpu, mut bus) = setup(&[0x38, 0x08, 0x18, 0x28]);
    cpu.step(&mut bus); // SEC
    cpu.step(&mut bus); // PHP
    cpu.step(&mut bus); // CLC
    cpu.step(&mut bus); // PLP
    assert!(cpu.flag(Flags::C));
}

#[test]
fn nop_cycles_exact() {
    let (mut cpu, mut bus) = setup(&[0xEA]);
    assert_eq!(cpu.step(&mut bus), 2);
}

#[test]
fn sed_cld_toggle_decimal() {
    let (mut cpu, mut bus) = setup(&[0xF8, 0xD8]);
    cpu.step(&mut bus); assert!(cpu.flag(Flags::D));
    cpu.step(&mut bus); assert!(!cpu.flag(Flags::D));
}
