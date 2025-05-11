use embedded_hal::digital::OutputPin;

use super::{Interface, InterfaceKind};

// OutputBus trait and GenericXBitBus implementations can remain largely the same.
// ... (OutputBus, Generic8BitBus, Generic16BitBus definitions) ...
pub trait OutputBus {
    type Word: Copy + From<u8> + Eq; // Ensure From<u8> and Eq are still relevant or adjust
    const KIND: InterfaceKind;
    type Error: core::fmt::Debug;
    fn set_value(&mut self, value: Self::Word) -> Result<(), Self::Error>;
}

// ... (Generic8BitBus and Generic16BitBus implementations)

/// Parallel interface error
#[derive(Clone, Copy, Debug)]
pub enum ParallelError<BUS, DC, WR> {
    Bus(BUS),
    Dc(DC),
    Wr(WR),
}

pub struct ParallelInterface<BUS, DC, WR> {
    bus: BUS,
    dc: DC,
    wr: WR,
}

impl<BUS, DC, WR> ParallelInterface<BUS, DC, WR>
where
    BUS: OutputBus,
    // BUS::Word: From<u8> + Eq, // This might be adjusted based on how you handle Word
    DC: OutputPin,
    WR: OutputPin,
{
    pub fn new(bus: BUS, dc: DC, wr: WR) -> Self {
        Self { bus, dc, wr }
    }

    pub fn release(self) -> (BUS, DC, WR) {
        (self.bus, self.dc, self.wr)
    }

    // Keep send_word as it's a fundamental operation for parallel interfaces
    async fn send_word(
        // Assuming async if OutputPin ops become async
        &mut self,
        word: BUS::Word,
    ) -> Result<(), ParallelError<BUS::Error, DC::Error, WR::Error>> {
        self.wr.set_low().map_err(ParallelError::Wr)?;
        self.bus.set_value(word).map_err(ParallelError::Bus)?;
        self.wr.set_high().map_err(ParallelError::Wr)
    }
}

impl<BUS, DC, WR> Interface for ParallelInterface<BUS, DC, WR>
where
    BUS: OutputBus, // BUS::Word will be u8 or u16
    DC: OutputPin,
    WR: OutputPin,
{
    type Word = BUS::Word; // This will be u8 for Generic8BitBus, u16 for Generic16BitBus
    type Error = ParallelError<BUS::Error, DC::Error, WR::Error>;

    const KIND: InterfaceKind = BUS::KIND;

    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(ParallelError::Dc)?;
        self.send_word(BUS::Word::from(command)).await?; // send_word is async
        if !args.is_empty() {
            self.dc.set_high().map_err(ParallelError::Dc)?;
            for &arg in args {
                self.send_word(BUS::Word::from(arg)).await?; // send_word is async
            }
        }
        Ok(())
    }

    async fn send_data_slice(&mut self, data: &[Self::Word]) -> Result<(), Self::Error> {
        // data is &[BUS::Word], so &[u8] for 8-bit bus, &[u16] for 16-bit bus.
        // If you want Display::show_raw_framebuffer to always take &[u8],
        // then the Display layer would need to convert &[u8] to &[u16] for 16-bit parallel,
        // or this function would take &[u8] and do the conversion.
        // For simplicity here, assuming data matches Self::Word.
        for &word in data {
            self.send_word(word).await?; // send_word is async
        }
        Ok(())
    }
}
