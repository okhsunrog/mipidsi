//! [super::Display] builder module

use embedded_hal::digital::{self, OutputPin as BlockingOutputPin};
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;

use crate::{
    interface::Interface, // Removed InterfacePixelFormat
    models::{Model, ModelInitError},
    options::{ColorInversion, ColorOrder, ModelOptions, Orientation, RefreshOrder},
    Display, // Removed dcs::SetAddressMode from here, it's used as a type
};

/// Builder for [Display] instances.
pub struct Builder<DI, MODEL, RST>
where
    DI: Interface,
    MODEL: Model, // No ColorFormat bound here
{
    di: DI,
    model: MODEL,
    rst: Option<RST>,
    options: ModelOptions,
}

impl<DI, MODEL> Builder<DI, MODEL, NoResetPin>
where
    DI: Interface,
    MODEL: Model,
{
    #[must_use]
    pub fn new(model: MODEL, di: DI) -> Self {
        Self {
            di,
            model,
            rst: None,
            options: ModelOptions::full_size::<MODEL>(),
        }
    }
}

impl<DI, MODEL, RST> Builder<DI, MODEL, RST>
where
    DI: Interface,
    MODEL: Model,
    RST: BlockingOutputPin,
{
    #[must_use]
    pub fn invert_colors(mut self, color_inversion: ColorInversion) -> Self {
        self.options.invert_colors = color_inversion;
        self
    }
    #[must_use]
    pub fn color_order(mut self, color_order: ColorOrder) -> Self {
        self.options.color_order = color_order;
        self
    }
    #[must_use]
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.options.orientation = orientation;
        self
    }
    #[must_use]
    pub fn refresh_order(mut self, refresh_order: RefreshOrder) -> Self {
        self.options.refresh_order = refresh_order;
        self
    }
    #[must_use]
    pub fn display_size(mut self, width: usize, height: usize) -> Self {
        self.options.display_size = (width as u16, height as u16);
        self
    }
    #[must_use]
    pub fn display_offset(mut self, x: u16, y: u16) -> Self {
        self.options.display_offset = (x, y);
        self
    }

    #[must_use]
    pub fn reset_pin<RST2: BlockingOutputPin>(self, rst: RST2) -> Builder<DI, MODEL, RST2> {
        Builder {
            di: self.di,
            model: self.model,
            rst: Some(rst),
            options: self.options,
        }
    }

    pub async fn init(
        mut self,
        delay_source: &mut impl AsyncDelayNs,
    ) -> Result<Display<DI, MODEL, RST>, InitError<DI::Error, RST::Error>> {
        let to_u32 = |(a, b)| (u32::from(a), u32::from(b));
        let (width, height) = to_u32(self.options.display_size);
        let (offset_x, offset_y) = to_u32(self.options.display_offset);
        let (max_width, max_height) = to_u32(MODEL::FRAMEBUFFER_SIZE);

        if width == 0 || height == 0 || width > max_width || height > max_height {
            return Err(InitError::InvalidConfiguration(
                ConfigurationError::InvalidDisplaySize,
            ));
        }
        if width + offset_x > max_width || height + offset_y > max_height {
            return Err(InitError::InvalidConfiguration(
                ConfigurationError::InvalidDisplayOffset,
            ));
        }

        if let Some(ref mut rst_pin) = self.rst {
            rst_pin.set_low().map_err(InitError::ResetPin)?;
            delay_source.delay_us(MODEL::RESET_DURATION).await;
            rst_pin.set_high().map_err(InitError::ResetPin)?;
            delay_source.delay_us(10_000).await;
        } else {
            // Directly send the SoftReset DCS command via InterfaceExt
            use crate::dcs::InterfaceExt; // Ensure this is in scope
            self.di
                .write_command(crate::dcs::SoftReset)
                .await
                .map_err(InitError::Interface)?;
        }

        let madctl = self
            .model
            .init(&mut self.di, delay_source, &self.options)
            .await
            .map_err(|model_err| match model_err {
                ModelInitError::Interface(e) => InitError::Interface(e),
                ModelInitError::InvalidConfiguration(c) => InitError::InvalidConfiguration(c),
            })?;

        Ok(Display {
            di: self.di,
            model: self.model,
            rst: self.rst,
            options: self.options,
            madctl, // This is crate::dcs::SetAddressMode type
            sleeping: false,
        })
    }
}

#[derive(Debug)]
pub enum InitError<DIError, PinError> {
    Interface(DIError),
    ResetPin(PinError),
    InvalidConfiguration(ConfigurationError),
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationError {
    UnsupportedInterface,
    InvalidDisplaySize,
    InvalidDisplayOffset,
}

impl<DIError, PinError> From<ModelInitError<DIError>> for InitError<DIError, PinError> {
    fn from(value: ModelInitError<DIError>) -> Self {
        match value {
            ModelInitError::Interface(e) => InitError::Interface(e),
            ModelInitError::InvalidConfiguration(ce) => InitError::InvalidConfiguration(ce),
        }
    }
}

pub enum NoResetPin {}
impl digital::OutputPin for NoResetPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
impl digital::ErrorType for NoResetPin {
    type Error = core::convert::Infallible;
}
