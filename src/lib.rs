use std::cmp::max;
use std::{error, fmt};
use std::slice;
use std::mem;
use std::ops::Range;
use metal::*;

const METAL_MODULE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/metalsha.metallib"));

#[derive(Debug)]
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

impl error::Error for Error {}

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

pub trait Digest {
    const DIGEST_SIZE: usize;
    const KERNEL_FN: &'static str;
}

pub struct SHA1;

impl Digest for SHA1 {
    const DIGEST_SIZE: usize = 20;
    const KERNEL_FN: &'static str = "kernel_sha1_hash";
}

#[repr(C)]
pub struct DigestRunArgs {
    inlen: u64
}

pub struct DigestRunArgsBuffer {
    buffer: Buffer
}

impl DigestRunArgsBuffer {
    fn new_from_buffer(buffer: Buffer) -> Self {
        assert_eq!(buffer.length() as usize, mem::size_of::<DigestRunArgs>());
        Self { buffer }
    }

    pub fn as_ref_mut(&mut self) -> &mut DigestRunArgs {
        unsafe { mem::transmute(self.buffer.contents()) }
    }

    pub fn as_ref(& self) -> &DigestRunArgs {
        unsafe { mem::transmute(self.buffer.contents()) }
    }

    pub fn as_ref_inner(&self) -> &BufferRef {
        self.buffer.as_ref()
    }
}

pub struct BatchBuffer {
    frame_length: usize,
    actual_size: usize,
    buffer: Buffer,
}

impl BatchBuffer {
    pub fn new(buffer: Buffer, frame_length: usize) -> Self {
        Self {
            frame_length,
            actual_size: 0,
            buffer,
        }
    }

    fn as_ref_inner(&self) -> &BufferRef {
        self.buffer.as_ref()
    }

    fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                mem::transmute(self.buffer.contents()),
                self.length(),
            )
        }
    }

    fn as_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                mem::transmute(self.buffer.contents()),
                self.length(),
            )
        }
    }

    pub fn length(&self) -> usize {
        self.buffer.length() as usize
    }

    pub fn num_frames(&self) -> usize {
        (self.actual_size + self.frame_length - 1) / self.frame_length
    }

    pub fn reset(&mut self) -> &mut Self {
        self.actual_size = 0;
        self
    }

    fn raw_frame_bounds(&self, at: usize) -> Option<Range<usize>> {
        let frame_start = at - at % self.frame_length;
        let frame_end = frame_start + self.frame_length;

        if frame_end <= self.length() {
            Some(frame_start..frame_end)
        } else {
            None
        }
    }

    fn frame_mut(&mut self, at: usize) -> Option<&mut [u8]> {
        if let Some(frame) = self.raw_frame_bounds(at) {
            self.actual_size = max(frame.end, self.actual_size);
            Some(&mut self.as_slice_mut()[frame])
        } else {
            None
        }
    }

    fn frame(&self, at: usize) -> Option<&[u8]> {
        if let Some(frame) = self.raw_frame_bounds(at) {
            if frame.end <= self.actual_size {
                return Some(&self.as_slice()[frame]);
            }
        }
        None
    }
}

pub struct BatchBufferSetter<'r> {
    at: usize,
    inner: &'r mut BatchBuffer
}

impl<'r> BatchBufferSetter<'r> {
    fn new(batch_buffer: &'r mut BatchBuffer) -> Self {
        batch_buffer.reset();
        Self {
            at: 0,
            inner: batch_buffer
        }
    }

    pub fn next_frame(&mut self) -> Option<&mut [u8]> {
        let frame = self.inner.frame_mut(self.at);

        if let Some(frame) = frame.as_ref() {
            self.at += frame.len();
        }

        frame
    }
}

pub struct BatchBufferReader<'r> {
    at: usize,
    inner: &'r BatchBuffer
}

impl<'r> BatchBufferReader<'r> {
    fn new(batch_buffer: &'r BatchBuffer) -> Self {
        Self {
            at: 0,
            inner: batch_buffer
        }
    }

    pub fn next_frame(&mut self) -> Option<&[u8]> {
        let frame = self.inner.frame(self.at);

        if let Some(frame) = frame.as_ref() {
            self.at += frame.len();
        }

        frame
    }
}

pub struct DigestCommandRun<'r, D> {
    hasher: &'r Hasher,
    pipeline_state: ComputePipelineState,
    input_buffer: BatchBuffer,
    args_buffer: DigestRunArgsBuffer,
    output_buffer: BatchBuffer,
    _digest: D,
}

impl<'r, D: Digest> DigestCommandRun<'r, D> {
    fn new(hasher: &'r Hasher, digest: D, inlen: usize, count: usize) -> Self {
        let input_buffer = hasher.new_batch_buffer(inlen, count);
        let output_buffer = hasher.new_batch_buffer(D::DIGEST_SIZE, count);
        let args_buffer = hasher.new_args_buffer();

        let pipeline_state = Self::new_pipeline_state(hasher);
        Self {
            hasher,
            pipeline_state,
            input_buffer,
            args_buffer,
            output_buffer,
            _digest: digest,
        }
    }

