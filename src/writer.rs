use crate::{masked_crc, tensorboard, Result};
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use prost::Message;

pub struct SummaryWriter<W: std::io::Write> {
    writer: W,
    buf_len: [u8; 8],
    buf: Vec<u8>,
}

impl<W: std::io::Write> SummaryWriter<W> {
    pub fn new(writer: W) -> Result<Self> {
        let mut slf = Self { writer, buf_len: Default::default(), buf: vec![0u8, 128] };
        slf.write(0, tensorboard::event::What::FileVersion("brain.Event:2".to_string()))?;
        Ok(slf)
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

    pub fn write(&mut self, step: i64, what: tensorboard::event::What) -> Result<()> {
        let now = std::time::SystemTime::now();
        let now = now.duration_since(std::time::UNIX_EPOCH)?;
        let wall_time = now.as_secs() as f64 + now.subsec_nanos() as f64 / 1e9;
        self.write_event(tensorboard::Event {
            wall_time,
            step,
            source_metadata: None,
            what: Some(what),
        })
    }

    pub fn write_scalar(&mut self, step: i64, name: &str, value: f32) -> Result<()> {
        let value = tensorboard::summary::Value {
            node_name: name.to_string(),
            tag: "".to_string(),
            metadata: None,
            value: Some(tensorboard::summary::value::Value::SimpleValue(value)),
        };
        let what = tensorboard::event::What::Summary(tensorboard::Summary { value: vec![value] });
        self.write(step, what)
    }
}
