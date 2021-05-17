use gb_core::cpu::{Cpu, CpuInputPins, CpuOutputPins, CpuRunner, FRegister};

pub const RESULT_ADDR: u16 = 0xAA55;
pub const RESULT_ADDR_LO: u8 = 0x55;
pub const RESULT_ADDR_HI: u8 = 0xAA;

/// Represents either a write to $AA55, or an unexpected error that caused the test machine to halt.
pub type InstructionTestResult = Result<(Cpu, u8), InstructionTestError>;

#[derive(Debug)]
pub enum InstructionTestError {
    OutOfRangeAccess(u16, u16),
    MaxCyclesReached,
}

pub struct InstructionTest {
    pub cpu: Cpu,
    pub code: Vec<u8>,
    pub code_offset: u16,
}

impl InstructionTest {
    pub fn new(init_cpu: Cpu, code: Vec<u8>, code_offset: u16) -> Self {
        InstructionTest {
            cpu: init_cpu,
            code,
            code_offset,
        }
    }

    /// Run the cpu and return every write to $AA55 (stops after n cycles)
    pub fn run<'a>(
        self,
        max_cycles: Option<u64>,
    ) -> impl Iterator<Item = InstructionTestResult> + 'a {
        struct Running {
            error: bool,
            cycles_elapsed: u64,
            max_cycles: Option<u64>,
            cpu: CpuRunner,
            memory: Vec<u8>,
            code_offset: u16,
            last_access: u16,
            to_write: Option<u8>,
        }

        impl Iterator for Running {
            type Item = InstructionTestResult;
            fn next(&mut self) -> Option<Self::Item> {
                if self.error {
                    return None;
                }

                loop {
                    let addr = self.last_access;
                    let data = match self.to_write {
                        // Ignore reads and writes to $AA55
                        _ if addr == RESULT_ADDR => 0,
                        Some(d) => match self
                            .memory
                            .get_mut((addr - self.code_offset) as usize)
                            .ok_or(InstructionTestError::OutOfRangeAccess(
                                self.cpu.cpu.registers.get_pc(),
                                addr,
                            )) {
                            Ok(ptr) => {
                                *ptr = d;
                                0
                            }
                            Err(e) => {
                                self.error = true;
                                return Some(Err(e));
                            }
                        },
                        None => match self.memory.get((addr - self.code_offset) as usize).ok_or(
                            InstructionTestError::OutOfRangeAccess(
                                self.cpu.cpu.registers.get_pc(),
                                addr,
                            ),
                        ) {
                            Ok(d) => *d,
                            Err(e) => {
                                self.error = true;
                                return Some(Err(e));
                            }
                        },
                    };

                    let out = self.cpu.clock(CpuInputPins {
                        data,
                        ..Default::default()
                    });
                    println!("CPU: {:?}", self.cpu.cpu);
                    self.cycles_elapsed += 1;
                    if self.cycles_elapsed >= self.max_cycles.unwrap_or(u64::MAX) {
                        self.error = true;
                        return Some(Err(InstructionTestError::MaxCyclesReached));
                    }

                    match out {
                        CpuOutputPins::Read { addr } => {
                            self.last_access = addr;
                            self.to_write = None;
                        }
                        CpuOutputPins::Write { addr, data } => {
                            self.last_access = addr;
                            self.to_write = Some(data);
                        }
                    }

                    if self.last_access == RESULT_ADDR {
                        if let Some(d) = self.to_write {
                            return Some(Ok((self.cpu.cpu, d)));
                        }
                    }
                }
            }
        }

        Running {
            error: false,
            cycles_elapsed: 0,
            max_cycles,
            cpu: self.cpu.runner(),
            memory: self.code,
            code_offset: self.code_offset,
            last_access: self.code_offset,
            to_write: None,
        }
    }
}

