//! Contains logic for CPU operation

use super::decode;
use super::{CpuInputPins, CpuOutputPins, FRegister};

impl super::Cpu {
    /// Set the output pins to fetch the memory located at the address in the PC register, and then increment the PC register.
    /// The value of the address pins is equal to the PC register *before* being incremented.
    fn fetch_byte(&mut self) -> CpuOutputPins {
        let pc = self.registers.get_pc();
        self.registers.set_pc(pc.wrapping_add(1));
        CpuOutputPins {
            addr: pc,
            data: 0,
            is_read: true,
        }
    }

    /// Set the pins to write a byte to memory
    fn write_byte(&self, addr: u16, data: u8) -> CpuOutputPins {
        CpuOutputPins {
            addr,
            data,
            is_read: false,
        }
    }

    fn read_byte(&self, addr: u16) -> CpuOutputPins {
        CpuOutputPins {
            addr,
            data: 0,
            is_read: true,
        }
    }

    fn store_16_bits(&mut self, v: u16, dest: LoadDest16Bit) {
        match dest {
            LoadDest16Bit::AF => self.registers.set_af(v),
            LoadDest16Bit::BC => self.registers.set_bc(v),
            LoadDest16Bit::DE => self.registers.set_de(v),
            LoadDest16Bit::HL => self.registers.set_hl(v),
            LoadDest16Bit::SP => self.registers.set_sp(v),
        }
    }

    fn read_16_bits(&mut self, from: LoadDest16Bit) -> u16 {
        match from {
            LoadDest16Bit::AF => self.registers.get_af(),
            LoadDest16Bit::BC => self.registers.get_bc(),
            LoadDest16Bit::DE => self.registers.get_de(),
            LoadDest16Bit::HL => self.registers.get_hl(),
            LoadDest16Bit::SP => self.registers.get_sp(),
        }
    }

    /// Perform an ALU operation on the accumulator and update the flags register. The operation is chosen by:
    ///
    /// 0 = ADD, 1 = ADC, 2 = SUB, 3 = SBC, 4 = AND, 5 = XOR, 6 = OR, 7 = CP
    fn do_math(&mut self, v: u8, operation: MathOperation) {
        use MathOperation::*;
        match operation {
            Add => {
                let a = self.registers.get_a();
                let (sum, overflow) = a.overflowing_add(v);
                self.registers.set_a(sum);
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, sum == 0);
                    f.set_value(FRegister::HALFCARRY, (a & 0x0f) + (v & 0x0f) >= 0x10);
                    f.set_value(FRegister::CARRY, overflow);

                    f
                })
            }
            Adc => {
                let a = self.registers.get_a();
                let (sum, overflow1) = a.overflowing_add(v);
                let (sum, overflow2) =
                    sum.overflowing_add(if self.registers.get_f().contains(FRegister::CARRY) {
                        1
                    } else {
                        0
                    });
                let overflow = overflow1 | overflow2;
                self.registers.set_a(sum);
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, sum == 0);
                    f.set_value(FRegister::HALFCARRY, (a & 0x0f) + (v & 0x0f) >= 0x10);
                    f.set_value(FRegister::CARRY, overflow);

                    f
                })
            }
            Sub => {
                let a = self.registers.get_a();
                let nv = (!v).wrapping_add(1); // Two's complement of v (makes flags easier)
                let (sum, overflow) = a.overflowing_add(nv);
                self.registers.set_a(sum);
                self.registers.modify_f(|mut f| {
                    f.set(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, sum == 0);
                    f.set_value(FRegister::HALFCARRY, (a & 0x0f) + (nv & 0x0f) >= 0x10);
                    f.set_value(FRegister::CARRY, overflow);

                    f
                })
            }
            Sbc => {
                let a = self.registers.get_a();
                let (sum, overflow1) = a.overflowing_add(v);
                let (sum, overflow2) =
                    sum.overflowing_add(if self.registers.get_f().contains(FRegister::CARRY) {
                        1
                    } else {
                        0
                    });
                let overflow = overflow1 | overflow2;
                self.registers.set_a(sum);
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, sum == 0);
                    f.set_value(FRegister::HALFCARRY, (a & 0x0f) + (v & 0x0f) >= 0x10);
                    f.set_value(FRegister::CARRY, overflow);

                    f
                })
            }
            And => {
                self.registers.modify_a(|a| a & v);
                let new_a = self.registers.get_a();
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, new_a == 0);
                    f.set(FRegister::HALFCARRY);
                    f.unset(FRegister::CARRY);

                    f
                });
            }
            Xor => {
                self.registers.modify_a(|a| a ^ v);
                let new_a = self.registers.get_a();
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, new_a == 0);
                    f.unset(FRegister::HALFCARRY);
                    f.unset(FRegister::CARRY);

                    f
                });
            }
            Or => {
                self.registers.modify_a(|a| a | v);
                let new_a = self.registers.get_a();
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, new_a == 0);
                    f.unset(FRegister::HALFCARRY);
                    f.unset(FRegister::CARRY);

                    f
                });
            }
            Cp => {
                let a = self.registers.get_a();
                let nv = (!v).wrapping_add(1); // Two's complement of v (makes flags easier)
                let (sum, overflow) = a.overflowing_add(nv);
                self.registers.modify_f(|mut f| {
                    f.set(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, sum == 0);
                    f.set_value(FRegister::HALFCARRY, (a & 0x0f) + (nv & 0x0f) >= 0x10);
                    f.set_value(FRegister::CARRY, overflow);

                    f
                })
            }
        }
    }

    pub fn runner(self) -> CpuRunner {
        CpuRunner {
            cpu: self,
            gen: Box::pin(cpu_runner_gen()),
        }
    }
}

