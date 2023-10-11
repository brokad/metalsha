pub mod sha1;

const SHA1_METAL_MODULE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/metalsha.metallib"));