#[test]
fn nop() {
    let cpu = Cpu::default();

    let mut cpu = cpu.runner();

    assert!(matches!(
        cpu.clock(CpuInputPins {
            data: 0,
            ..Default::default()
        }),
        // Should fetch first instruction
        CpuOutputPins::Read { addr: 0 }
    ),);

    assert!(matches!(
        cpu.clock(CpuInputPins {
            data: 0x00, // NOP
            ..Default::default()
        }),
        // Recieved NOP, should immediately fetch next instruction due to fetch/execute overlap
        CpuOutputPins::Read { addr: 1 }
    ));
}

#[test]
fn load() {
    let cpu = Cpu::default();

    let code = vec![
        0x3E, 0xA5, // LD A, $A5
        0x21, 0x55, 0xAA, // LD HL, $AA55
        0x77, // LD (HL), A
    ];

    let tester = InstructionTest::new(cpu, code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|t| t.1)
            .collect::<Vec<_>>(),
        vec![0xA5]
    );
}

#[test]
fn add() {
    let cpu = Cpu::default();

    let code = vec![
        0x21, 0x55, 0xAA, // LD HL, $AA55
        0x3E, 12, // LD A, 12
        0x06, 17,   // LD B, 17
        0x80, // ADD A,B
        0x77, // LD (HL), A
        0x3E, 0xFF, // LD A, $FF
        0x06, 1,    // LD B, 1
        0x80, // ADD A,B
        0x77, // LD (HL), A
        0x3E, 0x0F, // LD A, $F,
        0x06, 1,    // LD B, 1
        0x80, // ADD A,B
        0x77, // LD (HL), A
    ];

    let tester = InstructionTest::new(cpu, code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|(cpu, d)| (cpu.registers.get_f(), d))
            .collect::<Vec<_>>(),
        vec![
            (FRegister::EMPTY, 29),
            (FRegister::ZERO | FRegister::CARRY | FRegister::HALFCARRY, 0),
            (FRegister::HALFCARRY, 16),
        ]
    );
}

#[test]
fn adc() {
    let cpu = Cpu::default();

    let code = vec![
        0x21, 0x55, 0xAA, // LD HL, $AA55
        0x3E, 12, // LD A, 12
        0x06, 17,   // LD B, 17
        0x88, // ADC A,B
        0x77, // LD (HL), A
        0x3E, 0xFF, // LD A, $FF
        0x06, 1,    // LD B, 1
        0x88, // ADC A,B
        0x77, // LD (HL), A
        0x3E, 0x0F, // LD A, $F,
        0x06, 1,    // LD B, 1
        0x88, // ADC A,B
        0x77, // LD (HL), A
        0x3E, 0xFF, // LD A, $FF
        0x06, 1,    // LD B, 1
        0x88, // ADC A,B
        0x77, // LD (HL), A
        0x3E, 1, // LD A, 1
        0x06, 0xFF, // LD B, $FF
        0x88, // ADC A,B
        0x77, // LD (HL), A
    ];

    let tester = InstructionTest::new(cpu, code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|(cpu, d)| (cpu.registers.get_f(), d))
            .collect::<Vec<_>>(),
        vec![
            (FRegister::EMPTY, 29),
            (FRegister::ZERO | FRegister::CARRY | FRegister::HALFCARRY, 0),
            (FRegister::HALFCARRY, 17),
            (FRegister::ZERO | FRegister::CARRY | FRegister::HALFCARRY, 0),
            (FRegister::CARRY | FRegister::HALFCARRY, 1),
        ]
    );
}

