use gb_core::cpu::{self, Cpu, CpuInputPins, CpuOutputPins, CpuRunner};

pub const RESULT_ADDR: u16 = 0xAA55;
pub const RESULT_ADDR_LO: u8 = 0x55;
pub const RESULT_ADDR_HI: u8 = 0xAA;

/// Represents either a write to $AA55, or an unexpected error that caused the test machine to halt.
pub type InstructionTestResult = Result<u8, InstructionTestError>;

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
        max_cycles: Option<usize>,
    ) -> impl Iterator<Item = InstructionTestResult> + 'a {
        struct Running {
            error: bool,
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
                            return Some(Ok(d))
                        }
                    }
                }
            }
        }

        Running {
            error: false,
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
    let mut cpu = Cpu::default();

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
        0x3E, 0xA5,         // LD A, $A5
        0x21, 0x55, 0xAA,   // LD HL, $AA55
        0x77                // LD (HL), A
    ];

    let tester = InstructionTest::new(cpu, code, 0);

    assert_eq!(
        tester.run(None).filter_map(Result::ok).collect::<Vec<_>>(),
        vec![0xA5]
    );
}
