//! Addressing-mode fetchers. Each returns the effective address (and where
//! applicable a page-cross indicator used for conditional cycle penalties).

use super::{Bus, Cpu};

#[inline]
pub fn addr_zp<B: Bus>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    cpu.fetch(bus) as u16
}

#[inline]
pub fn addr_zpx<B: Bus>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    cpu.fetch(bus).wrapping_add(cpu.x) as u16
}

#[inline]
pub fn addr_zpy<B: Bus>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    cpu.fetch(bus).wrapping_add(cpu.y) as u16
}

#[inline]
pub fn addr_abs<B: Bus>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    cpu.fetch16(bus)
}

#[inline]
pub fn addr_absx<B: Bus>(cpu: &mut Cpu, bus: &mut B) -> (u16, u8) {
    let base = cpu.fetch16(bus);
    let ea = base.wrapping_add(cpu.x as u16);
    let cross = (base & 0xFF00) != (ea & 0xFF00);
    (ea, if cross { 1 } else { 0 })
}

#[inline]
pub fn addr_absy<B: Bus>(cpu: &mut Cpu, bus: &mut B) -> (u16, u8) {
    let base = cpu.fetch16(bus);
    let ea = base.wrapping_add(cpu.y as u16);
    let cross = (base & 0xFF00) != (ea & 0xFF00);
    (ea, if cross { 1 } else { 0 })
}

#[inline]
pub fn addr_indx<B: Bus>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let zp = cpu.fetch(bus).wrapping_add(cpu.x);
    let lo = bus.read(zp as u16) as u16;
    let hi = bus.read(zp.wrapping_add(1) as u16) as u16;
    (hi << 8) | lo
}

#[inline]
pub fn addr_indy<B: Bus>(cpu: &mut Cpu, bus: &mut B) -> (u16, u8) {
    let zp = cpu.fetch(bus);
    let lo = bus.read(zp as u16) as u16;
    let hi = bus.read(zp.wrapping_add(1) as u16) as u16;
    let base = (hi << 8) | lo;
    let ea = base.wrapping_add(cpu.y as u16);
    let cross = (base & 0xFF00) != (ea & 0xFF00);
    (ea, if cross { 1 } else { 0 })
}
