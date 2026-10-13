//! Physical I/O: switch matrix scanning and the output shift-register chain.
//!
//! Pin assignments here must match `docs/hardware.md`.

use arduino_hal::port::Pin;
use arduino_hal::port::mode::{Input, Output, PullUp};
use embedded_hal::spi::SpiBus;
use pinball_core::{Outputs, SwitchStates};

/// Reads the 8×8 switch matrix plus 8 direct inputs.
///
/// Columns are strobed low one at a time; rows have pull-ups, so a closed
/// switch (with its isolation diode) reads low on its row.
pub struct SwitchScanner {
    pub columns: [Pin<Output>; 8],
    pub rows: [Pin<Input<PullUp>>; 8],
    pub direct: [Pin<Input<PullUp>>; 8],
}

impl SwitchScanner {
    /// Scans every switch into `states`.
    pub fn scan(&mut self, states: &mut SwitchStates) {
        for (c, column) in self.columns.iter_mut().enumerate() {
            column.set_low();
            // Let the lines settle before sampling.
            arduino_hal::delay_us(5);
            states.set_column(c as u8, read_active_low(&self.rows));
            column.set_high();
        }
        states.set_direct(read_active_low(&self.direct));
    }
}

fn read_active_low(pins: &[Pin<Input<PullUp>>; 8]) -> u8 {
    pins.iter()
        .enumerate()
        .fold(0, |bits, (i, pin)| bits | (u8::from(pin.is_low()) << i))
}

/// A daisy chain of 74HC595 shift registers driving MOSFET coil drivers
/// (4 bytes) and lamp drivers (8 bytes).
///
/// `OE` is held high by a pull-up resistor on the board, so every output stays
/// off from power-on until the firmware explicitly enables the chain.
pub struct OutputChain {
    pub spi: arduino_hal::Spi,
    pub latch: Pin<Output>,
    pub output_enable: Pin<Output>,
}

impl OutputChain {
    /// Shifts out a full frame and latches it.
    pub fn write(&mut self, outputs: &Outputs) {
        // The last byte shifted ends up in the first register in the chain:
        // coils sit nearest the Arduino, lamps further down.
        let mut frame = [0u8; 12];
        frame[..8].copy_from_slice(&outputs.lamps.to_be_bytes());
        frame[8..].copy_from_slice(&outputs.coils.to_be_bytes());
        // The AVR SPI driver is infallible.
        let _ = self.spi.write(&frame);
        let _ = self.spi.flush();
        self.latch.set_high();
        self.latch.set_low();
    }

    /// Lets the registers drive their outputs.
    pub fn enable(&mut self) {
        self.output_enable.set_low();
    }
}
