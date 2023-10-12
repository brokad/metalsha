use std::mem;

use metal::*;
use crate::{BatchBuilder, Error, Hasher, ResultExt};

pub struct Sha1<'r> {
    hasher: &'r Hasher,
    command_buffer: &'r CommandBufferRef,
    encoder: &'r ComputeCommandEncoderRef
}

impl<'r> Sha1<'r> {
    fn from_hasher(hasher: &'r Hasher) -> Self {
        let compute_pass_descriptor = ComputePassDescriptor::new();

        let command_buffer = hasher.new_command_buffer();

        let encoder = command_buffer
            .compute_command_encoder_with_descriptor(compute_pass_descriptor);

        Self {
            hasher,
            command_buffer,
            encoder,
        }
    }

    fn set_pipeline_state(&self) -> Result<(), Error> {
        let kernel = self
            .hasher
            .library()
            .get_function("kernel_sha1_hash", None)
            .into_module_result()?;

        let pipeline_state_descriptor = ComputePipelineDescriptor::new();
        pipeline_state_descriptor.set_compute_function(Some(&kernel));
        let function = pipeline_state_descriptor.compute_function().unwrap();

        let pipeline_state = self
            .hasher
            .device()
            .new_compute_pipeline_state_with_function(function)
            .into_module_result()?;

        self.encoder.set_compute_pipeline_state(&pipeline_state);

        Ok(())
    }

    fn set_buffers(
        &mut self,
        BatchBuilder {
            buffer,
            framelen,
            asize,
            ..
        }: BatchBuilder
    ) -> Result<(), Error> {
        self.encoder.set_buffer(0, Some(buffer.as_ref()), 0);

        let framelen_buffer = self.hasher.device().new_buffer_with_data(
            unsafe { mem::transmute(&framelen) },
            mem::size_of_val(&framelen) as NSUInteger,
            MTLResourceOptions::StorageModeShared
        );

        self.encoder.set_buffer(1, Some(framelen_buffer.as_ref()), 0);

        let output_buffer = self
            .hasher
            .device()
            .new_buffer(asize as u64, MTLResourceOptions::StorageModeShared);

        self.encoder.set_buffer(2, Some(output_buffer.as_ref()), 0);

        Ok(())
    }
}