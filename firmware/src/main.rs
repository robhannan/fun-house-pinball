//! Firmware entry point: wires the Arduino Mega 2560's I/O to the game logic in
//! `pinball-core`. Keep this layer thin — rules belong in the core crate where
//! they can be tested on the host.

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod clock;
mod io;

use arduino_hal::hal::wdt;
use arduino_hal::prelude::*;
use arduino_hal::spi;
use pinball_core::sound::{DFPLAYER_VOLUME, dfplayer_play, dfplayer_volume};
use pinball_core::{Instant, Machine, Outputs, SwitchStates};

use io::{OutputChain, SoundOut, SwitchScanner};

/// The DFPlayer needs a moment after power-on before it accepts commands.
const SOUND_STARTUP_MS: u32 = 2_000;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // Hold the output registers disabled before anything else happens.
    let output_enable = pins.d48.into_output_high();

    let (spi, _) = arduino_hal::Spi::new(
        dp.SPI,
        pins.d52.into_output(),
        pins.d51.into_output(),
        pins.d50.into_pull_up_input(),
        pins.d53.into_output(),
        // The 74HC595 samples data on the rising clock edge, which needs SPI
        // mode 0 (avr-hal defaults to mode 1). 1 MHz leaves margin for long
        // wires to the playfield.
        spi::Settings {
            mode: embedded_hal::spi::MODE_0,
            clock: spi::SerialClockRate::OscfOver16,
            ..spi::Settings::default()
        },
    );
    let mut outputs = OutputChain {
        spi,
        latch: pins.d49.into_output().downgrade(),
        output_enable: output_enable.downgrade(),
    };
    // Clear the registers' random power-on contents before enabling them.
    outputs.write(&Outputs::default());
    outputs.enable();

    let mut switches = SwitchScanner {
        columns: [
            pins.d22.into_output_high().downgrade(),
            pins.d23.into_output_high().downgrade(),
            pins.d24.into_output_high().downgrade(),
            pins.d25.into_output_high().downgrade(),
            pins.d26.into_output_high().downgrade(),
            pins.d27.into_output_high().downgrade(),
            pins.d28.into_output_high().downgrade(),
            pins.d29.into_output_high().downgrade(),
        ],
        rows: [
            pins.d30.into_pull_up_input().downgrade(),
            pins.d31.into_pull_up_input().downgrade(),
            pins.d32.into_pull_up_input().downgrade(),
            pins.d33.into_pull_up_input().downgrade(),
            pins.d34.into_pull_up_input().downgrade(),
            pins.d35.into_pull_up_input().downgrade(),
            pins.d36.into_pull_up_input().downgrade(),
            pins.d37.into_pull_up_input().downgrade(),
        ],
        direct: [
            pins.d38.into_pull_up_input().downgrade(),
            pins.d39.into_pull_up_input().downgrade(),
            pins.d40.into_pull_up_input().downgrade(),
            pins.d41.into_pull_up_input().downgrade(),
            pins.d42.into_pull_up_input().downgrade(),
            pins.d43.into_pull_up_input().downgrade(),
            pins.d44.into_pull_up_input().downgrade(),
            pins.d45.into_pull_up_input().downgrade(),
        ],
    };

    // DFPlayer Mini sound module on the second serial port: TX1 (D18) to its
    // RX through a 1 kΩ resistor. Its TX isn't needed, but RX1 (D19) is wired
    // for status in future.
    let mut sound = SoundOut::new(arduino_hal::Usart::new(
        dp.USART1,
        pins.d19,
        pins.d18.into_output(),
        9600.into_baudrate(),
    ));
    let mut sound_ready = false;

    // If the main loop ever stalls, reset the board (which drops OE and with
    // it every coil) rather than leave a coil energised.
    let mut watchdog = wdt::Wdt::new(dp.WDT, dp.CPU.mcusr());
    watchdog.start(wdt::Timeout::Ms64).unwrap();

    clock::init(dp.TC0);
    // SAFETY: all interrupt-shared state is behind `interrupt::Mutex`.
    unsafe { avr_device::interrupt::enable() };

    ufmt::uwriteln!(&mut serial, "Fun House controller ready\r").unwrap_infallible();

    let mut machine = Machine::new();
    let mut raw = SwitchStates::new();
    let mut last_tick = clock::now();

    loop {
        let now = clock::now();
        if now == last_tick {
            continue;
        }
        last_tick = now;

        switches.scan(&mut raw);
        let frame = machine.tick(now, &raw);
        outputs.write(&frame);

        // Sounds only once the volume command has been queued (in an earlier
        // tick), so a sound can't replace it.
        if let (true, Some(effect)) = (sound_ready, frame.sound) {
            sound.send(dfplayer_play(effect.track()));
        }
        if !sound_ready && now.reached(Instant::from_millis(SOUND_STARTUP_MS)) {
            sound.send(dfplayer_volume(DFPLAYER_VOLUME));
            sound_ready = true;
        }
        sound.poll();

        watchdog.feed();
    }
}

/// On panic, kill every output first, then report and halt.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    avr_device::interrupt::disable();

    // SAFETY: nothing else runs after a panic, so stealing the peripherals
    // cannot race with `main`.
    let dp = unsafe { arduino_hal::Peripherals::steal() };
    let pins = arduino_hal::pins!(dp);
    pins.d48.into_output_high();

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    ufmt::uwriteln!(&mut serial, "PANIC: outputs disabled\r").unwrap_infallible();

    // The watchdog will reset us; blink the on-board LED until it does.
    let mut led = pins.d13.into_output();
    loop {
        led.toggle();
        arduino_hal::delay_ms(100);
    }
}