#[test]
fn sub() {
    let cpu = Cpu::default();

    let code = vec![
        0x21, 0x55, 0xAA, // LD HL, $AA55
        0x3E, 17, // LD A, 17
        0x06, 12,   // LD B, 12
        0x90, // SUB B
        0x77, // LD (HL), A
        0x3E, 0, // LD A, 0
        0x06, 1,    // LD B, 1
        0x90, // SUB B
        0x77, // LD (HL), A
        0x3E, 0x10, // LD A, $10,
        0x06, 1,    // LD B, 1
        0x90, // SUB B
        0x77, // LD (HL), A
        0x3E, 5, // LD A, 5,
        0x06, 5,    // LD B, 5
        0x90, // SUB B
        0x77, // LD (HL), A
    ];

    let tester = InstructionTest::new(cpu, code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|(cpu, d)| (cpu.registers.get_f(), d))
            .collect::<Vec<_>>(),
        vec![
            (FRegister::NEGATIVE | FRegister::CARRY, 5),
            (FRegister::NEGATIVE, 0xFF),
            (FRegister::NEGATIVE | FRegister::CARRY, 15),
            (
                FRegister::NEGATIVE | FRegister::ZERO | FRegister::CARRY | FRegister::HALFCARRY,
                0
            ),
        ]
    );
}

#[test]
fn add_hl() {
    /// Output the opcodes necessary to perform an addition of two u16s and output the result in little endian order
    fn perform_add(a: u16, b: u16) -> Vec<u8> {
        let a_lo = (a & 0xff) as u8;
        let b_lo = (b & 0xff) as u8;
        let a_hi = (a >> 8) as u8;
        let b_hi = (b >> 8) as u8;

        vec![
            0x21, a_lo, a_hi, // LD HL, <a>
            0x01, b_lo, b_hi, // LD BC, <b>
            0x09, // ADD HL, BC
            0x54, // LD D, H
            0x5D, // LD E, L
            0x21, 0x55, 0xAA, // LD HL, $AA55
            0x73, // LD (HL), E
            0x72, // LD (HL), D
        ]
    }

    let cpu = Cpu::default();

    let additions = [
        (0x0105, 0x010B, 0x0210, FRegister::EMPTY),
        (0x00FF, 0x0001, 0x0100, FRegister::HALFCARRY),
        (
            0xFFFF,
            0x0001,
            0x0000,
            FRegister::HALFCARRY | FRegister::CARRY,
        ),
        (0xFF00, 0x0100, 0x0000, FRegister::CARRY),
        (
            0xFFFF,
            0xFFFF,
            0xFFFE,
            FRegister::HALFCARRY | FRegister::CARRY,
        ),
    ];

    let code = additions
        .iter()
        .flat_map(|(a, b, _, _)| perform_add(*a, *b))
        .collect::<Vec<u8>>();

    let compare_against = additions
        .iter()
        .flat_map(|(_, _, r, f)| {
            let l = (r & 0xff) as u8;
            let h = (r >> 8) as u8;
            vec![(*f, l), (*f, h)]
        })
        .collect::<Vec<(FRegister, u8)>>();

    let tester = InstructionTest::new(cpu, code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|(cpu, d)| (cpu.registers.get_f(), d))
            .collect::<Vec<_>>(),
        compare_against
    )
}

#[test]
fn inc() {
    let code = vec![
        0x21, 0x55, 0xAA, // LD HL, $AA55
        0x3E, 0x1E, // LD A, $1E
        0x3C, // INC A
        0x77, // LD (HL), A
        0x3C, // INC A
        0x77, // LD (HL), A
        0x3E, 0xFF, // LD A, $FF
        0x3C, // INC A
        0x77, // LD (HL), A
    ];

    let tester = InstructionTest::new(Cpu::default(), code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|(cpu, d)| (cpu.registers.get_f(), d))
            .collect::<Vec<_>>(),
        vec![
            (FRegister::EMPTY, 0x1F),
            (FRegister::HALFCARRY, 0x20),
            (FRegister::ZERO | FRegister::HALFCARRY, 0),
        ]
    );
}

