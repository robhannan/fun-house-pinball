//! Firmware entry point: wires the Arduino Mega 2560's I/O to the game logic in
//! `pinball-core`. Keep this layer thin — rules belong in the core crate where
//! they can be tested on the host.

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod clock;
mod io;
mod servo;

use arduino_hal::hal::wdt;
use arduino_hal::prelude::*;
use arduino_hal::spi;
use pinball_core::flaps::FlapTrim;
use pinball_core::sound::{
    WAV_TRIGGER_GAIN_DB, wav_trigger_amp, wav_trigger_gain, wav_trigger_play, wav_trigger_stop,
};
use pinball_core::test_mode::describe;
use pinball_core::{Instant, Machine, Outputs, Phase, SwitchStates};

use io::{ConsoleOut, OutputChain, SoundOut, SwitchScanner};
use servo::FlapServos;

/// Where the trapdoor flaps' trimmed angles are kept in the EEPROM.
const FLAP_TRIM_ADDRESS: u16 = 0;

/// The WAV Trigger indexes its SD card at power-on before it accepts commands.
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

    // WAV Trigger sound board on the second serial port at its default 57600
    // baud: TX1 (D18) to its RX through a 1 kΩ resistor. Its TX isn't needed,
    // but RX1 (D19) is wired for status in future.
    let mut sound = SoundOut::new(arduino_hal::Usart::new(
        dp.USART1,
        pins.d19,
        pins.d18.into_output(),
        57600.into_baudrate(),
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
    // From here on, the USB serial port only gets queued, non-blocking output:
    // test-mode reports (57600 baud, CR LF line endings).
    let mut console = ConsoleOut::new(serial);

    let mut machine = Machine::new();

    // The trapdoor flaps' angles, as last set in test mode. Blank or corrupt
    // EEPROM gives the defaults.
    let mut eeprom = arduino_hal::Eeprom::new(dp.EEPROM);
    let mut stored = [0xFF; FlapTrim::BYTES];
    let _ = eeprom.read(FLAP_TRIM_ADDRESS, &mut stored);
    let mut saved_trim = FlapTrim::from_bytes(&stored);
    machine.set_flap_trim(saved_trim);

    // Trapdoor flap servos on Timer 3 (see `servo.rs`), starting closed.
    let mut flaps = FlapServos::new(
        dp.TC3,
        [
            pins.d5.into_output().downgrade(),
            pins.d2.into_output().downgrade(),
            pins.d3.into_output().downgrade(),
        ],
        [0, 1, 2].map(|n| saved_trim.angle(n, false)),
    );

    let mut raw = SwitchStates::new();
    let mut last_tick = clock::now();

    loop {
        let now = clock::now();
        if now == last_tick {
            // Keep the serial ports busy while waiting.
            sound.poll();
            console.poll();
            continue;
        }
        last_tick = now;

        switches.scan(&mut raw);
        let frame = machine.tick(now, &raw);
        outputs.write(&frame);
        flaps.set(frame.flap_angles);

        // Save new flap angles once test mode is left. Writing the EEPROM
        // takes about 3.4 ms a byte, but only happens in attract mode, with
        // coil power off, and well inside the watchdog's 64 ms.
        let trim = machine.flap_trim();
        if trim != saved_trim && machine.phase() == Phase::Attract {
            let bytes = trim.to_bytes();
            for (offset, &byte) in (FLAP_TRIM_ADDRESS..).zip(&bytes) {
                if eeprom.read_byte(offset) != byte {
                    // The EEPROM's two-step write must not be interrupted, or
                    // the byte is silently dropped. This holds off the 1 ms
                    // clock for one byte's write, which only delays a tick.
                    avr_device::interrupt::free(|_| eeprom.write_byte(offset, byte));
                }
            }
            let mut check = [0; FlapTrim::BYTES];
            let _ = eeprom.read(FLAP_TRIM_ADDRESS, &mut check);
            saved_trim = trim;
            console.send_line(if check == bytes {
                b"flap angles saved"
            } else {
                b"flap angles NOT saved: EEPROM write failed"
            });
        }

        if sound_ready {
            for effect in frame.stop_sounds.iter() {
                sound.send(&wav_trigger_stop(u16::from(effect.track())));
            }
            for effect in frame.sounds.iter() {
                sound.send(&wav_trigger_play(
                    u16::from(effect.track()),
                    effect.locked(),
                ));
            }
        } else if now.reached(Instant::from_millis(SOUND_STARTUP_MS)) {
            sound.send(&wav_trigger_amp(true));
            sound.send(&wav_trigger_gain(WAV_TRIGGER_GAIN_DB));
            sound_ready = true;
        }
        sound.poll();

        for report in frame.reports.iter() {
            console.send_line(describe(report).as_bytes());
        }
        console.poll();

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
