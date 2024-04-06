extern crate serial_core as serial;

pub use embedded_can::{ExtendedId, Id, StandardId};
use serial::prelude::*;
#[cfg(target_family = "unix")]
use std::os::unix::prelude::AsRawFd;
use std::{
    convert::{TryFrom, TryInto},
    io,
};

pub mod embedded_can_impl;

// maximum rx buffer len: extended CAN frame with timestamp
const SLCAN_MTU: usize = "T1111222281122334455667788EA5F\r".len() + 1;
const SLCAN_CMD_LEN: usize = 1;
const SLCAN_STANDARD_ID_LEN: usize = 3;
const SLCAN_EXTENDED_ID_LEN: usize = 8;

const BELL: u8 = 0x07;
const CARRIAGE_RETURN: u8 = b'\r';

const HEX_LUT: &[u8] = "0123456789ABCDEF".as_bytes();

#[repr(u8)]
pub enum BitRate {
    Setup10Kbit = b'0',
    Setup20Kbit = b'1',
    Setup50Kbit = b'2',
    Setup100Kbit = b'3',
    Setup125Kbit = b'4',
    Setup250Kbit = b'5',
    Setup500Kbit = b'6',
    Setup800Kbit = b'7',
    Setup1Mbit = b'8',
}

#[repr(u8)]
pub enum Command {
    /// Setup with standard CAN [bit rates](BitRate).
    Setup = b'S',
    /// Open the CAN channel in normal mode (sending & receiving).
    Open = b'O',
    /// Close the CAN channel.
    Close = b'C',
    /// Transmit a standard (11bit) CAN frame.
    TransmitStandardFrame = b't',
    /// Transmit an extended (29bit) CAN frame.
    TransmitExtendedFrame = b'T',
}

impl TryFrom<u8> for Command {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value as char {
            'S' => Ok(Command::Setup),
            'O' => Ok(Command::Open),
            'C' => Ok(Command::Close),
            't' => Ok(Command::TransmitStandardFrame),
            'T' => Ok(Command::TransmitExtendedFrame),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid Command",
            )),
        }
    }
}

pub struct CanFrame {
    pub id: Id,
    pub dlc: usize,
    pub data: [u8; 8],
}

pub struct CanSocket<P: SerialPort> {
    port: P,
    rbuff: [u8; SLCAN_MTU],
    rcount: usize,
    error: bool,
}

#[derive(Debug)]
pub struct Error(io::Error);

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error(value)
    }
}

#[cfg(target_family = "unix")]
impl<P> AsRawFd for CanSocket<P>
where
    P: SerialPort + AsRawFd,
{
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.port.as_raw_fd()
    }
}

fn hextou8(s: u8) -> Result<u8, ()> {
    let c = s as char;

    if c.is_ascii_digit() {
        Ok(s - b'0')
    } else if ('a'..='f').contains(&c) {
        Ok(s - b'a' + 10)
    } else if ('A'..='F').contains(&c) {
        Ok(s - b'A' + 10)
    } else {
        Err(())
    }
}

fn hex2tou8(s: &[u8]) -> Result<u8, ()> {
    let msn = hextou8(s[0])?;
    let lsn = hextou8(s[1])?;

    Ok((msn << 4) | lsn)
}

fn unpack_data(s: &[u8], len: usize) -> Result<[u8; 8], ()> {
    let mut buf = [u8::default(); 8];

    for i in 0..len {
        let offset = 2 * i;

        buf[i] = hex2tou8(&s[offset..])?;
    }

    Ok(buf)
}

fn hextou16(buf: &[u8]) -> Result<u16, ()> {
    let mut value = 0u16;

    for s in buf.iter() {
        value <<= 4;

        match hextou8(*s) {
            Ok(byte) => value |= byte as u16,
            Err(_) => return Err(()),
        }
    }

    Ok(value)
}

fn hextou32(buf: &[u8]) -> Result<u32, ()> {
    let mut value = 0u32;

    for s in buf.iter() {
        value <<= 4;

        match hextou8(*s) {
            Ok(byte) => value |= byte as u32,
            Err(_) => return Err(()),
        }
    }

    Ok(value)
}

fn hexdigit(value: u32) -> u8 {
    HEX_LUT[(value & 0xF) as usize]
}

fn u16tohex3(value: u16) -> [u8; 3] {
    [
        hexdigit(value as u32 >> 8),
        hexdigit(value as u32 >> 4),
        #[allow(clippy::identity_op)]
        hexdigit(value as u32 >> 0),
    ]
}

fn u32tohex8(value: u32) -> [u8; 8] {
    [
        hexdigit(value >> 28),
        hexdigit(value >> 24),
        hexdigit(value >> 20),
        hexdigit(value >> 16),
        hexdigit(value >> 12),
        hexdigit(value >> 8),
        hexdigit(value >> 4),
        #[allow(clippy::identity_op)]
        hexdigit(value >> 0),
    ]
}

fn bytestohex(data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::<u8>::with_capacity(2 * data.len());

    for byte in data {
        buf.push(hexdigit((byte >> 4) as u32));
        #[allow(clippy::identity_op)]
        buf.push(hexdigit((byte >> 0) as u32));
    }

    buf
}

