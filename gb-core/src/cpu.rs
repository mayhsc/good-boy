pub mod flags;
pub mod helpers;
pub mod registers;

use crate::{
    IE_ADDR, IF_ADDR,
    bus::{Bus, MemoryMap},
    cpu::{
        flags::Condition,
        registers::{Operand8, Reg8, Reg16, Registers},
    },
    joypad::Button,
    serial::link::SerialLink,
};

pub struct Cpu<B: Bus> {
    pub(crate) regs: Registers,
    pub(crate) bus: B,
    opcode: u8,
    ime: bool,
    ei_delay: u8,
    m_cycles: u32,
    halted: bool,
    halt_bug: bool,
}

impl<B: Bus> Cpu<B> {
    pub fn new(bus: B) -> Self {
        let mut cpu = Self {
            regs: Registers::default(),
            bus,
            opcode: 0,
            ime: false,
            ei_delay: 0,
            m_cycles: 0,
            halted: false,
            halt_bug: false,
        };
        cpu.opcode = cpu.fetch_byte();
        cpu
    }

    pub fn step(&mut self) -> u32 {
        self.m_cycles = 0;
        if self.halted {
            self.tick();
            if self.pending_interrupts() != 0 {
                self.halted = false;
                if self.ime {
                    self.regs.pc = self.regs.pc.wrapping_sub(1);
                    self.handle_interrupt();
                    self.opcode = self.fetch_byte();
                }
            }
            return self.m_cycles;
        }

        self.execute(self.opcode);
        self.handle_interrupt();

        if self.ei_delay > 0 {
            self.ei_delay -= 1;
            if self.ei_delay == 0 {
                self.ime = true;
            }
        }
        self.opcode = self.fetch_byte();
        self.m_cycles
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.m_cycles += 1;
        self.bus.tick_timers(4);
        self.bus.read(addr)
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.m_cycles += 1;
        self.bus.tick_timers(4);

        self.bus.write(addr, value);
    }

    pub fn tick(&mut self) {
        self.m_cycles += 1;
        self.bus.tick_timers(4);
    }

    fn fetch_byte(&mut self) -> u8 {
        let b = self.read(self.regs.pc);
        if self.halt_bug {
            self.halt_bug = false;
        } else {
            self.regs.increment();
        }
        b
    }
    fn fetch_byte16(&mut self) -> u16 {
        (self.fetch_byte() as u16) | ((self.fetch_byte() as u16) << 8)
    }

    fn handle_interrupt(&mut self) {
        if !self.ime {
            return;
        }

        self.tick();
        self.tick();

        let _if = self.bus.peek(IF_ADDR);
        let pending_interrupts = self.pending_interrupts();
        if pending_interrupts == 0 {
            return;
        }

        self.ime = false;
        self.ei_delay = 0;

        let (bit_index, target_pc) = match pending_interrupts {
            op if (op & 0x01) != 0 => (0, 0x0040), // V-Blank
            op if (op & 0x02) != 0 => (1, 0x0048), // LCD STAT
            op if (op & 0x04) != 0 => (2, 0x0050), // Timer
            op if (op & 0x08) != 0 => (3, 0x0058), // Serial
            op if (op & 0x10) != 0 => (4, 0x0060), // Joypad
            _ => return,
        };

        let updated_if = _if & !(1 << bit_index);
        self.write(IF_ADDR, updated_if);

        self.push(self.regs.pc);
        self.regs.pc = target_pc;
    }

    fn pending_interrupts(&self) -> u8 {
        let ie = self.bus.peek(IE_ADDR);
        let _if = self.bus.peek(IF_ADDR);

        ie & _if & 0x1F
    }

