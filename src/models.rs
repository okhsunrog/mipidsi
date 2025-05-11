//! Display models.

use crate::{
    dcs::{self, InterfaceExt, SetAddressMode}, // Added DcsCommand, InterfaceExt
    interface::Interface,
    options::{self, ModelOptions, Rotation},
};
use embedded_hal_async::delay::DelayNs;

pub use crate::builder::ConfigurationError;

// mod gc9107;
// mod gc9a01;
// mod ili9225;
// mod ili9341;
// mod ili9342c;
// mod ili934x;
// mod ili9486;
// mod ili9488;
// mod ili948x;
// mod rm67162;
// mod st7735s;
mod st7789;
// mod st7796;

// pub use gc9107::*;
// pub use gc9a01::*;
// pub use ili9225::*;
// pub use ili9341::*;
// pub use ili9342c::*;
// pub use ili934x::*;
// pub use ili9486::*;
// pub use ili9488::*;
// pub use ili948x::*;
// pub use rm67162::*;
// pub use st7735s::*;
pub use st7789::*;
// pub use st7796::*;

/// Display model.
pub trait Model: Sized {
    const FRAMEBUFFER_SIZE: (u16, u16);
    const RESET_DURATION: u32 = 10;

    async fn init<DELAY, DI>(
        &mut self,
        di: &mut DI,
        delay: &mut DELAY,
        options: &ModelOptions,
    ) -> Result<SetAddressMode, ModelInitError<DI::Error>>
    where
        DELAY: DelayNs,
        DI: Interface; // DI will also impl InterfaceExt automatically

    async fn update_options<DI>(
        &self,
        di: &mut DI,
        options: &ModelOptions,
    ) -> Result<SetAddressMode, DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
    {
        let madctl_cmd = SetAddressMode::from(options);
        di.write_command(madctl_cmd).await?; // Use InterfaceExt::write_command
        Ok(madctl_cmd) // Return the struct
    }

    async fn update_address_window<DI>(
        di: &mut DI,
        _rotation: Rotation,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
    {
        di.write_command(dcs::SetColumnAddress::new(sx, ex)).await?;
        di.write_command(dcs::SetPageAddress::new(sy, ey)).await
    }

    async fn sleep<DI, DELAY>(di: &mut DI, delay: &mut DELAY) -> Result<(), DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
        DELAY: DelayNs,
    {
        di.write_command(dcs::EnterSleepMode).await?;
        delay.delay_us(120_000).await;
        Ok(())
    }

    async fn wake<DI, DELAY>(di: &mut DI, delay: &mut DELAY) -> Result<(), DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
        DELAY: DelayNs,
    {
        di.write_command(dcs::ExitSleepMode).await?;
        delay.delay_us(120_000).await;
        Ok(())
    }

    async fn write_memory_start<DI>(di: &mut DI) -> Result<(), DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
    {
        di.write_command(dcs::WriteMemoryStart).await
    }

    async fn software_reset<DI>(di: &mut DI) -> Result<(), DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
    {
        di.write_command(dcs::SoftReset).await
        // Consider adding a small delay here if required by datasheets after SoftReset
    }

    async fn set_tearing_effect<DI>(
        di: &mut DI,
        tearing_effect: options::TearingEffect,
        _options: &ModelOptions,
    ) -> Result<(), DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
    {
        di.write_command(dcs::SetTearingEffect::new(tearing_effect))
            .await
    }

    async fn set_vertical_scroll_region<DI>(
        di: &mut DI,
        top_fixed_area: u16,
        bottom_fixed_area: u16,
    ) -> Result<(), DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
    {
        let rows = Self::FRAMEBUFFER_SIZE.1;
        let vsa_cmd = if top_fixed_area + bottom_fixed_area > rows {
            dcs::SetScrollArea::new(rows, 0, 0)
        } else {
            dcs::SetScrollArea::new(
                top_fixed_area,
                rows - top_fixed_area - bottom_fixed_area,
                bottom_fixed_area,
            )
        };
        di.write_command(vsa_cmd).await
    }

    async fn set_vertical_scroll_offset<DI>(di: &mut DI, offset: u16) -> Result<(), DI::Error>
    where
        DI: Interface, // DI will also impl InterfaceExt
    {
        di.write_command(dcs::SetScrollStart::new(offset)).await
    }
}

/// Error returned by [`Model::init`].
#[derive(Debug)]
pub enum ModelInitError<DiError> {
    Interface(DiError),
    InvalidConfiguration(ConfigurationError),
}

impl<DiError> From<DiError> for ModelInitError<DiError> {
    fn from(value: DiError) -> Self {
        Self::Interface(value)
    }
}
