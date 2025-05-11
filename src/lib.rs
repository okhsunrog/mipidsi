#![no_std]
#![allow(async_fn_in_trait)]

//! This crate provides a generic asynchronous display driver to connect to TFT displays
//! that implement the MIPI Display Command Set.
// ... (rest of your crate-level docs) ...

use crate::dcs::SetAddressMode; // Assuming dcs module is at crate root
pub mod interface;

use embedded_hal::digital::OutputPin as BlockingOutputPin;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;

pub mod options;
use crate::options::MemoryMapping; // Assuming options module is at crate root

mod builder;
pub use builder::*; // Uses the corrected builder

pub mod dcs;
pub mod models;
pub mod raw_framebuf;
use models::Model; // Uses the corrected Model trait

// pub mod _troubleshooting; // Optional

/// Display driver structure.
pub struct Display<DI, MODEL, RST>
where
    DI: interface::Interface,
    MODEL: Model, // Model trait is async for I/O methods, no ColorFormat bound here
    RST: BlockingOutputPin,
{
    /// The display interface.
    di: DI,
    /// The display model instance.
    model: MODEL,
    /// The reset pin.
    rst: Option<RST>,
    /// Display options.
    options: options::ModelOptions,
    /// Current MADCTL value (cached from model).
    madctl: SetAddressMode,
    /// Sleep state.
    sleeping: bool,
}

impl<DI, M, RST> Display<DI, M, RST>
where
    DI: interface::Interface,
    M: Model, // M is the concrete model type implementing the async Model trait
    RST: BlockingOutputPin,
{
    /// Returns the current display orientation.
    pub fn orientation(&self) -> options::Orientation {
        self.options.orientation
    }

    /// Sets the display orientation.
    pub async fn set_orientation(
        &mut self,
        orientation: options::Orientation,
    ) -> Result<(), DI::Error> {
        self.options.orientation = orientation;
        // `self.model` is an instance of M.
        // `update_options` is an async method on the Model trait that takes `&self` (model instance).
        let new_madctl = self
            .model
            .update_options(&mut self.di, &self.options)
            .await?;
        self.madctl = new_madctl;
        Ok(())
    }

    /// Sends a raw pixel data slice to the specified rectangular region of the display.
    pub async fn show_raw_data<DW>(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
        pixel_data: &[DW],
    ) -> Result<(), DI::Error>
    where
        DI: interface::Interface<Word = DW>,
        DW: Copy,
    {
        self.set_address_window(sx, sy, ex, ey).await?;
        // M::write_memory_start is an associated function on the Model trait (static-like for the type M)
        M::write_memory_start(&mut self.di).await?;
        self.di.send_data_slice(pixel_data).await
    }

    /// Sets the vertical scroll region of the display.
    pub async fn set_vertical_scroll_region(
        &mut self,
        top_fixed_area: u16,
        bottom_fixed_area: u16,
    ) -> Result<(), DI::Error> {
        M::set_vertical_scroll_region(&mut self.di, top_fixed_area, bottom_fixed_area).await
    }

    /// Sets the vertical scroll offset.
    pub async fn set_vertical_scroll_offset(&mut self, offset: u16) -> Result<(), DI::Error> {
        M::set_vertical_scroll_offset(&mut self.di, offset).await
    }

    /// Releases the display interface, model instance, and reset pin.
    pub fn release(self) -> (DI, M, Option<RST>) {
        (self.di, self.model, self.rst)
    }

    /// (Internal) Sets the address window for display RAM access.
    async fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), DI::Error> {
        let mut offset = self.options.display_offset;
        let mapping = MemoryMapping::from(self.options.orientation);
        if mapping.reverse_columns {
            offset.0 = M::FRAMEBUFFER_SIZE
                .0
                .saturating_sub(self.options.display_size.0.saturating_add(offset.0));
        }
        if mapping.reverse_rows {
            offset.1 = M::FRAMEBUFFER_SIZE
                .1
                .saturating_sub(self.options.display_size.1.saturating_add(offset.1));
        }
        if mapping.swap_rows_and_columns {
            offset = (offset.1, offset.0);
        }
        let (final_sx, final_sy, final_ex, final_ey) = (
            sx.saturating_add(offset.0),
            sy.saturating_add(offset.1),
            ex.saturating_add(offset.0),
            ey.saturating_add(offset.1),
        );

        // M::update_address_window is an associated function on the Model trait
        M::update_address_window(
            &mut self.di,
            self.options.orientation.rotation,
            final_sx,
            final_sy,
            final_ex,
            final_ey,
        )
        .await
    }

    /// Configures the tearing effect output signal.
    pub async fn set_tearing_effect(
        &mut self,
        tearing_effect: options::TearingEffect,
    ) -> Result<(), DI::Error> {
        M::set_tearing_effect(&mut self.di, tearing_effect, &self.options).await
    }

    /// Returns `true` if the display is currently in sleep mode.
    pub fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    /// Puts the display into sleep mode.
    pub async fn sleep<DLY: AsyncDelayNs>(&mut self, delay: &mut DLY) -> Result<(), DI::Error> {
        M::sleep(&mut self.di, delay).await?;
        self.sleeping = true;
        Ok(())
    }

    /// Wakes the display from sleep mode.
    pub async fn wake<DLY: AsyncDelayNs>(&mut self, delay: &mut DLY) -> Result<(), DI::Error> {
        M::wake(&mut self.di, delay).await?;
        self.sleeping = false;
        Ok(())
    }

    /// Returns a mutable reference to the underlying display interface for sending raw commands.
    /// # Safety
    /// (User responsible for not desynchronizing state)
    pub unsafe fn raw_interface_mut(&mut self) -> &mut DI {
        &mut self.di
    }
}
