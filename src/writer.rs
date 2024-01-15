use crate::{masked_crc, tensorboard, Result};
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use prost::Message;

pub struct SummaryWriter<W: std::io::Write> {
    writer: W,
    buf_len: [u8; 8],
    buf: Vec<u8>,
}

impl<W: std::io::Write> SummaryWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer, buf_len: Default::default(), buf: vec![0u8, 128] }
    }

    // https://github.com/LaurentMazare/ocaml-tensorboard/blob/11022591e15327f31595443d18e1f3e38cc0a433/src/tensorboard/tf_record_writer.ml#L25
    pub fn write_event(&mut self, event: tensorboard::Event) -> Result<()> {
        let event_len = event.encoded_len();
        LittleEndian::write_u64(&mut self.buf_len, event_len as u64);
        let buf_len_crc = masked_crc(self.buf_len.as_slice());
        self.writer.write_all(self.buf_len.as_slice())?;
        self.writer.write_u32::<LittleEndian>(buf_len_crc)?;
        self.buf.resize(event_len, 0u8);
        self.writer.write_all(self.buf.as_slice())?;
        let event_crc = masked_crc(self.buf.as_slice());
        self.writer.write_u32::<LittleEndian>(event_crc)?;
        Ok(())
    }
}
