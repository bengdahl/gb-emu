use gb_core::cpu::{Cpu, CpuInputPins, CpuOutputPins, CpuRunner, FRegister};

pub const RESULT_ADDR: u16 = 0xAA55;
pub const RESULT_ADDR_LO: u8 = 0x55;
pub const RESULT_ADDR_HI: u8 = 0xAA;

/// Represents either a write to $AA55, or an unexpected error that caused the test machine to halt.
pub type InstructionTestResult = Result<(Cpu, u8), InstructionTestError>;

pub enum InstructionTestError {
    OutOfRangeAccess,
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
                            .ok_or(InstructionTestError::OutOfRangeAccess)
                        {
                            Ok(ptr) => {
                                *ptr = d;
                                0
                            }
                            Err(e) => {
                                self.error = true;
                                return Some(Err(e));
                            }
                        },
                        None => match self
                            .memory
                            .get((addr - self.code_offset) as usize)
                            .ok_or(InstructionTestError::OutOfRangeAccess)
                        {
                            Ok(d) => *d,
                            Err(e) => {
                                self.error = true;
                                return Some(Err(e));
                            }
                        },
                    };

                    let out = self.cpu.clock(CpuInputPins { data });
                    self.cycles_elapsed += 1;
                    if self.cycles_elapsed >= self.max_cycles.unwrap_or(u64::MAX) {
                        self.error = true;
                        return Some(Err(InstructionTestError::MaxCyclesReached));
                    }

                    match out {
                        CpuOutputPins {
                            is_read: true,
                            addr,
                            ..
                        } => {
                            self.last_access = addr;
                            self.to_write = None;
                        }
                        CpuOutputPins {
                            is_read: false,
                            addr,
                            data,
                            ..
                        } => {
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
        cpu.clock(CpuInputPins { data: 0 }),
        // Should fetch first instruction
        CpuOutputPins {
            is_read: true,
            addr: 0,
            ..
        }
    ),);

    assert!(matches!(
        cpu.clock(CpuInputPins {
            data: 0x00 // NOP
        }),
        // Recieved NOP, should immediately fetch next instruction due to fetch/execute overlap
        CpuOutputPins {
            is_read: true,
            addr: 1,
            ..
        }
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
