use serial::SerialPortSettings;
use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct MockSerial {
    pub tx_buffer: Arc<Mutex<VecDeque<u8>>>,
    pub rx_buffer: Arc<Mutex<VecDeque<u8>>>,
}

impl MockSerial {
    pub fn new() -> MockSerial {
        MockSerial {
            tx_buffer: Arc::new(Mutex::new(VecDeque::new())),
            rx_buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn take_tx_buff_as_string(&self) -> String {
        let mut tx_buff = self.tx_buffer.lock().unwrap();
        String::from_utf8(tx_buff.drain(..).collect()).unwrap()
    }

    pub fn insert_to_rx_buffer(&self, text: impl AsRef<str>) {
        let text = text.as_ref();
        let mut rx = self.rx_buffer.lock().unwrap();
        rx.extend(text.as_bytes());
    }
}

impl std::io::Read for MockSerial {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.rx_buffer.lock().unwrap().read(buf)
    }
}

impl std::io::Write for MockSerial {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tx_buffer.lock().unwrap().write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // No need for this to do anything for the mock.
        Ok(())
    }
}

impl serial_core::SerialPort for MockSerial {
    fn configure(&mut self, _settings: &serial::PortSettings) -> serial::Result<()> {
        unimplemented!()
    }

    fn read_cd(&mut self) -> serial::Result<bool> {
        unimplemented!()
    }

    fn read_cts(&mut self) -> serial::Result<bool> {
        unimplemented!()
    }

    fn read_dsr(&mut self) -> serial::Result<bool> {
        unimplemented!()
    }

    fn read_ri(&mut self) -> serial::Result<bool> {
        unimplemented!()
    }

    fn reconfigure(
        &mut self,
        _setup: &dyn Fn(&mut dyn SerialPortSettings) -> serial::Result<()>,
    ) -> serial::Result<()> {
        unimplemented!()
    }

    fn set_dtr(&mut self, _level: bool) -> serial::Result<()> {
        unimplemented!()
    }

    fn set_rts(&mut self, _level: bool) -> serial::Result<()> {
        unimplemented!()
    }

    fn set_timeout(&mut self, _timeout: std::time::Duration) -> serial::Result<()> {
        unimplemented!()
    }

    fn timeout(&self) -> std::time::Duration {
        unimplemented!()
    }
}

#[test]
fn write() {
    let mut mock_serial = MockSerial::new();
    mock_serial.write_all("Test123".as_bytes()).unwrap();
    assert_eq!(mock_serial.take_tx_buff_as_string(), "Test123".to_string());
}

#[test]
fn write_buff_clear() {
    let mut mock_serial = MockSerial::new();
    mock_serial.write_all("Test123".as_bytes()).unwrap();
    assert_eq!(mock_serial.take_tx_buff_as_string(), "Test123".to_string());
    assert_eq!(mock_serial.take_tx_buff_as_string(), "".to_string());
}

#[test]
fn read() {
    let mut mock_serial = MockSerial::new();
    mock_serial.insert_to_rx_buffer("Test123");

    let mut buff = String::new();
    _ = mock_serial.read_to_string(&mut buff).unwrap();
    assert_eq!(buff, "Test123".to_string());
}