    fn execute(&mut self, op: u8) {
        match op {
            // NOP
            0x00 => {}
            // HLT
            0x76 => self.halt(),
            // LD
            0x40..=0x7f => {
                let src = helpers::operand8_from_index(op & 0b111);
                let dst = helpers::operand8_from_index((op >> 3) & 0b111);
                self.ld(dst, src);
            }
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => {
                let dst = helpers::operand8_from_index((op >> 3) & 0b111);
                let byte = self.fetch_byte();
                self.ld(dst, Operand8::D8(byte));
            }
            op if (op & 0b11001111 == 0b00000010) => {
                let v = self.regs.a;
                let addr = self
                    .regs
                    .get16mem(helpers::reg16mem_from_index(op >> 4 & 0b11));
                self.write(addr, v);
            }
            op if (op & 0b11001111 == 0b00001010) => {
                let addr = self
                    .regs
                    .get16mem(helpers::reg16mem_from_index(op >> 4 & 0b11));
                let v = self.read(addr);
                self.regs.set8(crate::cpu::registers::Reg8::A, v);
            }
            0x80..=0x9F => {
                let v = self.read_operand8_index(op & 0b111);
                let carry = ((op >> 3) & 0b1) == 1;
                let sub = ((op >> 4) & 0b1) == 1;

                if sub {
                    self.regs.sub_a(v, carry);
                } else {
                    self.regs.add_a(v, carry);
                }
            }
            0xC6 | 0xCE | 0xD6 | 0xDE => {
                let byte = self.fetch_byte();
                let carry = ((op >> 3) & 0b1) == 1;
                let sub = ((op >> 4) & 0b1) == 1;

                if sub {
                    self.regs.sub_a(byte, carry);
                } else {
                    self.regs.add_a(byte, carry);
                }
            }
            0xA0..=0xA7 => {
                let v = self.read_operand8_index(op & 0b111);
                self.regs.and(v);
            }
            0xA8..=0xAF => {
                let v = self.read_operand8_index(op & 0b111);
                self.regs.xor(v);
            }
            0xB0..=0xB7 => {
                let v = self.read_operand8_index(op & 0b111);
                self.regs.or(v);
            }
            0xB8..=0xBF => {
                let v = self.read_operand8_index(op & 0b111);
                self.regs.cp(v);
            }
            0xE6 | 0xEE | 0xf6 | 0xfe => {
                let v = self.fetch_byte();
                let a = (op >> 3) & 0b00000111;
                if a == 0b100 {
                    self.regs.and(v);
                } else if a == 0b101 {
                    self.regs.xor(v);
                } else if a == 0b110 {
                    self.regs.or(v);
                } else {
                    self.regs.cp(v);
                }
            }
            op if (op & 0b11001111 == 0b00000001) => {
                let r16 = helpers::reg16_from_index((op >> 4) & 0b11);
                let v = self.fetch_byte16();
                self.regs.set16(r16, v);
            }
            0x08 => {
                let addr = self.fetch_byte16();
                let v = self.regs.sp;
                self.write(addr, v as u8);
                self.write(addr + 1, (v >> 8) as u8);
            }
            op if (op & 0b11001111 == 0b00000011) => {
                let r = helpers::reg16_from_index((op >> 4) & 0b11);
                let (v, _) = self.regs.get16(&r).overflowing_add(1);
                self.regs.set16(r, v);
                self.tick();
            }
            op if (op & 0b11001111 == 0b00001011) => {
                let r = helpers::reg16_from_index((op >> 4) & 0b11);
                let (v, _) = self.regs.get16(&r).overflowing_sub(1);
                self.regs.set16(r, v);
                self.tick();
            }
            op if (op & 0b11001111 == 0b00001001) => {
                let r = helpers::reg16_from_index((op >> 4) & 0b11);
                let v = self.regs.get16(&r);
                self.regs.add_hl(v);
                self.tick();
            }
            op if (op & 0b11000111 == 0b00000100) => {
                let dst = helpers::operand8_from_index((op >> 3) & 0b111);
                let v = self.read_operand8_index((op >> 3) & 0b111);
                self.inc(dst, v);
            }
            op if (op & 0b11000111 == 0b00000101) => {
                let dst = helpers::operand8_from_index((op >> 3) & 0b111);
                let v = self.read_operand8_index((op >> 3) & 0b111);
                self.dec(dst, v);
            }
            0x07 => self.regs.rlca(),
            0x0F => self.regs.rrca(),
            0x17 => self.regs.rla(),
            0x1F => self.regs.rra(),
            0x27 => self.regs.daa(),
            0x2F => self.regs.cpl(),
            0x37 => self.regs.scf(),
            0x3F => self.regs.ccf(),
            0x18 => {
                let offset = self.fetch_byte() as i8;
                self.regs.pc = self.regs.pc.wrapping_add_signed(offset as i16);
                self.tick();
            }
            op if (op & 0b11100111 == 0b00100000) => {
                let offset = self.fetch_byte() as i8;
                let c = helpers::cond_from_index((op >> 3) & 0b11);
                if self.regs.get_condition(c) {
                    self.regs.jr(offset);
                    self.tick();
                }
            }
            op if (op & 0b11100111 == 0b11000000) => {
                let c = helpers::cond_from_index((op >> 3) & 0b11);
                self.tick();
                if self.regs.get_condition(c) {
                    self.ret();
                }
            }
            0xC9 => {
                self.ret();
            }
            0xD9 => {
                self.ret();
                self.ime = true;
            }
            op if (op & 0b11100111 == 0b11000010) => {
                let c = helpers::cond_from_index((op >> 3) & 0b11);
                self.jmp(Some(c));
            }
            0xC3 => {
                self.jmp(None);
            }
            0xE9 => self.regs.pc = self.regs.hl(),
            op if (op & 0b11100111 == 0b11000100) => {
                let c = helpers::cond_from_index((op >> 3) & 0b11);

                self.call(Some(c));
            }
            op if (op & 0b11001011) == 0b11000001 => {
                let r = helpers::reg16stk_from_index((op >> 4) & 0b11);
                let push = ((op >> 2) & 1) == 1;
                if push {
                    let v = self.regs.get16stk(&r);
                    self.push(v);
                    self.tick();
                } else {
                    let v = self.pop();
                    self.regs.set16stk(r, v);
                }
                // self.tick();
            }
            0xCD => self.call(None),
            op if (op & 0b1100_0111) == 0b1100_0111 => {
                let tgt = ((op >> 3) & 0b111) as u16 * 8;
                self.call_addr(tgt);
                self.tick();
            }
            0xE2 => {
                let a = self.regs.get8(Reg8::A);
                self.write(0xFF00 + self.regs.get8(Reg8::C) as u16, a);
            }
            0xE0 => {
                let n = self.fetch_byte();
                let a = self.regs.get8(Reg8::A);
                self.write(0xFF00 + n as u16, a);
            }
            0xEA => {
                let addr = self.fetch_byte16();
                let a = self.regs.get8(Reg8::A);
                self.write(addr, a);
            }

            0xF2 => {
                let v = self.read(0xFF00 + self.regs.get8(Reg8::C) as u16);
                self.regs.set8(Reg8::A, v);
            }
            0xF0 => {
                let n = self.fetch_byte();
                let v = self.read(0xFF00 + n as u16);
                self.regs.set8(Reg8::A, v);
            }
            0xFA => {
                let addr = self.fetch_byte16();
                let v = self.read(addr);
                self.regs.set8(Reg8::A, v);
            }
            0xE8 => {
                let n = self.fetch_byte() as i8 as i16 as u16;
                let sp = self.regs.sp;
                let result = sp.wrapping_add(n);

                let half_carry = (sp & 0xF) + (n & 0xF) > 0xF;
                let carry = (sp & 0xFF) + (n & 0xFF) > 0xFF;

                self.tick();
                self.regs.sp = result;
                self.regs.f.set_z(false);
                self.regs.f.set_n(false);
                self.regs.f.set_h(half_carry);
                self.regs.f.set_c(carry);
                self.tick();
            }

            0xF8 => {
                let n = self.fetch_byte() as i8 as i16 as u16;
                let sp = self.regs.sp;
                let result = sp.wrapping_add(n);

                let half_carry = (sp & 0xF) + (n & 0xF) > 0xF;
                let carry = (sp & 0xFF) + (n & 0xFF) > 0xFF;

                self.tick();
                self.regs.set16(Reg16::HL, result);
                self.regs.f.set_z(false);
                self.regs.f.set_n(false);
                self.regs.f.set_h(half_carry);
                self.regs.f.set_c(carry);
            }
            0xF9 => {
                self.regs.sp = self.regs.hl();
                self.tick();
            }
            0xF3 => {
                self.ime = false;
                self.ei_delay = 0;
            }
            0xFB => {
                self.ei_delay = 2;
            }
            0xCB => {
                let cb_op = self.fetch_byte();
                self.execute_cb(cb_op);
            }
            0x10 => {}
            _ => {
                println!("Opcode: {op}");
                panic!("Instruction has not been implemented yet");
            }
        };
    }

