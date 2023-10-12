use std::fmt;
use std::slice;
use std::mem;
use metal::*;

pub mod sha1;

const METAL_MODULE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/metalsha.metallib"));

pub enum Error {
    Metal(String)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metal(s) => write!(f, "metal: {s}")
        }
    }
}

mod sealed {
    pub trait Sealed {}

    impl<O> Sealed for Result<O, String> {}
}

pub trait ResultExt<O>: sealed::Sealed {
    fn into_module_result(self) -> Result<O, Error>;
}

impl<O> ResultExt<O> for Result<O, String> {
    fn into_module_result(self) -> Result<O, Error> {
        self.map_err(|s| Error::Metal(s))
    }
}

pub trait Digest<'r> {
    fn from_hasher(hasher: &'r Hasher) -> Self;
}

pub struct BatchBuilder {
    pub(crate) framelen: usize,
    pub(crate) bsize: usize,
    pub(crate) asize: usize,
    pub(crate) buffer: Buffer,
}

impl BatchBuilder {
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                mem::transmute(self.buffer.contents()),
                self.bsize
            )
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                mem::transmute(self.buffer.contents()),
                self.bsize
            )
        }
    }

    pub fn next_frame(&mut self) -> Option<&mut [u8]> {
        let framestart = self.asize;
        let frameend = self.asize + self.framelen;
        if frameend <= self.bsize {
            self.asize += self.framelen;
            Some(&mut self.as_slice_mut()[framestart..frameend])
        } else {
            None
        }
    }
}

pub struct Hasher {
    device: Device,
    library: Library,
    command_queue: CommandQueue,
}

impl Hasher {
    pub fn from_device(device: Device) -> Result<Self, Error> {
        let library = device
            .new_library_with_data(METAL_MODULE)
            .into_module_result()?;

        let command_queue = device.new_command_queue();

        Ok(Self {
            device,
            library,
            command_queue,
        })
    }

    pub fn new() -> Result<Self, Error> {
        Self::from_device(Device::system_default().unwrap())
    }

    pub fn new_command_buffer(&self) -> &CommandBufferRef {
        self.command_queue.new_command_buffer()
    }

    pub fn digest<'s, D: Digest<'s>>(&'s self) -> D {
        D::from_hasher(self)
    }

    pub fn new_batch(&self, framelen: usize, count: usize) -> BatchBuilder {
        let bsize = framelen * count;

        let buffer = self
            .device
            .new_buffer(bsize as u64, MTLResourceOptions::StorageModeShared);

        BatchBuilder {
            framelen,
            bsize,
            asize: 0,
            buffer,
        }
    }

    pub fn new_batch_with_data(&self, _data: &[u8]) -> BatchBuilder {
        todo!()
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn device(&self) -> &Device {
        &self.device
    }
}