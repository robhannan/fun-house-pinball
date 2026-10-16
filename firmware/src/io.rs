//! Physical I/O: switch matrix scanning and the output shift-register chain.
//!
//! Pin assignments here must match `docs/hardware.md`.

use arduino_hal::port::Pin;
use arduino_hal::port::mode::{Input, Output, PullUp};
use arduino_hal::prelude::_embedded_hal_serial_Write;
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

/// Sends commands to the WAV Trigger sound board without ever waiting.
///
/// Commands are queued, and [`SoundOut::poll`] sends whatever the serial port
/// will take right now. The main loop calls it while waiting for each tick too,
/// so at 57600 baud a 9-byte command goes out in about 1.6 ms. Coil pulses only
/// end when the loop runs, so it must never block on the serial port.
pub struct SoundOut<W> {
    serial: W,
    queue: [u8; QUEUE_LEN],
    head: usize,
    len: usize,
}

/// Room for seven play commands: more than can sensibly start in one tick.
const QUEUE_LEN: usize = 64;

impl<W: _embedded_hal_serial_Write<u8, Error = core::convert::Infallible>> SoundOut<W> {
    /// Wraps the serial port connected to the WAV Trigger's RX pin.
    pub fn new(serial: W) -> Self {
        Self {
            serial,
            queue: [0; QUEUE_LEN],
            head: 0,
            len: 0,
        }
    }

    /// Queues a command. Returns `false` (and drops it) if the queue is full,
    /// rather than splitting a command.
    pub fn send(&mut self, command: &[u8]) -> bool {
        if self.len + command.len() > QUEUE_LEN {
            return false;
        }
        for &byte in command {
            self.queue[(self.head + self.len) % QUEUE_LEN] = byte;
            self.len += 1;
        }
        true
    }

    /// Sends queued bytes until the serial port is busy or the queue is empty.
    pub fn poll(&mut self) {
        while self.len > 0 && self.serial.write(self.queue[self.head]).is_ok() {
            self.head = (self.head + 1) % QUEUE_LEN;
            self.len -= 1;
        }
    }
}