    fn execute_cb(&mut self, op: u8) {
        let reg_idx = op & 0b111;
        let operand = helpers::operand8_from_index(reg_idx);
        let mut val = self.read_operand8_index(reg_idx);

        match op {
            0x00..=0x07 => {
                let carry = (val & 0x80) != 0;
                val = val.rotate_left(1);
                self.write_operand8(operand, val);
                self.regs.f.set_z(val == 0);
                self.regs.f.set_n(false);
                self.regs.f.set_h(false);
                self.regs.f.set_c(carry);
            }
            0x08..=0x0F => {
                let carry = (val & 0x01) != 0;
                val = val.rotate_right(1);
                self.write_operand8(operand, val);
                self.regs.f.set_z(val == 0);
                self.regs.f.set_n(false);
                self.regs.f.set_h(false);
                self.regs.f.set_c(carry);
            }
            0x10..=0x17 => {
                let old_carry = self.regs.f.c();
                let new_carry = (val & 0x80) != 0;
                val = (val << 1) | (old_carry as u8);
                self.write_operand8(operand, val);
                self.regs.f.set_z(val == 0);
                self.regs.f.set_n(false);
                self.regs.f.set_h(false);
                self.regs.f.set_c(new_carry);
            }
            0x18..=0x1F => {
                let old_carry = self.regs.f.c();
                let new_carry = (val & 0x01) != 0;
                val = (val >> 1) | ((old_carry as u8) << 7);
                self.write_operand8(operand, val);
                self.regs.f.set_z(val == 0);
                self.regs.f.set_n(false);
                self.regs.f.set_h(false);
                self.regs.f.set_c(new_carry);
            }
            0x20..=0x27 => {
                let carry = (val & 0x80) != 0;
                val <<= 1;
                self.write_operand8(operand, val);
                self.regs.f.set_z(val == 0);
                self.regs.f.set_n(false);
                self.regs.f.set_h(false);
                self.regs.f.set_c(carry);
            }
            0x28..=0x2F => {
                let carry = (val & 0x01) != 0;
                val = ((val as i8) >> 1) as u8;
                self.write_operand8(operand, val);
                self.regs.f.set_z(val == 0);
                self.regs.f.set_n(false);
                self.regs.f.set_h(false);
                self.regs.f.set_c(carry);
            }
            0x30..=0x37 => {
                val = (val << 4) | (val >> 4);
                self.write_operand8(operand, val);
                self.regs.f.set_z(val == 0);
                self.regs.f.set_n(false);
                self.regs.f.set_h(false);
                self.regs.f.set_c(false);
            }
            0x38..=0x3F => {
                let carry = (val & 0x01) != 0;
                val >>= 1;
                self.write_operand8(operand, val);
                self.regs.f.set_z(val == 0);
                self.regs.f.set_n(false);
                self.regs.f.set_h(false);
                self.regs.f.set_c(carry);
            }

            0x40..=0x7F => {
                let bit = (op >> 3) & 0b111;
                let is_zero = (val & (1 << bit)) == 0;
                self.regs.f.set_z(is_zero);
                self.regs.f.set_n(false);
                self.regs.f.set_h(true);
            }

            0x80..=0xBF => {
                let bit = (op >> 3) & 0b111;
                val &= !(1 << bit);
                self.write_operand8(operand, val);
            }

            0xC0..=0xFF => {
                let bit = (op >> 3) & 0b111;
                val |= 1 << bit;
                self.write_operand8(operand, val);
            }
        }
    }

