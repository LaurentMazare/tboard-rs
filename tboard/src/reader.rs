// https://github.com/google/tsl/blob/2a6d8ef9f36c70eed0fe6400b248160d95afb817/tsl/lib/io/record_writer.cc#L99
use crate::{masked_crc, tensorboard, Result};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use prost::Message;

pub struct SummaryReader<R: std::io::Read> {
    reader: R,
    buf_len: [u8; 8],
    buf: Vec<u8>,
}

impl<R: std::io::Read> SummaryReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader, buf_len: Default::default(), buf: vec![0u8, 128] }
    }
}

impl<R: std::io::Read> Iterator for SummaryReader<R> {
    type Item = Result<tensorboard::Event>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_exact(&mut self.buf_len) {
            Ok(()) => {}
            Err(err) => {
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    return None;
                }
                return Some(Err(err.into()));
            }
        };
        let computed_crc = masked_crc(&self.buf_len);
        let event_len = LittleEndian::read_u64(&self.buf_len);
        let file_crc = match self.reader.read_u32::<LittleEndian>() {
            Ok(crc) => crc,
            Err(err) => return Some(Err(err.into())),
        };
        if file_crc != computed_crc {
            return Some(Err(crate::Error::LenCrcMismatch { file_crc, computed_crc }));
        }
        self.buf.resize(event_len as usize, 0u8);
        match self.reader.read_exact(&mut self.buf) {
            Ok(()) => {}
            Err(err) => return Some(Err(err.into())),
        }
        let event = match tensorboard::Event::decode(self.buf.as_slice()) {
            Ok(event) => event,
            Err(err) => return Some(Err(err.into())),
        };
        let file_crc = match self.reader.read_u32::<LittleEndian>() {
            Ok(crc) => crc,
            Err(err) => return Some(Err(err.into())),
        };
        let computed_crc = masked_crc(&self.buf);
        if file_crc != computed_crc {
            return Some(Err(crate::Error::CrcMismatch { file_crc, computed_crc }));
        }
        Some(Ok(event))
    }
}
