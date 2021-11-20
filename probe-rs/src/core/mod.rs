pub(crate) mod communication_interface;

pub use communication_interface::CommunicationInterface;
pub use probe_rs_target::Architecture;
use probe_rs_target::CoreType;

use crate::architecture::{
    arm::core::State, riscv::communication_interface::RiscvCommunicationInterface,
};
use crate::error;
use crate::Target;
use crate::{Error, Memory, MemoryInterface};
use anyhow::{anyhow, Result};
use std::time::Duration;

pub trait CoreRegister: Clone + From<u32> + Into<u32> + Sized + std::fmt::Debug {
    const ADDRESS: u32;
    const NAME: &'static str;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CoreRegisterAddress(pub u16);

impl From<CoreRegisterAddress> for u32 {
    fn from(value: CoreRegisterAddress) -> Self {
        u32::from(value.0)
    }
}

impl From<u16> for CoreRegisterAddress {
    fn from(value: u16) -> Self {
        CoreRegisterAddress(value)
    }
}
#[derive(Debug, Clone)]
pub struct CoreInformation {
    pub pc: u32,
}

#[derive(Debug, Clone)]
pub struct RegisterDescription {
    pub(crate) name: &'static str,
    pub(crate) kind: RegisterKind,
    pub(crate) address: CoreRegisterAddress,
}

impl RegisterDescription {
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl From<RegisterDescription> for CoreRegisterAddress {
    fn from(description: RegisterDescription) -> CoreRegisterAddress {
        description.address
    }
}

impl From<&RegisterDescription> for CoreRegisterAddress {
    fn from(description: &RegisterDescription) -> CoreRegisterAddress {
        description.address
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RegisterKind {
    General,
    PC,
}

/// Register description for a core.
#[derive(Debug)]
pub struct RegisterFile {
    pub(crate) platform_registers: &'static [RegisterDescription],

    /// Register description for the program counter
    pub(crate) program_counter: &'static RegisterDescription,

    pub(crate) stack_pointer: &'static RegisterDescription,

    pub(crate) return_address: &'static RegisterDescription,

    pub(crate) argument_registers: &'static [RegisterDescription],
    pub(crate) result_registers: &'static [RegisterDescription],
}

impl RegisterFile {
    pub fn registers(&self) -> impl Iterator<Item = &RegisterDescription> {
        self.platform_registers.iter()
    }

    pub fn program_counter(&self) -> &RegisterDescription {
        self.program_counter
    }

    pub fn stack_pointer(&self) -> &RegisterDescription {
        self.stack_pointer
    }

    pub fn return_address(&self) -> &RegisterDescription {
        self.return_address
    }

    pub fn argument_register(&self, index: usize) -> &RegisterDescription {
        &self.argument_registers[index]
    }

    pub fn get_argument_register(&self, index: usize) -> Option<&RegisterDescription> {
        self.argument_registers.get(index)
    }

    pub fn result_register(&self, index: usize) -> &RegisterDescription {
        &self.result_registers[index]
    }

    pub fn get_result_register(&self, index: usize) -> Option<&RegisterDescription> {
        self.result_registers.get(index)
    }

    pub fn platform_register(&self, index: usize) -> &RegisterDescription {
        &self.platform_registers[index]
    }

    pub fn get_platform_register(&self, index: usize) -> Option<&RegisterDescription> {
        self.platform_registers.get(index)
    }
}

pub trait CoreInterface: MemoryInterface {
    /// Wait until the core is halted. If the core does not halt on its own,
    /// a [`DebugProbeError::Timeout`](crate::DebugProbeError::Timeout) error will be returned.
    fn wait_for_core_halted(&mut self, timeout: Duration) -> Result<(), error::Error>;

    /// Check if the core is halted. If the core does not halt on its own,
    /// a [`DebugProbeError::Timeout`](crate::DebugProbeError::Timeout) error will be returned.
    fn core_halted(&mut self) -> Result<bool, error::Error>;

    fn status(&mut self) -> Result<CoreStatus, error::Error>;

    /// Try to halt the core. This function ensures the core is actually halted, and
    /// returns a [`DebugProbeError::Timeout`](crate::DebugProbeError::Timeout) otherwise.
    fn halt(&mut self, timeout: Duration) -> Result<CoreInformation, error::Error>;

    fn run(&mut self) -> Result<(), error::Error>;

    /// Reset the core, and then continue to execute instructions. If the core
    /// should be halted after reset, use the [`reset_and_halt`] function.
    ///
    /// [`reset_and_halt`]: Core::reset_and_halt
    fn reset(&mut self) -> Result<(), error::Error>;

    /// Reset the core, and then immediately halt. To continue execution after
    /// reset, use the [`reset`] function.
    ///
    /// [`reset`]: Core::reset
    fn reset_and_halt(&mut self, timeout: Duration) -> Result<CoreInformation, error::Error>;

    /// Steps one instruction and then enters halted state again.
    fn step(&mut self) -> Result<CoreInformation, error::Error>;

    fn read_core_reg(&mut self, address: CoreRegisterAddress) -> Result<u32, error::Error>;

    fn write_core_reg(&mut self, address: CoreRegisterAddress, value: u32) -> Result<()>;

    fn get_available_breakpoint_units(&mut self) -> Result<u32, error::Error>;

    /// Read the hardware breakpoints from FpComp registers, and adds them to the Result Vector.
    /// A value of None in any position of the Vector indicates that the position is unset/available.
    /// We intentionally return all breakpoints, irrespective of whether they are enabled or not.
    fn get_hw_breakpoints(&mut self) -> Result<Vec<Option<u32>>, error::Error>;

    fn enable_breakpoints(&mut self, state: bool) -> Result<(), error::Error>;

    fn set_hw_breakpoint(&mut self, bp_unit_index: usize, addr: u32) -> Result<(), error::Error>;

    fn clear_hw_breakpoint(&mut self, unit_index: usize) -> Result<(), error::Error>;

    fn registers(&self) -> &'static RegisterFile;

    fn hw_breakpoints_enabled(&self) -> bool;

    /// Get the `Architecture` of the Core.
    fn architecture(&self) -> Architecture;
}

impl<'probe> MemoryInterface for Core<'probe> {
    fn read_word_32(&mut self, address: u32) -> Result<u32, Error> {
        self.inner.read_word_32(address)
    }

    fn read_word_8(&mut self, address: u32) -> Result<u8, Error> {
        self.inner.read_word_8(address)
    }

    fn read_32(&mut self, address: u32, data: &mut [u32]) -> Result<(), Error> {
        self.inner.read_32(address, data)
    }

    fn read_8(&mut self, address: u32, data: &mut [u8]) -> Result<(), Error> {
        self.inner.read_8(address, data)
    }

    fn write_word_32(&mut self, addr: u32, data: u32) -> Result<(), Error> {
        self.inner.write_word_32(addr, data)
    }

    fn write_word_8(&mut self, addr: u32, data: u8) -> Result<(), Error> {
        self.inner.write_word_8(addr, data)
    }

    fn write_32(&mut self, addr: u32, data: &[u32]) -> Result<(), Error> {
        self.inner.write_32(addr, data)
    }

    fn write_8(&mut self, addr: u32, data: &[u8]) -> Result<(), Error> {
        self.inner.write_8(addr, data)
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.inner.flush()
    }
}

#[derive(Debug)]
pub struct CoreState {
    id: usize,
}

impl CoreState {
    pub fn new(id: usize) -> Self {
        Self { id }
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

#[derive(Debug)]
pub enum SpecificCoreState {
    Armv6m(State),
    Armv7m(State),
    Armv7em(State),
    Armv8m(State),
    Riscv,
}

impl SpecificCoreState {
    pub(crate) fn from_core_type(typ: CoreType) -> Self {
        match typ {
            CoreType::Armv6m => SpecificCoreState::Armv6m(State::new()),
            CoreType::Armv7m => SpecificCoreState::Armv7m(State::new()),
            CoreType::Armv7em => SpecificCoreState::Armv7m(State::new()),
            CoreType::Armv8m => SpecificCoreState::Armv8m(State::new()),
            CoreType::Riscv => SpecificCoreState::Riscv,
        }
    }

    pub(crate) fn core_type(&self) -> CoreType {
        match self {
            SpecificCoreState::Armv6m(_) => CoreType::Armv6m,
            SpecificCoreState::Armv7m(_) => CoreType::Armv7m,
            SpecificCoreState::Armv7em(_) => CoreType::Armv7em,
            SpecificCoreState::Armv8m(_) => CoreType::Armv8m,
            SpecificCoreState::Riscv => CoreType::Riscv,
        }
    }

    pub(crate) fn attach_arm<'probe, 'target: 'probe>(
        &'probe mut self,
        state: &'probe mut CoreState,
        memory: Memory<'probe>,
        target: &'target Target,
    ) -> Result<Core<'probe>, Error> {
        let debug_sequence = match &target.debug_sequence {
            crate::config::DebugSequence::Arm(sequence) => sequence.clone(),
            crate::config::DebugSequence::Riscv(_) => {
                return Err(Error::UnableToOpenProbe(
                    "Core architecture and Probe mismatch.",
                ))
            }
        };

        Ok(match self {
            SpecificCoreState::Armv6m(s) => Core::new(
                crate::architecture::arm::armv6m::Armv6m::new(memory, s, debug_sequence)?,
                state,
            ),
            SpecificCoreState::Armv7m(s) | SpecificCoreState::Armv7em(s) => Core::new(
                crate::architecture::arm::armv7m::Armv7m::new(memory, s, debug_sequence)?,
                state,
            ),
            SpecificCoreState::Armv8m(s) => Core::new(
                crate::architecture::arm::armv8m::Armv8m::new(memory, s, debug_sequence)?,
                state,
            ),
            _ => {
                return Err(Error::UnableToOpenProbe(
                    "Core architecture and Probe mismatch.",
                ))
            }
        })
    }

    pub fn attach_riscv<'probe>(
        &self,
        state: &'probe mut CoreState,
        interface: &'probe mut RiscvCommunicationInterface,
    ) -> Result<Core<'probe>, Error> {
        Ok(match self {
            SpecificCoreState::Riscv => {
                Core::new(crate::architecture::riscv::Riscv32::new(interface), state)
            }
            _ => {
                return Err(Error::UnableToOpenProbe(
                    "Core architecture and Probe mismatch.",
                ))
            }
        })
    }
}

pub struct Core<'probe> {
    inner: Box<dyn CoreInterface + 'probe>,
    state: &'probe mut CoreState,
}

impl<'probe> Core<'probe> {
    pub fn new(core: impl CoreInterface + 'probe, state: &'probe mut CoreState) -> Core<'probe> {
        Self {
            inner: Box::new(core),
            state,
        }
    }

