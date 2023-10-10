use std::path::Path;
use std::slice;
use metal::*;
use objc::rc::autoreleasepool;

fn create_pipeline_state<P: AsRef<Path>>(path: P, device: &Device) -> ComputePipelineState {
    let library = device.new_library_with_file(path).unwrap();
    let kernel = library.get_function("add_arrays", None).unwrap();

    let pipeline_state_descriptor = ComputePipelineDescriptor::new();
    pipeline_state_descriptor.set_compute_function(Some(&kernel));

    device
        .new_compute_pipeline_state_with_function(
            pipeline_state_descriptor.compute_function().unwrap()
        )
        .unwrap()
}

fn create_input_and_output_buffers(device: &Device, num_elements: usize) -> ((Buffer, Buffer), Buffer) {
    let data = vec![1u32; num_elements];

    let make_buffer = || {
        device.new_buffer_with_data(
            unsafe { std::mem::transmute(data.as_ptr()) },
            (data.len() * std::mem::size_of::<u32>()) as u64,
            MTLResourceOptions::CPUCacheModeDefaultCache
        )
    };

    (
        (
            make_buffer(),
            make_buffer()
        ),
        make_buffer()
    )
}

fn main() {
    autoreleasepool(|| {
        let num_elements = 1024 * 64 * 64;

        let device = Device::system_default().unwrap();

        let command_queue = device.new_command_queue();
        let command_buffer = command_queue.new_command_buffer();

        let compute_pass_descriptor = ComputePassDescriptor::new();

        let encoder = command_buffer.compute_command_encoder_with_descriptor(compute_pass_descriptor);

        let pipeline_state = create_pipeline_state(
            "./metal/sh.damien.metalsha.metallib",
            &device
        );
        encoder.set_compute_pipeline_state(&pipeline_state);

        let ((buffer_a, buffer_b), buffer_o) = create_input_and_output_buffers(&device, num_elements);

        encoder.set_buffer(0, Some(&buffer_a), 0);
        encoder.set_buffer(1, Some(&buffer_b), 0);
        encoder.set_buffer(2, Some(&buffer_o), 0);

        let num_threads = pipeline_state.thread_execution_width();

        let thread_group_count = MTLSize {
            width: (num_elements as NSUInteger + num_threads) / num_threads,
            height: 1,
            depth: 1
        };

        let thread_group_size = MTLSize {
            width: num_threads,
            height: 1,
            depth: 1
        };

        encoder.dispatch_thread_groups(thread_group_count, thread_group_size);
        encoder.end_encoding();

        command_buffer.commit();
        command_buffer.wait_until_completed();

        let output: &[u32] = unsafe {
            slice::from_raw_parts(std::mem::transmute(buffer_o.contents()), num_elements)
        };

        for o in output {
            assert_eq!(*o, 2);
        }
    })
}
