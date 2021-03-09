use registers::{FRegister, Registers};

use crate::chip::Chip;

#[derive(Clone, Copy, Debug)]
pub struct Cpu {
    registers: Registers,
    state: CpuState,
}

impl Cpu {
    pub fn registers(&self) -> Registers {
        self.registers
    }

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

    fn fetch_next_instruction(&mut self) -> CpuOutputPins {
        self.state = CpuState::DecodeFirst;
        self.fetch_byte()
    }

    /// Decode the first byte of an instruction
    fn decode_first(&mut self, input: CpuInputPins) -> CpuOutputPins {
        // The giant switch statement
        match input.data {
            // First quarter of instructions
            d @ (0x00..=0x3F) => match d {
                d if (d & 0x0f == 0x2) => {
                    // LD to memory
                    let addr = match (d & 0xf0) >> 4 {
                        0 => self.registers.get_bc(),
                        1 => self.registers.get_de(),
                        2 => {
                            let a = self.registers.get_hl();
                            self.registers.modify_hl(|hl| hl.wrapping_add(1));
                            a
                        }
                        3 => {
                            let a = self.registers.get_hl();
                            self.registers.modify_hl(|hl| hl.wrapping_sub(1));
                            a
                        }
                        _ => unreachable!(),
                    };

                    self.state = CpuState::ReadyForInstruction;
                    self.write_byte(addr, self.registers.get_a())
                }
                d if (d & 0x0f == 0xa) => {
                    // LD from memory
                    let addr = match (d & 0xf0) >> 4 {
                        0 => self.registers.get_bc(),
                        1 => self.registers.get_de(),
                        2 => {
                            let a = self.registers.get_hl();
                            self.registers.modify_hl(|hl| hl.wrapping_add(1));
                            a
                        }
                        3 => {
                            let a = self.registers.get_hl();
                            self.registers.modify_hl(|hl| hl.wrapping_sub(1));
                            a
                        }
                        _ => unreachable!(),
                    };

                    self.state = CpuState::LoadFromMemory { dst_reg: 7 };
                    self.read_byte(addr)
                }
                d if (d & 0x07 == 0x6) => {
                    // LD from immediate
                    let dst = (d & 0x38) >> 3;

                    self.state = CpuState::LoadFromMemory { dst_reg: dst };
                    self.fetch_byte()
                }

                _ => unreachable!(),
            },
            // The big block of LD instructions (and HLT)
            d @ (0x40..=0x7F) => {
                // The way this block of opcodes is laid out lets us do some easy math to figure out what goes where
                // 0 = B, 1 = C, 2 = D, 3 = E, 4 = H, 5 = L, 6 = (HL), 7 = A
                let dst = (d & 0x38) >> 3;
                let src = d & 0x03;

                // HLT
                if dst == 6 && src == 6 {
                    todo!("HLT")
                }

                let v = match src {
                    0 => self.registers.get_b(),
                    1 => self.registers.get_c(),
                    2 => self.registers.get_d(),
                    3 => self.registers.get_e(),
                    4 => self.registers.get_h(),
                    5 => self.registers.get_l(),
                    6 => {
                        // Ask for the byte from memory and continue next memory clock
                        self.state = CpuState::LoadFromMemory { dst_reg: dst };
                        return self.read_byte(self.registers.get_hl());
                    }
                    7 => self.registers.get_a(),
                    _ => unreachable!(),
                };

                match dst {
                    0 => self.registers.set_b(v),
                    1 => self.registers.set_c(v),
                    2 => self.registers.set_d(v),
                    3 => self.registers.set_e(v),
                    4 => self.registers.set_h(v),
                    5 => self.registers.set_l(v),
                    6 => {
                        // Write the byte out to memory, fetch the next instruction once we're done.
                        self.state = CpuState::ReadyForInstruction;
                        return self.write_byte(self.registers.get_hl(), v);
                    }
                    7 => self.registers.set_a(v),
                    _ => unreachable!(),
                }

                // We've finished immediately, so we immediately fetch the next instruction to decode/execute
                self.fetch_next_instruction()
            }

            // The 0x80-0xBF arithmetic instructions
            d @ (0x80..=0xBF) => {
                // Same layout as in the LD block
                // 0 = B, 1 = C, 2 = D, 3 = E, 4 = H, 5 = L, 6 = (HL), 7 = A
                let src = d & 0x03;

                // 0 = ADD, 1 = ADC, 2 = SUB, 3 = SBC, 4 = AND, 5 = XOR, 6 = OR, 7 = CP
                let operation = (d & 0x38) >> 3;

                let v = match src {
                    0 => self.registers.get_b(),
                    1 => self.registers.get_c(),
                    2 => self.registers.get_d(),
                    3 => self.registers.get_e(),
                    4 => self.registers.get_h(),
                    5 => self.registers.get_l(),
                    6 => {
                        // Ask for the byte from memory and continue next memory clock
                        self.state = CpuState::MathFromMemory { operation };
                        return self.read_byte(self.registers.get_hl());
                    }
                    7 => self.registers.get_a(),
                    _ => unreachable!(),
                };

                self.do_math(v, operation);

                // ALU operations finish instantly, so fetch the next instruction immediately
                self.state = CpuState::DecodeFirst;
                self.fetch_byte()
            }

            // Last quarter
            d @ (0xC0..=0xFF) => match d {
                _ => unreachable!(),
            },
        }
    }

