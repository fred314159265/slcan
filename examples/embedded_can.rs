//! Basic example using the serial crate for the serial communications, which continuously
//! reads frames from the SLCAN device and prints them to the console.
//!
//! Example execution using COM1 on Windows:
//! ```
//! cargo r --example embedded_can COM1
//! ```

use embedded_can::blocking::Can;
use std::io::ErrorKind;

fn main() {
    let arg = std::env::args().nth(1);
    let port = match arg {
        Some(port_name) => {
            println!("Opening serial port: {}", port_name);
            serial::open(&port_name)
        }
        None => {
            eprintln!("usage: macos_example <TTY path>");
            std::process::exit(1);
        }
    }
    .unwrap();
    let mut can = slcan::CanSocket::<serial::SystemPort>::new(port);

    can.close().unwrap();
    can.open(slcan::BitRate::Setup1Mbit).unwrap();

    loop {
        match can.receive() {
            Ok(frame) => println!("{}", frame),
            Err(e) if e.kind() == ErrorKind::WouldBlock => (),
            Err(e) if e.kind() == ErrorKind::TimedOut => (),
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }
}
