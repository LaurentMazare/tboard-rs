mod error;
mod reader;
mod writer;
pub use error::{Error, Result};
pub use reader::SummaryReader;
pub use writer::EventWriter;

// Protobuf types.
pub mod tensorboard {
    include!(concat!(env!("OUT_DIR"), "/tensorboard.rs"));
}

// https://github.com/LaurentMazare/ocaml-tensorboard/blob/11022591e15327f31595443d18e1f3e38cc0a433/src/tensorboard/tf_record_writer.ml#L19
pub(crate) fn masked_crc(buf: &[u8]) -> u32 {
    let crc32c = crc::Crc::<u32>::new(&crc::CRC_32_ISCSI);
    let csum = crc32c.checksum(buf);
    (csum.wrapping_shr(15) | csum.wrapping_shl(17)).wrapping_add(0xa282ead8)
}