    fn write_operand8(&mut self, dst: Operand8, v: u8) {
        match dst {
            Operand8::Reg(r) => self.regs.set8(r, v),
            Operand8::MemHL => self.write(self.regs.hl(), v),
            Operand8::D8(_) => panic!("Cannot write to immediate value D8"),
        }
    }
    fn pop(&mut self) -> u16 {
        let lo = self.read(self.regs.sp) as u16;
        self.regs.sp = self.regs.sp.wrapping_add(1);
        let hi = self.read(self.regs.sp) as u16;
        self.regs.sp = self.regs.sp.wrapping_add(1);
        (hi << 8) | lo
    }

    fn push(&mut self, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        self.write(self.regs.sp, hi);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        self.write(self.regs.sp, lo);
    }

    fn ld(&mut self, dst: Operand8, src: Operand8) {
        let v = match src {
            Operand8::Reg(r) => self.regs.get8(r),
            Operand8::MemHL => self.read(self.regs.hl()),
            Operand8::D8(v) => v,
        };

        match dst {
            Operand8::Reg(r) => self.regs.set8(r, v),
            Operand8::MemHL => self.write(self.regs.hl(), v),
            Operand8::D8(_) => panic!(),
        };
    }

    fn inc(&mut self, dst: Operand8, v: u8) {
        let (result, _) = v.overflowing_add(1);
        let half_carry = (v & 0xF) + (1 & 0xF) > 0xF;

        match dst {
            Operand8::Reg(r) => self.regs.set8(r, result),
            Operand8::MemHL => self.write(self.regs.hl(), result),
            Operand8::D8(_) => panic!(),
        }

        self.regs.f.set_z(result == 0);
        self.regs.f.set_n(false);
        self.regs.f.set_h(half_carry);
    }

