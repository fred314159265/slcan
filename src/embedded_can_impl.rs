use crate::{CanFrame, CanSocket};
use embedded_can::blocking::Can;
use embedded_can::{Frame, Id};
use serial_core::SerialPort;

impl embedded_can::Error for crate::Error {
    fn kind(&self) -> embedded_can::ErrorKind {
        // SLCAN doesn't support reporting CAN specific errors.
        embedded_can::ErrorKind::Other
    }
}

impl Frame for CanFrame {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        Some(CanFrame::new(id.into(), data.len(), data))
    }

    fn new_remote(_id: impl Into<Id>, _dlc: usize) -> Option<Self> {
        // currently unsupported
        None
    }

    fn is_extended(&self) -> bool {
        matches!(self.id, Id::Extended(_))
    }

    fn is_remote_frame(&self) -> bool {
        // currently unsupported
        false
    }

    fn id(&self) -> Id {
        self.id
    }

    fn dlc(&self) -> usize {
        self.dlc
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}

impl<P: SerialPort> Can for CanSocket<P> {
    type Frame = CanFrame;
    type Error = crate::Error;

    fn transmit(&mut self, frame: &Self::Frame) -> Result<(), Self::Error> {
        _ = self.write(frame.id, frame.data())?;
        Ok(())
    }

    fn receive(&mut self) -> Result<Self::Frame, Self::Error> {
        let frame = self.read()?;
        Ok(frame)
    }
}