#[test]
fn dec() {
    let code = vec![
        0x21, 0x55, 0xAA, // LD HL, $AA55
        0x3E, 0x21, // LD A, $21
        0x3D, // DEC A
        0x77, // LD (HL), A
        0x3D, // DEC A
        0x77, // LD (HL), A
        0x3E, 0x01, // LD A, $01
        0x3D, // DEC A
        0x77, // LD (HL), A
        0x3D, // DEC A
        0x77, // LD (HL), A
    ];

    let tester = InstructionTest::new(Cpu::default(), code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|(cpu, d)| (cpu.registers.get_f(), d))
            .collect::<Vec<_>>(),
        vec![
            (FRegister::NEGATIVE | FRegister::HALFCARRY, 0x20),
            (FRegister::NEGATIVE, 0x1F),
            (
                FRegister::NEGATIVE | FRegister::ZERO | FRegister::HALFCARRY,
                0
            ),
            (FRegister::NEGATIVE, 0xFF),
        ]
    );
}

#[test]
fn inc_dec_16_bit() {
    let inc_checks = vec![(0x0123, 0x0124), (0x01FF, 0x0200), (0xFFFF, 0x0000)];

    let dec_checks = vec![(0x0123, 0x0122), (0x0200, 0x01FF), (0x0000, 0xFFFF)];

    let mut code = vec![
        0x21, 0x55, 0xAA, // LD HL, $AA55
    ];

    for (i, _) in inc_checks.iter() {
        let lo = (i & 0xFF) as u8;
        let hi = (i >> 8) as u8;
        code.extend_from_slice(&[
            0x01, lo, hi,   // LD BC, <i>
            0x03, // INC BC
            0x71, // LD (HL), C
            0x70, // LD (HL), B
        ]);
    }

    for (i, _) in dec_checks.iter() {
        let lo = (i & 0xFF) as u8;
        let hi = (i >> 8) as u8;
        code.extend_from_slice(&[
            0x01, lo, hi,   // LD BC, <i>
            0x0B, // DEC BC
            0x71, // LD (HL), C
            0x70, // LD (HL), B
        ]);
    }

    let compare_against = [inc_checks, dec_checks]
        .iter()
        .flatten()
        .flat_map(|(_, o)| {
            let lo = (o & 0xff) as u8;
            let hi = (o >> 8) as u8;
            vec![(FRegister::EMPTY, lo), (FRegister::EMPTY, hi)]
        })
        .collect::<Vec<(FRegister, u8)>>();

    let tester = InstructionTest::new(Cpu::default(), code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|(cpu, d)| (cpu.registers.get_f(), d))
            .collect::<Vec<_>>(),
        compare_against
    )
}

#[test]
#[rustfmt::skip]
fn jp() {
    let code = vec![
        0x21, 0x55, 0xAA, // LD HL, $AA55
        0x3E, 0x00, // LD A, $00
        0xC3, 0x0A, 0x00, // JP .jp1
        0x3E, 0xFF, // LD A, $FF
        // .jp1
        0x77, // LD (HL), A
    ];

    let tester = InstructionTest::new(Cpu::default(), code, 0);

    assert_eq!(
        tester
            .run(None)
            .filter_map(Result::ok)
            .map(|(cpu, d)| (cpu.registers.get_f(), d))
            .collect::<Vec<_>>(),
        vec![(FRegister::EMPTY, 0)]
    )
}

