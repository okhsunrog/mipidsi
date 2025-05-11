mod spi;
pub use spi::*;

mod parallel;
pub use parallel::*;
// Command and pixel interface
pub trait Interface {
    /// The native width of the interface (e.g., u8 for SPI, u8/u16 for parallel).
    type Word: Copy; // This might always become u8 if you simplify enough

    /// Error type
    type Error: core::fmt::Debug;

    /// Kind of interface
    const KIND: InterfaceKind;

    /// Send a command with optional parameters.
    /// (Keep this, assuming async based on previous context)
    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error>;

    /// Send a raw slice of data, typically pre-formatted pixel data.
    /// `WriteMemoryStart` (or equivalent) must be sent before calling this function.
    /// The data is assumed to be in the correct format for the display and interface.
    /// If Self::Word is u8, data is &[u8]. If Self::Word is u16, data is &[u16].
    /// For your goal of passing &[u8] directly, we'll aim for Self::Word = u8
    /// or handle the u8 slice appropriately in implementations.
    async fn send_data_slice(&mut self, data: &[Self::Word]) -> Result<(), Self::Error>;
}

// Update the blanket impl for &mut T
impl<T: Interface + ?Sized> Interface for &mut T {
    type Word = T::Word;
    type Error = T::Error;
    const KIND: InterfaceKind = T::KIND;

    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        T::send_command(self, command, args).await
    }

    async fn send_data_slice(&mut self, data: &[Self::Word]) -> Result<(), Self::Error> {
        T::send_data_slice(self, data).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InterfaceKind {
    Serial4Line,
    Parallel8Bit,
    Parallel16Bit,
}