    fn dec(&mut self, dst: Operand8, v: u8) {
        let (result, _) = v.overflowing_sub(1);
        let half_carry = (v & 0xF) < (1 & 0xF);

        match dst {
            Operand8::Reg(r) => self.regs.set8(r, result),
            Operand8::MemHL => self.write(self.regs.hl(), result),
            Operand8::D8(_) => panic!(),
        }

        self.regs.f.set_z(result == 0);
        self.regs.f.set_n(true);
        self.regs.f.set_h(half_carry);
    }

    fn ret(&mut self) {
        self.regs.pc = self.pop();
        self.tick();
    }

    fn jmp(&mut self, condition: Option<Condition>) {
        let addr = self.fetch_byte16();

        if condition.map_or(true, |cond| self.regs.get_condition(cond)) {
            self.regs.pc = addr;
            self.tick();
        }
    }

    fn call(&mut self, condition: Option<Condition>) {
        let addr = self.fetch_byte16();

        if condition.map_or(true, |cond| self.regs.get_condition(cond)) {
            self.push(self.regs.pc);
            self.regs.pc = addr;
            self.tick();
        }
    }

    fn call_addr(&mut self, addr: u16) {
        self.push(self.regs.pc);
        self.regs.pc = addr;
    }

    fn read_operand8_index(&mut self, idx: u8) -> u8 {
        match helpers::operand8_from_index(idx) {
            Operand8::Reg(r) => self.regs.get8(r),
            Operand8::MemHL => self.read(self.regs.hl()),
            Operand8::D8(v) => v,
        }
    }

    fn halt(&mut self) {
        if !self.ime && self.pending_interrupts() != 0 {
            self.halt_bug = true;
        } else {
            self.halted = true;
        }
    }

    #[cfg(test)]
    pub fn set_opcode(&mut self, op: u8) {
        self.opcode = op
    }
}

impl Cpu<MemoryMap> {
    pub fn frame_buffer(&self) -> &[u8] {
        &self.bus.framebuffer()
    }

    pub fn drawn(&self) -> &[bool] {
        &self.bus.drawn()
    }

    pub fn dump_vram(&self) {
        self.bus.dump_vram();
    }

    pub fn press(&mut self, btn: Button) {
        self.bus.press(btn);
    }

    pub fn release(&mut self, btn: Button) {
        self.bus.release(btn);
    }

    pub fn swap_samples(&mut self, out: &mut Vec<f32>) {
        self.bus.swap_samples(out);
    }

    pub fn dump_ext_ram(&self) -> Option<&[u8]> {
        self.bus.dump_ext_ram()
    }

    pub fn attach_link(&mut self, link: Option<Box<dyn SerialLink>>) {
        self.bus.attach_link(link);
    }
}