#[test]
#[rustfmt::skip]
fn jp_conditional() {
    // jp_conditional_test.S
    let code = vec![
        0x21, 0x55, 0xAA, 0xAF, 0x06, 0x00, 0xCA, 0x0B, 0x00, 0x06, 0xFF, 0x70, 0xF6, 0xFF, 0x06, 
        0xFF, 0xCA, 0x15, 0x00, 0x06, 0x00, 0x70, 0xAF, 0x06, 0xFF, 0xC2, 0x1E, 0x00, 0x06, 0x00, 
        0x70, 0xF6, 0xFF, 0x06, 0x00, 0xC2, 0x28, 0x00, 0x06, 0xFF, 0x70, 0x37, 0x06, 0x00, 0xDA, 
        0x31, 0x00, 0x06, 0xFF, 0x70, 0x37, 0x3F, 0x06, 0xFF, 0xDA, 0x3B, 0x00, 0x06, 0x00, 0x70, 
        0x37, 0x06, 0xFF, 0xD2, 0x44, 0x00, 0x06, 0x00, 0x70, 0x37, 0x3F, 0x06, 0x00, 0xD2, 0x4E, 
        0x00, 0x06, 0xFF, 0x70,
    ];

    let tester = InstructionTest::new(Cpu::default(), code, 0);

    assert!(
        tester
            .run(Some(1000))
            .all(|d| {
                match d {
                    Ok((_, d)) => d==0,
                    Err(InstructionTestError::MaxCyclesReached) => panic!(),
                    _ => true
                }
            })
    )
}

#[test]
fn call_ret() {
    // call_ret_test.S
    let code = vec![
        0x21, 0x00, 0x01, 0xF9, 0x21, 0x55, 0xAA, 0xC3, 0x10, 0x00, 0x06, 0x00, 0xC9, 0x06, 0xFF,
        0xC9, 0x06, 0xFF, 0xCD, 0x0A, 0x00, 0x70, 0xAF, 0x06, 0xFF, 0xCC, 0x0A, 0x00, 0x70, 0x06,
        0x00, 0xC4, 0x0D, 0x00, 0x70, 0x3C, 0x06, 0x00, 0xCC, 0x0D, 0x00, 0x70, 0x06, 0xFF, 0xC4,
        0x0A, 0x00, 0x70, 0x37, 0x06, 0xFF, 0xDC, 0x0A, 0x00, 0x70, 0x06, 0x00, 0xD4, 0x0D, 0x00,
        0x70, 0x3F, 0x06, 0x00, 0xCC, 0x0D, 0x00, 0x70, 0x06, 0xFF, 0xC4, 0x0A, 0x00, 0x70, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    let tester = InstructionTest::new(Cpu::default(), code, 0);

    let (outputs, end) = tester.run(None).partition::<Vec<_>, _>(|o| o.is_ok());

    let outputs: Vec<u8> = outputs.iter().map(|o| o.as_ref().unwrap().1).collect();
    let end = end[0].as_ref().unwrap_err();

    assert_eq!(
        outputs,
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        "end: {:?}",
        end
    );
}

#[test]
#[rustfmt::skip]
fn jr() {
    // jr_test.S
    let code = vec![
        0x21, 0x55, 0xAA, 0xAF, 0x06, 0x00, 0x28, 0x02, 0x06, 0xFF, 0x70, 0xF6, 0xFF, 0x06, 0xFF, 0x28, 
        0x02, 0x06, 0x00, 0x70, 0xAF, 0x06, 0xFF, 0x20, 0x02, 0x06, 0x00, 0x70, 0xF6, 0xFF, 0x06, 0x00, 
        0x20, 0x02, 0x06, 0xFF, 0x70, 0x37, 0x06, 0x00, 0x38, 0x02, 0x06, 0xFF, 0x70, 0x37, 0x3F, 0x06, 
        0xFF, 0x38, 0x02, 0x06, 0x00, 0x70, 0x37, 0x06, 0xFF, 0x30, 0x02, 0x06, 0x00, 0x70, 0x37, 0x3F, 
        0x06, 0x00, 0x30, 0x02, 0x06, 0xFF, 0x70, 0xC3, 0xAF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 
        0x4E, 0x06, 0x20, 0x70, 0xC3, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 
        0x10, 0x70, 0x18, 0xAD,
    ];

    let tester = InstructionTest::new(Cpu::default(), code, 0);

    let outputs = tester
        .run(None)
        .filter_map(|o| o.ok().map(|(_,v)| v))
        .collect::<Vec<u8>>();

    assert_eq!(
        outputs, 
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x20]
    );
}