    pub fn create_state(id: usize) -> CoreState {
        CoreState::new(id)
    }

    pub fn id(&self) -> usize {
        self.state.id
    }

    /// Wait until the core is halted. If the core does not halt on its own,
    /// a [`DebugProbeError::Timeout`](crate::DebugProbeError::Timeout) error will be returned.
    pub fn wait_for_core_halted(&mut self, timeout: Duration) -> Result<(), error::Error> {
        self.inner.wait_for_core_halted(timeout)
    }

    /// Check if the core is halted. If the core does not halt on its own,
    /// a [`DebugProbeError::Timeout`](crate::DebugProbeError::Timeout) error will be returned.
    pub fn core_halted(&mut self) -> Result<bool, error::Error> {
        self.inner.core_halted()
    }

    /// Try to halt the core. This function ensures the core is actually halted, and
    /// returns a [`DebugProbeError::Timeout`](crate::DebugProbeError::Timeout) otherwise.
    pub fn halt(&mut self, timeout: Duration) -> Result<CoreInformation, error::Error> {
        self.inner.halt(timeout)
    }

    pub fn run(&mut self) -> Result<(), error::Error> {
        self.inner.run()
    }

    /// Reset the core, and then continue to execute instructions. If the core
    /// should be halted after reset, use the [`reset_and_halt`] function.
    ///
    /// [`reset_and_halt`]: Core::reset_and_halt
    pub fn reset(&mut self) -> Result<(), error::Error> {
        self.inner.reset()
    }

