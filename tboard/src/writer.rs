use crate::{masked_crc, tensorboard, Result};
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use prost::Message;

fn global_uid() -> u64 {
    // https://users.rust-lang.org/t/idiomatic-rust-way-to-generate-unique-id/33805
    use std::sync::atomic;
    static COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);
    COUNTER.fetch_add(1, atomic::Ordering::Relaxed)
}

/// Similar to tensorboard EventFileWriter
pub struct EventWriter<W: std::io::Write> {
    writer: W,
    buf_len: [u8; 8],
    buf: Vec<u8>,
    filename: Option<std::path::PathBuf>,
}

impl EventWriter<std::fs::File> {
    /// Create an `EventFileWriter` like structure in the specified log directory.
    pub fn create<P: AsRef<std::path::Path>>(logdir: P) -> Result<Self> {
        let logdir = logdir.as_ref();
        if logdir.is_file() {
            let logdir = logdir.canonicalize();
            crate::bail!("{logdir:?} is not a directory")
        }
        if !logdir.exists() {
            std::fs::create_dir_all(logdir)?
        }
        // https://github.com/tensorflow/tensorboard/blob/d1ab6e7a39e4dc4d556a8a73c0ae5c1b116801ba/tensorboard/summary/writer/event_file_writer.py#L76
        let now = std::time::SystemTime::now();
        let now = now.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        let hostname = hostname::get()?;
        let hostname = hostname.to_string_lossy();
        let pid = std::process::id();
        let uid = global_uid();
        let filename = logdir.join(format!("events.out.tfevents.{now:010}.{hostname}.{pid}.{uid}"));
        let file = std::fs::File::create(&filename)?;
        Self::from_writer(file, Some(filename))
    }
}

impl<W: std::io::Write> EventWriter<W> {
    pub fn from_writer(writer: W, filename: Option<std::path::PathBuf>) -> Result<Self> {
        let mut slf = Self { writer, buf_len: Default::default(), buf: vec![0u8, 128], filename };
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
        self.buf.truncate(0);
        event.encode(&mut self.buf)?;
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
            node_name: "".to_string(),
            tag: name.to_string(),
            metadata: None,
            value: Some(tensorboard::summary::value::Value::SimpleValue(value)),
        };
        let what = tensorboard::event::What::Summary(tensorboard::Summary { value: vec![value] });
        self.write(step, what)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    pub fn filename(&self) -> Option<&std::path::PathBuf> {
        self.filename.as_ref()
    }
}