impl CanFrame {
    pub fn new(id: Id, dlc: usize, data: &[u8]) -> Self {
        let mut copy = [u8::default(); 8];
        copy[..data.len()].copy_from_slice(data);

        Self {
            id,
            dlc,
            data: copy,
        }
    }
}

impl std::fmt::Display for CanFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CanFrame{{ id: {:?}, dlc: {}, data: {:?} }}",
            self.id, self.dlc, self.data
        )
    }
}

impl<P: SerialPort> CanSocket<P> {
    pub fn new(port: P) -> Self {
        CanSocket {
            port,
            rbuff: [0; SLCAN_MTU],
            rcount: 0,
            error: false,
        }
    }

    pub fn open(&mut self, bitrate: BitRate) -> io::Result<()> {
        self.port
            .write_all(&[Command::Setup as u8, bitrate as u8, b'\r'])?;
        self.port.write_all(&[Command::Open as u8, b'\r'])?;

        Ok(())
    }

    pub fn close(&mut self) -> io::Result<()> {
        self.port.write_all(&[Command::Close as u8, b'\r'])?;

        Ok(())
    }

    pub fn write(&mut self, id: Id, data: &[u8]) -> Result<usize, crate::Error> {
        let dlc = data.len();

        if dlc > 8 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "data length").into());
        }

        let mut buf = Vec::<u8>::with_capacity(6 + 2 * dlc);

        match id {
            Id::Standard(standard_id) => {
                buf.push(Command::TransmitStandardFrame as u8);
                buf.extend_from_slice(&u16tohex3(standard_id.as_raw()));
            }
            Id::Extended(extended_id) => {
                buf.push(Command::TransmitExtendedFrame as u8);
                buf.extend_from_slice(&u32tohex8(extended_id.as_raw()));
            }
        }

        buf.push(hexdigit(dlc as u32));
        buf.extend_from_slice(&bytestohex(data));
        buf.push(b'\r');

        let len = self.port.write(buf.as_slice())?;
        Ok(len)
    }

    pub fn read(&mut self) -> io::Result<CanFrame> {
        let mut buf = [0u8; 1];
        let mut len = self.port.read(&mut buf)?;

        while len == 1usize {
            let s = buf[0];

            if s == CARRIAGE_RETURN || s == BELL {
                let valid = !self.error && self.rcount > 4;

                self.error = false;
                self.rcount = 0;

                if valid {
                    return self.bump();
                }
            } else if !self.error {
                if self.rcount < SLCAN_MTU {
                    self.rbuff[self.rcount] = s;
                    self.rcount += 1;
                } else {
                    self.error = true;
                }
            }

            len = self.port.read(&mut buf)?;
        }

        Err(io::Error::new(io::ErrorKind::WouldBlock, ""))
    }

    fn bump(&mut self) -> io::Result<CanFrame> {
        let cmd = self.rbuff[0].try_into();

        match cmd {
            Ok(Command::TransmitStandardFrame) => {
                let id = match hextou16(
                    &self.rbuff[SLCAN_CMD_LEN..SLCAN_CMD_LEN + SLCAN_STANDARD_ID_LEN],
                ) {
                    Ok(value) => value,
                    Err(()) => return Err(io::Error::new(io::ErrorKind::WouldBlock, "")),
                };
                let dlc = (self.rbuff[SLCAN_CMD_LEN + SLCAN_STANDARD_ID_LEN] - 0x30) as usize;

                if let Ok(data) = unpack_data(
                    &self.rbuff[SLCAN_CMD_LEN + SLCAN_STANDARD_ID_LEN + 1..],
                    dlc,
                ) {
                    let standard_id = StandardId::new(id).ok_or(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Standard id exceeds max size",
                    ))?;
                    Ok(CanFrame {
                        id: Id::Standard(standard_id),
                        dlc,
                        data,
                    })
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidData, ""))
                }
            }
            Ok(Command::TransmitExtendedFrame) => {
                let id = match hextou32(
                    &self.rbuff[SLCAN_CMD_LEN..SLCAN_CMD_LEN + SLCAN_EXTENDED_ID_LEN],
                ) {
                    Ok(value) => value,
                    Err(()) => return Err(io::Error::new(io::ErrorKind::WouldBlock, "")),
                };
                let dlc = (self.rbuff[SLCAN_CMD_LEN + SLCAN_EXTENDED_ID_LEN] - 0x30) as usize;

                if let Ok(data) = unpack_data(
                    &self.rbuff[SLCAN_CMD_LEN + SLCAN_EXTENDED_ID_LEN + 1..],
                    dlc,
                ) {
                    let extended_id = ExtendedId::new(id).ok_or(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Extended id exceeds max size",
                    ))?;
                    Ok(CanFrame {
                        id: Id::Extended(extended_id),
                        dlc,
                        data,
                    })
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidData, ""))
                }
            }
            _ => Err(io::Error::new(io::ErrorKind::WouldBlock, "")),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
