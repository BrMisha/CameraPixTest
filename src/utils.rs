use std::sync::Mutex;
use std::time::Duration;
use mavlink::{
    common::{GpsFixType, MavMessage, ATTITUDE_DATA, REQUEST_DATA_STREAM_DATA},
    error::{MessageReadError, MessageWriteError},
    read_versioned_msg, write_versioned_msg, MavConnection, MavHeader, MavlinkVersion, Message,
};
use serialport::{DataBits, FlowControl, Parity, StopBits};
use log::{debug, error, info, trace};
use std::io;

pub struct SerialConnection {
    port: Mutex<Box<dyn serialport::SerialPort>>,
    sequence: Mutex<u8>,
    protocol_version: MavlinkVersion,
}

impl SerialConnection {
    pub fn new(path: &str, baud_rate: u32) -> io::Result<Self> {
        let port = serialport::new(path, baud_rate)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(Duration::from_millis(100))
            .open()?;

        Ok(Self {
            port: Mutex::new(port),
            sequence: Mutex::new(0),
            protocol_version: MavlinkVersion::V2,
        })
    }
}

impl<M: Message> MavConnection<M> for SerialConnection {
    fn recv(&self) -> Result<(MavHeader, M), MessageReadError> {
        let mut port = self.port.lock().unwrap();

        loop {
            match read_versioned_msg(&mut *port, self.protocol_version) {
                ok @ Ok(..) => {
                    return ok;
                }
                Err(MessageReadError::Parse(e)) => {
                    debug!("Mavlink error ignored: {e}");
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    fn send(&self, header: &MavHeader, data: &M) -> Result<usize, MessageWriteError> {
        let mut port = self.port.lock().unwrap();
        let mut sequence = self.sequence.lock().unwrap();

        let header = MavHeader {
            sequence: *sequence,
            system_id: header.system_id,
            component_id: header.component_id,
        };

        *sequence = sequence.wrapping_add(1);

        write_versioned_msg(&mut *port, self.protocol_version, header, data)
    }

    fn set_protocol_version(&mut self, version: MavlinkVersion) {
        self.protocol_version = version;
    }

    fn get_protocol_version(&self) -> MavlinkVersion {
        self.protocol_version
    }
}