    /// Reset the core, and then immediately halt. To continue execution after
    /// reset, use the [`reset`] function.
    ///
    /// [`reset`]: Core::reset
    pub fn reset_and_halt(&mut self, timeout: Duration) -> Result<CoreInformation, error::Error> {
        self.inner.reset_and_halt(timeout)
    }

    /// Steps one instruction and then enters halted state again.
    pub fn step(&mut self) -> Result<CoreInformation, error::Error> {
        self.inner.step()
    }

    pub fn status(&mut self) -> Result<CoreStatus, error::Error> {
        self.inner.status()
    }

    pub fn read_core_reg(
        &mut self,
        address: impl Into<CoreRegisterAddress>,
    ) -> Result<u32, error::Error> {
        self.inner.read_core_reg(address.into())
    }

    pub fn write_core_reg(
        &mut self,
        address: CoreRegisterAddress,
        value: u32,
    ) -> Result<(), error::Error> {
        Ok(self.inner.write_core_reg(address, value)?)
    }

    pub fn get_available_breakpoint_units(&mut self) -> Result<u32, error::Error> {
        self.inner.get_available_breakpoint_units()
    }

    fn enable_breakpoints(&mut self, state: bool) -> Result<(), error::Error> {
        self.inner.enable_breakpoints(state)
    }

