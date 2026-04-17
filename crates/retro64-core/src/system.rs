//! The [`C64`] orchestrator — wires CPU + memory + chips + extensions and
//! exposes the public API used by the desktop and web frontends.

use crate::cia::{Cia, CiaIndex, JoystickState, KeyboardMatrix};
pub use crate::cia::C64Key;
use crate::config::{Config, JoystickPort};
use crate::cpu::{Bus, Cpu};
use crate::extensions::Extensions;
use crate::memory::{Memory, RomSet};
use crate::sid::Sid;
use crate::storage::{d64::D64, prg, t64::T64, MediaKind, StorageError};
use crate::vic::VicII;

/// Full Commodore 64.
pub struct C64 {
    cpu: Cpu,
    mem: Memory,
    vic: VicII,
    sid: Sid,
    cia1: Cia,
    cia2: Cia,
    kb: KeyboardMatrix,
    joy1: JoystickState,
    joy2: JoystickState,
    ext: Extensions,

    /// Which host joystick is wired to port 1 vs 2.
    joy_port: JoystickPort,

    /// Disk image currently mounted for device 8 (owned).
    disk: Option<Vec<u8>>,

    model: crate::config::Model,
    frame_complete: bool,
    warp: bool,

    /// Last framebuffer slice returned to the caller.
    fb_snapshot: Vec<u32>,
}

impl C64 {
    /// Build a new C64 from a [`Config`].
    pub fn new(cfg: Config) -> Self {
        let mut mem = Memory::new();
        if let Some(dir) = cfg.rom_dir.as_ref() {
            if let Ok(roms) = RomSet::from_dir(dir) {
                mem.install_roms(&roms);
            }
        }

        let vic = VicII::new(cfg.model);
        let fb_snapshot = vec![0u32; (vic.width * vic.height) as usize];

        let mut c64 = C64 {
            cpu: Cpu::new(),
            mem,
            vic,
            sid: Sid::new(cfg.model, cfg.sample_rate),
            cia1: Cia::new(CiaIndex::Cia1),
            cia2: Cia::new(CiaIndex::Cia2),
            kb: KeyboardMatrix::new(),
            joy1: JoystickState::default(),
            joy2: JoystickState::default(),
            ext: Extensions::new(cfg.extensions_enabled),
            joy_port: cfg.joystick_port,
            disk: None,
            model: cfg.model,
            frame_complete: false,
            warp: cfg.warp,
            fb_snapshot,
        };
        c64.reset();
        c64
    }

    /// Hardware reset.
    pub fn reset(&mut self) {
        let mut bus = SystemBus {
            mem: &mut self.mem, vic: &mut self.vic, sid: &mut self.sid,
            cia1: &mut self.cia1, cia2: &mut self.cia2,
            kb: &self.kb, joy1: &self.joy1, joy2: &self.joy2,
            ext: &mut self.ext,
        };
        self.cpu.reset(&mut bus);
    }

    /// Width of the framebuffer returned by [`run_frame`].
    pub fn screen_width(&self) -> u32 { self.vic.width }
    /// Height of the framebuffer returned by [`run_frame`].
    pub fn screen_height(&self) -> u32 { self.vic.height }
    /// Target refresh rate in Hz.
    pub fn target_fps(&self) -> f32 { self.model.fps() }
    /// Warp-speed flag (frontend hint).
    pub fn warp(&self) -> bool { self.warp }
    /// Update the warp flag.
    pub fn set_warp(&mut self, w: bool) { self.warp = w; }

    /// Read-only access to the current framebuffer.
    pub fn framebuffer(&self) -> &[u32] { &self.fb_snapshot }

    /// Press a key.
    pub fn key_down(&mut self, k: C64Key) { self.kb.press(k); }
    /// Release a key.
    pub fn key_up(&mut self, k: C64Key) { self.kb.release(k); }
    /// Apply raw joystick direction/fire bits (bit 0=up, 1=down, 2=left, 3=right, 4=fire).
    pub fn joystick(&mut self, port: JoystickPort, bits: u8) {
        match port {
            JoystickPort::Port1 => self.joy1.bits = bits & 0x1F,
            JoystickPort::Port2 => self.joy2.bits = bits & 0x1F,
        }
    }
    /// Latch an NMI (the RESTORE key and CIA2 source both do this).
    pub fn trigger_nmi(&mut self) { self.cpu.trigger_nmi(); }

    /// Load a PRG file, injecting its body into RAM and queuing an
    /// autostart `RUN` command.
    pub fn load_prg(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        let p = prg::parse(bytes)?;
        prg::inject(&mut self.mem.ram, &p);
        if p.load_addr == 0x0801 {
            prg::autostart(&mut self.mem.ram);
        }
        Ok(())
    }

    /// Load any supported media, dispatching by kind.
    pub fn load_media(&mut self, bytes: &[u8], kind: MediaKind) -> Result<(), StorageError> {
        match kind {
            MediaKind::Prg => self.load_prg(bytes),
            MediaKind::D64 => self.insert_disk(bytes),
            MediaKind::T64 => {
                let t = T64::new(bytes)?;
                let blob = t.extract(0).ok_or(StorageError::TooShort)?;
                self.load_prg(&blob)
            }
            MediaKind::Tap => Err(StorageError::Unsupported("TAP tape images")),
            MediaKind::Crt => Err(StorageError::Unsupported("CRT cartridges")),
        }
    }

