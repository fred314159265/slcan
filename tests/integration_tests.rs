mod mock_serial;

use mock_serial::MockSerial;
use slcan::{BitRate, CanFrame, ExtendedId, Id, StandardId};

#[test]
fn open() {
    let mock_serial = MockSerial::new();
    let mut slcan = slcan::CanSocket::new(mock_serial.clone());

    slcan.open(BitRate::Setup500Kbit).unwrap();
    assert_eq!(mock_serial.take_tx_buff_as_string(), "S6\rO\r".to_string());
}

#[test]
fn close() {
    let mock_serial = MockSerial::new();
    let mut slcan = slcan::CanSocket::new(mock_serial.clone());

    slcan.close().unwrap();
    assert_eq!(mock_serial.take_tx_buff_as_string(), "C\r".to_string());
}

#[test]
fn write_std() {
    let mock_serial = MockSerial::new();
    let mut slcan = slcan::CanSocket::new(mock_serial.clone());

    slcan
        .write(
            Id::Standard(StandardId::new(0x123).unwrap()),
            &[0xB, 0xE, 0xE, 0xF],
        )
        .unwrap();
    assert_eq!(
        mock_serial.take_tx_buff_as_string(),
        "t12340B0E0E0F\r".to_string()
    );

    slcan
        .write(Id::Standard(StandardId::new(0xBC).unwrap()), &[])
        .unwrap();
    assert_eq!(mock_serial.take_tx_buff_as_string(), "t0BC0\r".to_string());
}

#[test]
fn write_ext() {
    let mock_serial = MockSerial::new();
    let mut slcan = slcan::CanSocket::new(mock_serial.clone());

    slcan
        .write(
            Id::Extended(ExtendedId::new(0x1ABC_DEF1).unwrap()),
            &[0xB, 0xE, 0xE, 0xF],
        )
        .unwrap();
    assert_eq!(
        mock_serial.take_tx_buff_as_string(),
        "T1ABCDEF140B0E0E0F\r".to_string()
    );

    slcan
        .write(Id::Extended(ExtendedId::new(0x1ABC_DEF1).unwrap()), &[])
        .unwrap();
    assert_eq!(
        mock_serial.take_tx_buff_as_string(),
        "T1ABCDEF10\r".to_string()
    );
}

#[test]
fn read_std() {
    let mock_serial = MockSerial::new();
    let mut slcan = slcan::CanSocket::new(mock_serial.clone());

    mock_serial.insert_to_rx_buffer("t12340B0E0E0F\r");
    let ideal = CanFrame::new(
        Id::Standard(StandardId::new(0x123).unwrap()),
        4,
        &[0xB, 0xE, 0xE, 0xF],
    );
    assert_eq!(slcan.read().unwrap(), ideal);

    // Check buffer is now empty.
    assert!(slcan.read().is_err());

    mock_serial.insert_to_rx_buffer("t0BC0\r");
    let ideal = CanFrame::new(Id::Standard(StandardId::new(0xBC).unwrap()), 0, &[]);
    assert_eq!(slcan.read().unwrap(), ideal);
}

#[test]
fn read_ext() {
    let mock_serial = MockSerial::new();
    let mut slcan = slcan::CanSocket::new(mock_serial.clone());

    mock_serial.insert_to_rx_buffer("T1ABCDEF140B0E0E0F\r");
    let ideal = CanFrame::new(
        Id::Extended(ExtendedId::new(0x1ABC_DEF1).unwrap()),
        4,
        &[0xB, 0xE, 0xE, 0xF],
    );
    assert_eq!(slcan.read().unwrap(), ideal);

    // Check buffer is now empty.
    assert!(slcan.read().is_err());

    mock_serial.insert_to_rx_buffer("T1ABCDEF10\r");
    let ideal = CanFrame::new(Id::Extended(ExtendedId::new(0x1ABC_DEF1).unwrap()), 0, &[]);
    assert_eq!(slcan.read().unwrap(), ideal);
}