    /// Perform an ALU operation on the accumulator and update the flags register. The operation is chosen by:
    ///
    /// 0 = ADD, 1 = ADC, 2 = SUB, 3 = SBC, 4 = AND, 5 = XOR, 6 = OR, 7 = CP
    fn do_math(&mut self, v: u8, operation: u8) {
        match operation {
            0 => {
                // ADD
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
            1 => {
                // ADC
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
            2 => {
                // SUB
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
            3 => {
                // SBC
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
            4 => {
                // AND
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
            5 => {
                // XOR
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
            6 => {
                // OR
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
            7 => {
                // SUB
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
            _ => unreachable!(),
        }
    }
}

impl Chip for Cpu {
    type InputPins = CpuInputPins;
    type OutputPins = CpuOutputPins;

    fn clock(&mut self, input: Self::InputPins) -> Self::OutputPins {
        match self.state {
            CpuState::ReadyForInstruction => self.fetch_next_instruction(),
            CpuState::DecodeFirst => self.decode_first(input),
            CpuState::LoadFromMemory { dst_reg } => {
                match dst_reg {
                    0 => self.registers.set_b(input.data),
                    1 => self.registers.set_c(input.data),
                    2 => self.registers.set_d(input.data),
                    3 => self.registers.set_e(input.data),
                    4 => self.registers.set_h(input.data),
                    5 => self.registers.set_l(input.data),
                    6 => {
                        self.state = CpuState::ReadyForInstruction;
                        return self.write_byte(self.registers.get_hl(), input.data);
                    }
                    7 => self.registers.set_a(input.data),
                    _ => unreachable!(),
                }

                self.fetch_next_instruction()
            }
            CpuState::MathFromMemory { operation } => {
                self.do_math(input.data, operation);

                self.fetch_next_instruction()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    /// The CPU isn't doing anything, so it's ready to ask the memory for the next opcode
    ReadyForInstruction,
    /// The CPU is expecting the first byte of an instruction from memory
    DecodeFirst,
    /// The CPU is waiting on a read from memory into the specified register
    LoadFromMemory { dst_reg: u8 },
    /// The CPU is waiting on a read from memory to perform a math operation
    MathFromMemory { operation: u8 },
}

#[derive(Debug, Clone, Copy)]
pub struct CpuOutputPins {
    addr: u16,
    data: u8,
    is_read: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CpuInputPins {
    data: u8,
}

mod registers {
    use paste::paste;
    use std::{
        fmt::Debug,
        ops::{BitAnd, BitOr, BitOrAssign, Not},
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Registers {
        a: u8,
        f: FRegister,
        b: u8,
        c: u8,
        d: u8,
        e: u8,
        h: u8,
        l: u8,
        sp: u16,
        pc: u16,
    }

    macro_rules! reg_setters_and_getters {
        ($($reg:ident: $type: ident),+) => {
            $(
                paste! {
                    pub fn [<get_ $reg>](&self) -> $type {
                        self.$reg
                    }

                    pub fn [<set_ $reg>](&mut self, v: $type) {
                        self.$reg = v
                    }

                    pub fn [<modify_ $reg>]<F: FnOnce($type) -> $type>(&mut self, f: F) {
                        self.$reg = f(self.$reg)
                    }
                }
            )+
        };
    }

    macro_rules! combined_registers {
        ($combined:ident, $low:ident, $high:ident) => {
            paste! {
                pub fn [<get_ $combined>](&self) -> u16 {
                    (self.$high as u16) << 8 | self.$low as u16
                }

                pub fn [<set_ $combined>](&mut self, v: u16) {
                    self.$high = (v >> 8) as u8;
                    self.$low = (v&0xFF) as u8;
                }

                pub fn [<modify_ $combined>]<F: FnOnce(u16) -> u16>(&mut self, f: F) {
                    self.[<set_ $combined>](f(self.[<get_ $combined>]()));
                }
            }
        };
    }

    impl Registers {
        #![allow(dead_code)]
        reg_setters_and_getters!(
            a: u8,
            f: FRegister,
            b: u8,
            c: u8,
            d: u8,
            e: u8,
            h: u8,
            l: u8,
            sp: u16,
            pc: u16
        );

        pub fn get_af(&self) -> u16 {
            (self.a as u16) << 8 | u8::from(self.f) as u16
        }

        pub fn set_af(&mut self, v: u16) {
            self.a = (v >> 8) as u8;
            self.f = ((v & 0xFF) as u8).into();
        }

        pub fn modify_af<F: FnOnce(u16) -> u16>(&mut self, f: F) {
            self.set_af(f(self.get_af()));
        }

        combined_registers!(bc, c, b);
        combined_registers!(de, e, d);
        combined_registers!(hl, l, h);
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct FRegister(u8);

    impl FRegister {
        pub const ZERO: FRegister = FRegister(0x80);
        pub const NEGATIVE: FRegister = FRegister(0x40);
        pub const HALFCARRY: FRegister = FRegister(0x20);
        pub const CARRY: FRegister = FRegister(0x10);

        /// Returns true if any flags in the parameter are set in this value, and false otherwise
        #[inline(always)]
        pub fn contains(self, other: FRegister) -> bool {
            self.0 | other.0 != 0
        }

        /// Equivalent to `self = self | other`
        #[inline(always)]
        pub fn set(&mut self, other: FRegister) {
            *self = *self | other
        }

        /// Equivalent to `self = self & !other`
        #[inline(always)]
        pub fn unset(&mut self, other: FRegister) {
            *self = *self & !other
        }

        /// Equivalent to `if value { self.set(flags) } else { self.unset(flags) }`
        #[inline(always)]
        pub fn set_value(&mut self, flags: FRegister, value: bool) {
            if value {
                self.set(flags)
            } else {
                self.unset(flags)
            }
        }
    }

    impl Default for FRegister {
        fn default() -> Self {
            FRegister(0)
        }
    }

    impl BitOr for FRegister {
        type Output = Self;

        #[inline(always)]
        fn bitor(self, rhs: Self) -> Self::Output {
            FRegister(self.0 | rhs.0)
        }
    }

    impl BitOrAssign for FRegister {
        fn bitor_assign(&mut self, rhs: Self) {
            *self = *self | rhs
        }
    }

    impl BitAnd for FRegister {
        type Output = Self;

        #[inline(always)]
        fn bitand(self, rhs: Self) -> Self::Output {
            FRegister(self.0 & rhs.0)
        }
    }

    impl Not for FRegister {
        type Output = Self;

        #[inline(always)]
        fn not(self) -> Self::Output {
            FRegister((!self.0) & 0xF0)
        }
    }

    impl From<u8> for FRegister {
        fn from(v: u8) -> Self {
            FRegister(v & 0xF0)
        }
    }

    impl From<FRegister> for u8 {
        fn from(reg: FRegister) -> u8 {
            reg.0
        }
    }

    impl Debug for FRegister {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "")
        }
    }
}