    /// Mount a D64 disk image (replaces the currently-mounted disk).
    pub fn insert_disk(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        let _ = D64::new(bytes)?; // validate
        self.disk = Some(bytes.to_vec());
        Ok(())
    }

    /// List files on the mounted disk (empty if none).
    pub fn disk_directory(&self) -> Vec<String> {
        let Some(bytes) = self.disk.as_ref() else { return Vec::new(); };
        let Ok(d) = D64::new(bytes) else { return Vec::new(); };
        d.directory().iter()
            .map(|e| {
                let trim = e.name.iter().position(|b| *b == 0xA0).unwrap_or(16);
                String::from_utf8_lossy(&e.name[..trim]).to_string()
            })
            .collect()
    }

    /// Emulate one video frame. Returns a slice into the framebuffer.
    pub fn run_frame(&mut self) -> &[u32] {
        self.frame_complete = false;
        let cycles_per_line = self.model.cycles_per_line();
        while !self.frame_complete {
            // Drive IRQ/NMI lines from chips.
            self.cpu.set_irq(self.vic.irq_line || self.cia1.irq_line);
            if self.cia2.irq_line { self.cpu.trigger_nmi(); self.cia2.irq_line = false; }

            let mut spent = 0u32;
            while spent < cycles_per_line {
                let mut bus = SystemBus {
                    mem: &mut self.mem, vic: &mut self.vic, sid: &mut self.sid,
                    cia1: &mut self.cia1, cia2: &mut self.cia2,
                    kb: &self.kb, joy1: &self.joy1, joy2: &self.joy2,
                    ext: &mut self.ext,
                };
                let c = self.cpu.step(&mut bus);
                let c = c.max(1) as u32;
                spent += c;
            }

            // Apply CIA2 vic-bank selection (writes to DD00 update this).
            self.mem.vic_bank_base = (self.cia2.vic_bank_sel as u16) * 0x4000;

            self.vic.step_line(&self.mem);
            self.sid.clock(cycles_per_line);
            self.cia1.tick(cycles_per_line);
            self.cia2.tick(cycles_per_line);

            if self.vic.frame_done {
                self.vic.frame_done = false;
                self.frame_complete = true;
            }
        }
        self.fb_snapshot.clone_from(&self.vic.fb);
        let _ = self.joy_port;
        &self.fb_snapshot
    }

    /// Drain accumulated audio samples (mono, sample_rate Hz).
    pub fn drain_audio(&mut self) -> Vec<i16> { self.sid.drain() }
}

/// Bus adapter passed into the CPU. Owns mutable references to all chips.
pub struct SystemBus<'a> {
    /// Main memory + banking.
    pub mem: &'a mut Memory,
    /// VIC-II state.
    pub vic: &'a mut VicII,
    /// SID state.
    pub sid: &'a mut Sid,
    /// CIA1.
    pub cia1: &'a mut Cia,
    /// CIA2.
    pub cia2: &'a mut Cia,
    /// Keyboard matrix (read-only).
    pub kb: &'a KeyboardMatrix,
    /// Joystick #1.
    pub joy1: &'a JoystickState,
    /// Joystick #2.
    pub joy2: &'a JoystickState,
    /// Extensions.
    pub ext: &'a mut Extensions,
}

impl<'a> Bus for SystemBus<'a> {
    fn read(&mut self, addr: u16) -> u8 {
        // Fast path: RAM / ROM via the PLA
        use crate::memory::Region;
        let region = self.mem.pla.region(addr);
        match region {
            Region::Io => self.read_io(addr),
            _ => self.mem.cpu_read(addr),
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        use crate::memory::Region;
        let region = self.mem.pla.region(addr);
        if let Region::Io = region {
            self.write_io(addr, val);
        } else {
            self.mem.cpu_write(addr, val);
        }
    }
}

impl<'a> SystemBus<'a> {
    fn read_io(&mut self, addr: u16) -> u8 {
        match addr {
            0xD000..=0xD3FF => self.vic.read(addr),
            0xD400..=0xD7FF => self.sid.read(addr),
            0xD800..=0xDBFF => self.mem.color_read(addr) | 0xF0,
            0xDC00..=0xDCFF => self.cia1.read(addr as u8, self.kb, &joystick_for(self.cia1.index, self.joy1, self.joy2)),
            0xDD00..=0xDDFF => self.cia2.read(addr as u8, self.kb, &joystick_for(self.cia2.index, self.joy1, self.joy2)),
            0xDE00..=0xDEFF => self.ext.read(addr),
            0xDF00..=0xDFFF => 0xFF,
            _ => 0xFF,
        }
    }

    fn write_io(&mut self, addr: u16, val: u8) {
        match addr {
            0xD000..=0xD3FF => self.vic.write(addr, val),
            0xD400..=0xD7FF => self.sid.write(addr, val),
            0xD800..=0xDBFF => self.mem.color_write(addr, val),
            0xDC00..=0xDCFF => self.cia1.write(addr as u8, val),
            0xDD00..=0xDDFF => self.cia2.write(addr as u8, val),
            0xDE00..=0xDEFF => self.ext.write(addr, val, &mut self.mem.ram),
            _ => {}
        }
    }
}

fn joystick_for(_i: CiaIndex, joy1: &JoystickState, joy2: &JoystickState) -> JoystickState {
    // For now both ports see both joysticks; refinements would route PA/PB separately.
    JoystickState { bits: joy1.bits | joy2.bits }
}