    pub fn input_buffer(&mut self) -> BatchBufferSetter<'_> {
        self.input_buffer.reset();
        BatchBufferSetter::new(&mut self.input_buffer)
    }

    pub fn output_buffer(&self) -> BatchBufferReader<'_> {
        BatchBufferReader::new(&self.output_buffer)
    }

    fn new_pipeline_state(hasher: &Hasher) -> ComputePipelineState {
        let kernel = hasher
            .library()
            .get_function(D::KERNEL_FN, None)
            .unwrap();

        let pipeline_state_descriptor = ComputePipelineDescriptor::new();
        pipeline_state_descriptor.set_compute_function(Some(&kernel));
        let function = pipeline_state_descriptor.compute_function().unwrap();

        hasher
            .device()
            .new_compute_pipeline_state_with_function(function)
            .unwrap()
    }

    pub fn run(Self {
        hasher,
        pipeline_state,
        input_buffer,
        args_buffer,
        output_buffer,
        ..
    }: &mut Self) {
        let compute_pass_descriptor = ComputePassDescriptor::new();
        let command_buffer = hasher.new_command_buffer();
        let encoder = command_buffer
            .compute_command_encoder_with_descriptor(compute_pass_descriptor);
        encoder.set_compute_pipeline_state(&pipeline_state);

        args_buffer.as_ref_mut().inlen = input_buffer.frame_length as u64;
        let output_actual_size = input_buffer.num_frames() * D::DIGEST_SIZE;
        assert!(output_buffer.length() >= output_actual_size);
        output_buffer.actual_size = output_actual_size;

        encoder.set_buffer(0, Some(input_buffer.as_ref_inner()), 0);
        encoder.set_buffer(1, Some(output_buffer.as_ref_inner()), 0);
        encoder.set_buffer(2, Some(args_buffer.as_ref_inner()), 0);

        let num_threads = pipeline_state.thread_execution_width();

        let thread_group_count = MTLSize {
            width: ((input_buffer.num_frames() as NSUInteger + num_threads) / num_threads),
            height: 1,
            depth: 1,
        };

        let thread_group_size = MTLSize {
            width: num_threads,
            height: 1,
            depth: 1,
        };

        encoder.dispatch_thread_groups(thread_group_count, thread_group_size);
        encoder.end_encoding();
        command_buffer.commit();
        command_buffer.wait_until_completed();
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

    pub fn digest<D: Digest>(
        &self,
        digest: D,
        inlen: usize,
        count: usize
    ) -> DigestCommandRun<D> {
        DigestCommandRun::new(self, digest, inlen, count)
    }

    fn new_args_buffer(&self) -> DigestRunArgsBuffer {
        let buffer = self
            .device
            .new_buffer(
                mem::size_of::<DigestRunArgs>() as u64,
                MTLResourceOptions::StorageModeShared
            );

        DigestRunArgsBuffer::new_from_buffer(buffer)
    }

    fn new_batch_buffer(&self, inlen: usize, count: usize) -> BatchBuffer {
        let bsize = inlen * count;

        let buffer = self
            .device
            .new_buffer(bsize as u64, MTLResourceOptions::StorageModeShared);

        BatchBuffer::new(buffer, inlen)
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn device(&self) -> &Device {
        &self.device
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    const IN_LEN: usize = 1024;
    const IN_COUNT: usize = 255;

    #[test]
    fn batch_buffer_rw() {
        let hasher = Hasher::new().unwrap();

        let mut buffer = hasher.new_batch_buffer(IN_LEN, IN_COUNT);
        let mut buffer_setter = BatchBufferSetter::new(&mut buffer);
        let buffer_setter_mut = &mut buffer_setter;

        let mut nframe: u8 = 0;

        while let Some(frame) = buffer_setter_mut.next_frame() {
            assert_eq!(frame.len(), IN_LEN);
            frame.copy_from_slice(&[nframe; IN_LEN]);
            nframe += 1;
            if nframe >= 16 {
                break
            }
        }

        assert_eq!(buffer.actual_size, IN_LEN * 16);

        let mut buffer_reader = BatchBufferReader::new(&buffer);

        nframe = 0;

        while let Some(frame) = buffer_reader.next_frame() {
            assert_eq!(*frame, [nframe; IN_LEN]);
            nframe += 1;
        }

        assert_eq!(nframe, 16);
    }

    #[test]
    fn args_buffer_rw() {
        let hasher = Hasher::new().unwrap();
        let mut args_buffer = hasher.new_args_buffer();
        args_buffer.as_ref_mut().inlen = 64;
        assert_eq!(args_buffer.as_ref().inlen, 64);
    }
}
