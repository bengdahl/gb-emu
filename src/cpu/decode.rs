//! Contains logic for instruction decoding
//!
//! This module is structured around the document:
//! https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
//!
//! Instructions in the Z80 (and by extension, the LR35902) instruction set can be decoded by interpreting each opcode
//! as an octal number with three digits (000-377). These digits can be used as indexes into a lookup table to easily
//! decode an instruction.
//!
//!    XXYYYZZZ
//!      __
//!      | _
//!      | |--> Q (Y>>1)
//!      |----> P (Y&1)

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Opcode(pub u8);

impl Opcode {
    /// The first octal digit of the opcode
    pub fn x(&self) -> u8 {
        self.0 >> 6
    }

    /// The second octal digit of the opcode
    pub fn y(&self) -> u8 {
        (self.0 & 0x38) >> 3
    }

    /// The third octal digit of the opcode
    pub fn z(&self) -> u8 {
        self.0 & 0x07
    }

    pub fn p(&self) -> u8 {
        self.y() >> 1
    }

    pub fn q(&self) -> u8 {
        self.y() & 1
    }
}

/// Represents table "r" in this document:
///
/// https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
#[inline]
pub fn r(i: u8) -> super::execute::LoadDest {
    assert!(i < 8, "value outside of octal range 0-7");
    use super::execute::LoadDest::*;
    // 0 = B, 1 = C, 2 = D, 3 = E, 4 = H, 5 = L, 6 = (HL), 7 = A
    match i {
        0 => B,
        1 => C,
        2 => D,
        3 => E,
        4 => H,
        5 => L,
        6 => IndHL,
        7 => A,
        _ => unreachable!(),
    }
}

/// Represents table "alu" in this document:
///
/// https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
#[inline]
pub fn alu(i: u8) -> super::execute::MathOperation {
    assert!(i < 8, "value outside of octal range 0-7");
    use super::execute::MathOperation::*;
    match i {
        0 => Add,
        1 => Adc,
        2 => Sub,
        3 => Sbc,
        4 => And,
        5 => Xor,
        6 => Or,
        7 => Cp,
        _ => unreachable!(),
    }
}

/// Represents table "rp" in this document:
///
/// https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
#[inline]
pub fn rp(i: u8) -> super::execute::LoadDest16Bit {
    assert!(i < 4, "value outside of range 0-3");
    use super::execute::LoadDest16Bit::*;
    // 0  1	 2  3
    // BC DE HL SP
    match i {
        0 => BC,
        1 => DE,
        2 => HL,
        3 => SP,
        _ => unreachable!(),
    }
}

/// Represents table "rp2" in this document:
///
/// https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
#[inline]
pub fn rp2(i: u8) -> super::execute::LoadDest16Bit {
    assert!(i < 4, "value outside of range 0-3");
    use super::execute::LoadDest16Bit::*;
    // 0  1	 2  3
    // BC DE HL AF
    match i {
        0 => BC,
        1 => DE,
        2 => HL,
        3 => AF,
        _ => unreachable!(),
    }
}