/// Provides a wrapper to use around the generator underneath the CPU execution logic.
pub struct CpuRunner {
    pub cpu: super::Cpu,
    gen: std::pin::Pin<
        Box<
            dyn std::ops::Generator<
                (super::Cpu, CpuInputPins),
                Yield = (super::Cpu, CpuOutputPins),
                Return = !,
            >,
        >,
    >,
}

impl CpuRunner {
    pub fn clock(&mut self, pins: CpuInputPins) -> CpuOutputPins {
        use std::ops::GeneratorState;
        match self.gen.as_mut().resume((self.cpu, pins)) {
            GeneratorState::Yielded((cpu, pins_out)) => {
                self.cpu = cpu;
                pins_out
            }
            GeneratorState::Complete(_) => unreachable!(),
        }
    }
}

/// Yields a generator containing state that will run the cpu
fn cpu_runner_gen(
) -> impl std::ops::Generator<(super::Cpu, CpuInputPins), Yield = (super::Cpu, CpuOutputPins), Return = !>
{
    // Every `yield` here will cause the CPU to wait for one memory cycle.
    #[allow(unused_assignments)]
    move |t: (super::Cpu, CpuInputPins)| {
        let (mut cpu, mut pins) = t;
        loop {
            macro_rules! cpu_yield {
                ($yielded:expr) => {
                    let _yielded = $yielded;
                    (cpu, pins) = yield (cpu, _yielded);
                };
            }

            /// Store an 8 bit value into a register specified by the `r` table. Yields a cpu cycle on indirect HL write, unyielding otherwise.
            ///
            /// See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
            macro_rules! store_8_bits {
                ($self:ident, $v:expr, $dest:expr) => {
                    match $dest {
                        LoadDest::B => $self.registers.set_b($v),
                        LoadDest::C => $self.registers.set_c($v),
                        LoadDest::D => $self.registers.set_d($v),
                        LoadDest::E => $self.registers.set_e($v),
                        LoadDest::H => $self.registers.set_h($v),
                        LoadDest::L => $self.registers.set_l($v),
                        LoadDest::IndHL => {
                            cpu_yield!($self.write_byte($self.registers.get_hl(), $v));
                        }
                        LoadDest::A => $self.registers.set_a($v),
                    }
                };
            }

            /// Read an 8 bit value from a register specified by the `r` table. Yields a cpu cycle on indirect HL read, unyielding otherwise.
            ///
            /// See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
            macro_rules! read_8_bits {
                ($self:ident, $dest:expr) => {
                    match $dest {
                        LoadDest::B => $self.registers.get_b(),
                        LoadDest::C => $self.registers.get_c(),
                        LoadDest::D => $self.registers.get_d(),
                        LoadDest::E => $self.registers.get_e(),
                        LoadDest::H => $self.registers.get_h(),
                        LoadDest::L => $self.registers.get_l(),
                        LoadDest::IndHL => {
                            cpu_yield!($self.read_byte($self.registers.get_hl()));
                            pins.data
                        }
                        LoadDest::A => $self.registers.get_a(),
                    }
                };
            }

            // Fetch
            cpu_yield!(cpu.fetch_byte());
            let opcode = super::decode::Opcode(pins.data);

            // Decode & execute
            //
            // Note: `continue` will immediately jump back to the instruction fetch logic.
            // This is intentional and is part of the fetch/execute overlap optimization done on the real cpu.
            //
            // FIXME: Unfortunately since rust has no equivalent to python's `yield from`, I cant think of a clean
            // way to factor this out. This must sadly stay one gigantic function until I can find some workaround.
            match opcode.x() {
                0 => match opcode.z() {
                    1 if opcode.q() == 0 => {
                        // 16-bit LD
                        let dst = decode::rp(opcode.p());

                        cpu_yield!(cpu.fetch_byte());
                        let low = pins.data;
                        cpu_yield!(cpu.fetch_byte());
                        let high = pins.data;

                        let v = ((high as u16) << 8) | (low as u16);
                        cpu.store_16_bits(v, dst);
                    }
                    2 if opcode.q() == 0 => {
                        // LD to memory
                        let addr = match opcode.y() {
                            0 => cpu.registers.get_bc(),
                            1 => cpu.registers.get_de(),
                            2 => {
                                let a = cpu.registers.get_hl();
                                cpu.registers.modify_hl(|hl| hl.wrapping_add(1));
                                a
                            }
                            3 => {
                                let a = cpu.registers.get_hl();
                                cpu.registers.modify_hl(|hl| hl.wrapping_sub(1));
                                a
                            }
                            _ => unreachable!(),
                        };

                        cpu_yield!(cpu.write_byte(addr, cpu.registers.get_a()));
                    }
                    2 if opcode.q() == 1 => {
                        // LD from memory
                        let addr = match opcode.y() {
                            0 => cpu.registers.get_bc(),
                            1 => cpu.registers.get_de(),
                            2 => {
                                let a = cpu.registers.get_hl();
                                cpu.registers.modify_hl(|hl| hl.wrapping_add(1));
                                a
                            }
                            3 => {
                                let a = cpu.registers.get_hl();
                                cpu.registers.modify_hl(|hl| hl.wrapping_sub(1));
                                a
                            }
                            _ => unreachable!(),
                        };

                        cpu_yield!(cpu.read_byte(addr));
                        cpu.registers.set_a(pins.data);
                        continue;
                    }
                    6 => {
                        // LD from immediate
                        let dst = decode::r(opcode.y());

                        cpu_yield!(cpu.fetch_byte());
                        store_8_bits!(cpu, pins.data, dst);
                        continue;
                    }

                    0x00 => continue, // NOP

                    _ => todo!("x=1 ({:#X?})", opcode),
                },
                1 if opcode.z() == 6 && opcode.y() == 6 => todo!("HLT"),
                1 => {
                    // 8-bit register-to-register LD
                    let dst = decode::r(opcode.y());
                    let from = decode::r(opcode.z());

                    let v = read_8_bits!(cpu, from);
                    store_8_bits!(cpu, v, dst);
                    continue;
                }
                2 => {
                    let op = decode::alu(opcode.y());
                    let reg = decode::r(opcode.z());

                    let v = read_8_bits!(cpu, reg);
                    cpu.do_math(v, op);
                    continue;
                }
                3 => match opcode.z() {
                    1 if opcode.q() == 0 => {
                        // POP
                        let dst = decode::rp2(opcode.p());

                        cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                        let low = pins.data;
                        cpu.registers.modify_sp(|sp| sp.wrapping_add(1));
                        cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                        let high = pins.data;
                        cpu.registers.modify_sp(|sp| sp.wrapping_add(1));

                        let v = ((high as u16) << 8) | (low as u16);
                        cpu.store_16_bits(v, dst);
                        continue;
                    }
                    5 if opcode.q() == 0 => {
                        // PUSH
                        let from = decode::rp2(opcode.p());
                        let v = cpu.read_16_bits(from);

                        cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                        let high = (v >> 8) as u8;
                        cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), high));
                        cpu.registers.modify_sp(|sp| sp.wrapping_add(1));
                        let low = (v & 0x00ff) as u8;
                        cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), low));
                        continue;
                    }
                    _ => todo!("x=3 ({:#X?})", opcode),
                },
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathOperation {
    Add = 0,
    Adc = 1,
    Sub = 2,
    Sbc = 3,
    And = 4,
    Xor = 5,
    Or = 6,
    Cp = 7,
}

/// 8 bit registers specified by the `r` table.
///
/// See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadDest {
    B,
    C,
    D,
    E,
    H,
    L,
    IndHL,
    A,
}

/// 16 bit register pairs used by the `rp` and `rp2` tables.
///
/// See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadDest16Bit {
    AF,
    BC,
    DE,
    HL,
    SP,
}