    pub fn registers(&self) -> &'static RegisterFile {
        self.inner.registers()
    }

    /// Find the index of the next available HW breakpoint comparator.
    fn find_free_breakpoint_comparator_index(&mut self) -> Result<usize, error::Error> {
        let mut next_available_hw_breakpoint = 0;
        for breakpoint in self.inner.get_hw_breakpoints()? {
            if breakpoint.is_none() {
                return Ok(next_available_hw_breakpoint);
            } else {
                next_available_hw_breakpoint += 1;
            }
        }
        Err(error::Error::Other(anyhow!(
            "No available hardware breakpoints"
        )))
    }

    /// Set a hardware breakpoint
    ///
    /// This function will try to set a hardware breakpoint. The amount
    /// of hardware breakpoints which are supported is chip specific,
    /// and can be queried using the `get_available_breakpoint_units` function.
    pub fn set_hw_breakpoint(&mut self, address: u32) -> Result<(), error::Error> {
        if !self.inner.hw_breakpoints_enabled() {
            self.enable_breakpoints(true)?;
        }

        // If there is a breakpoint set already, return its bp_unit_index, else find the next free index.
        let breakpoint_comparator_index = match self
            .inner
            .get_hw_breakpoints()?
            .iter()
            .position(|&bp| bp == Some(address))
        {
            Some(breakpoint_comparator_index) => breakpoint_comparator_index,
            None => self.find_free_breakpoint_comparator_index()?,
        };

        log::debug!(
            "Trying to set HW breakpoint #{} with comparator address  {:#08x}",
            breakpoint_comparator_index,
            address
        );

        // Actually set the breakpoint. Even if it has been set, set it again so it will be active.
        self.inner
            .set_hw_breakpoint(breakpoint_comparator_index, address)?;
        Ok(())
    }

    pub fn clear_hw_breakpoint(&mut self, address: u32) -> Result<(), error::Error> {
        let bp_position = self
            .inner
            .get_hw_breakpoints()?
            .iter()
            .position(|bp| bp.is_some() && bp.unwrap() == address);

        log::debug!(
            "Will clear HW breakpoint    #{} with comparator address    {:#08x}",
            bp_position.unwrap_or(usize::MAX),
            address
        );

        match bp_position {
            Some(bp_position) => {
                self.inner.clear_hw_breakpoint(bp_position)?;
                Ok(())
            }
            None => Err(error::Error::Other(anyhow!(
                "No breakpoint found at address {}",
                address
            ))),
        }
    }

    /// Clear all hardware breakpoints
    ///
    /// This function will clear all HW breakpoints which are configured on the target,
    /// regardless if they are set by probe-rs, AND regardless if they are enabled or not.
    /// Also used as a helper function in [`Session::drop`](crate::session::Session).
    pub fn clear_all_hw_breakpoints(&mut self) -> Result<(), error::Error> {
        log::trace!("clear all hw bps");
        for breakpoint in (self.inner.get_hw_breakpoints()?).into_iter().flatten() {
            self.clear_hw_breakpoint(breakpoint)?
        }
        Ok(())
    }

    pub fn architecture(&self) -> Architecture {
        self.inner.architecture()
    }
}

pub struct CoreList<'probe>(&'probe [CoreType]);

impl<'probe> CoreList<'probe> {
    pub fn new(cores: &'probe [CoreType]) -> Self {
        Self(cores)
    }
}

impl<'probe> std::ops::Deref for CoreList<'probe> {
    type Target = [CoreType];
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BreakpointId(usize);

impl BreakpointId {
    pub fn new(id: usize) -> Self {
        BreakpointId(id)
    }
}

#[derive(Clone, Debug)]
pub struct Breakpoint {
    address: u32,
    register_hw: usize,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CoreStatus {
    Running,
    Halted(HaltReason),
    /// This is a Cortex-M specific status, and will not be set or handled by RISCV code.
    LockedUp,
    Sleeping,
    Unknown,
}

impl CoreStatus {
    pub fn is_halted(&self) -> bool {
        matches!(self, CoreStatus::Halted(_))
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum HaltReason {
    /// Multiple reasons for a halt.
    ///
    /// This can happen for example when a single instruction
    /// step ends up on a breakpoint, after which both breakpoint and step / request
    /// are set.
    Multiple,
    /// Core halted due to a breakpoint, either
    /// a *soft* or a *hard* breakpoint.
    Breakpoint,
    /// Core halted due to an exception, e.g. an
    /// an interrupt.
    Exception,
    /// Core halted due to a data watchpoint
    Watchpoint,
    /// Core halted after single step
    Step,
    /// Core halted because of a debugger request
    Request,
    /// External halt request
    External,
    /// Unknown reason for halt.
    ///
    /// This can happen for example when the core is already halted when we connect.
    Unknown,
}
