use super::Chip;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Start,
    Select,
    B,
    A,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Default)]
pub struct Joypad {
    pub start: bool,
    pub select: bool,
    pub b: bool,
    pub a: bool,
    pub down: bool,
    pub up: bool,
    pub left: bool,
    pub right: bool,

    p1: u8,
}

impl Joypad {
    pub fn press(&mut self, button: Button) {
        use Button::*;
        match button {
            Start => self.start = true,
            Select => self.select = true,
            B => self.b = true,
            A => self.a = true,
            Left => self.left = true,
            Right => self.right = true,
            Up => self.up = true,
            Down => self.down = true,
        }
    }

    pub fn release(&mut self, button: Button) {
        use Button::*;
        match button {
            Start => self.start = false,
            Select => self.select = false,
            B => self.b = false,
            A => self.a = false,
            Left => self.left = false,
            Right => self.right = false,
            Up => self.up = false,
            Down => self.down = false,
        }
    }
}

impl Chip for Joypad {
    fn clock(
        &mut self,
        input: crate::cpu::CpuOutputPins,
        data: &mut u8,
        interrupt_request: &mut u8,
    ) {
        match input {
            crate::cpu::CpuOutputPins::Write {
                addr: 0xFF00,
                data: v,
            } => {
                self.p1 = v & 0b00110000;
            }
            crate::cpu::CpuOutputPins::Read { addr: 0xFF00 } => {
                *data = self.p1;
            }
            _ => (),
        };

        let action_buttons = if self.p1 & 0b00100000 == 0 {
            let start = !bool_to_bit(self.start, 3);
            let select = !bool_to_bit(self.select, 2);
            let b = !bool_to_bit(self.b, 1);
            let a = !bool_to_bit(self.a, 0);

            0x0F & start & select & b & a
        } else {
            0x0F
        };

        let direction_buttons = if self.p1 & 0b00010000 == 0 {
            let down = !bool_to_bit(self.down, 3);
            let up = !bool_to_bit(self.up, 2);
            let left = !bool_to_bit(self.left, 1);
            let right = !bool_to_bit(self.right, 0);

            0x0F & down & up & left & right
        } else {
            0x0F
        };

        let old_p1 = self.p1;
        self.p1 = (old_p1 & 0xF0) | (action_buttons & direction_buttons);

        let interrupt = old_p1 & 0x0F == 0x0F && self.p1 & 0x0F != 0x0F;
        if interrupt {
            *interrupt_request |= 1 << 4;
        }
    }
}

fn bool_to_bit(b: bool, bit: usize) -> u8 {
    assert!(bit < 8);
    if b {
        1 << bit
    } else {
        0
    }
}
