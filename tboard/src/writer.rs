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

    pub fn write_scalar(&mut self, step: i64, tag: &str, value: f32) -> Result<()> {
        let value = tensorboard::summary::Value {
            node_name: "".to_string(),
            tag: tag.to_string(),
            metadata: None,
            value: Some(tensorboard::summary::value::Value::SimpleValue(value)),
        };
        let what = tensorboard::event::What::Summary(tensorboard::Summary { value: vec![value] });
        self.write(step, what)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_audio(
        &mut self,
        step: i64,
        tag: &str,
        content_type: &str,
        encoded_audio_string: Vec<u8>,
        length_frames: i64,
        num_channels: i64,
        sample_rate: f32,
    ) -> Result<()> {
        let audio = tensorboard::summary::Audio {
            content_type: content_type.to_string(),
            encoded_audio_string,
            length_frames,
            num_channels,
            sample_rate,
        };
        let value = tensorboard::summary::Value {
            node_name: "".to_string(),
            tag: tag.to_string(),
            metadata: None,
            value: Some(tensorboard::summary::value::Value::Audio(audio)),
        };
        let what = tensorboard::event::What::Summary(tensorboard::Summary { value: vec![value] });
        self.write(step, what)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_histo(
        &mut self,
        step: i64,
        tag: &str,
        min: f64,
        max: f64,
        num: f64,
        sum: f64,
        sum_squares: f64,
        bucket: Vec<f64>,
        bucket_limit: Vec<f64>,
    ) -> Result<()> {
        let histo =
            tensorboard::HistogramProto { bucket, bucket_limit, max, min, num, sum, sum_squares };
        let value = tensorboard::summary::Value {
            node_name: "".to_string(),
            tag: tag.to_string(),
            metadata: None,
            value: Some(tensorboard::summary::value::Value::Histo(histo)),
        };
        let what = tensorboard::event::What::Summary(tensorboard::Summary { value: vec![value] });
        self.write(step, what)
    }

    pub fn write_image(
        &mut self,
        step: i64,
        tag: &str,
        width: i32,
        height: i32,
        colorspace: i32,
        encoded_image_string: Vec<u8>,
    ) -> Result<()> {
        let image = tensorboard::summary::Image { width, height, colorspace, encoded_image_string };
        let value = tensorboard::summary::Value {
            node_name: "".to_string(),
            tag: tag.to_string(),
            metadata: None,
            value: Some(tensorboard::summary::value::Value::Image(image)),
        };
        let what = tensorboard::event::What::Summary(tensorboard::Summary { value: vec![value] });
        self.write(step, what)
    }

    pub fn write_tensor(&mut self, step: i64, tag: &str) -> Result<()> {
        let tensor = tensorboard::TensorProto {
            dtype: tensorboard::DataType::DtFloat.into(),
            tensor_shape: None,
            tensor_content: vec![],
            version_number: 0,
            bool_val: vec![],
            double_val: vec![],
            dcomplex_val: vec![],
            float_val: vec![],
            float8_val: vec![],
            half_val: vec![],
            int_val: vec![],
            int64_val: vec![],
            resource_handle_val: vec![],
            scomplex_val: vec![],
            string_val: vec![],
            uint32_val: vec![],
            uint64_val: vec![],
            variant_val: vec![],
        };
        let value = tensorboard::summary::Value {
            node_name: "".to_string(),
            tag: tag.to_string(),
            metadata: None,
            value: Some(tensorboard::summary::value::Value::Tensor(tensor)),
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
