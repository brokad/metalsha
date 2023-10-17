use metalsha::*;

const TEST_INPUT: &'static str = "All work and no play makes Jack a dull boy";
const TEST_OUTPUT: [u8; 20] = [
    166, 44, 167, 32, 251,
    171, 131, 12, 136, 144,
    4, 78, 172, 190, 172,
    33, 111, 28, 162, 228
];

#[test]
fn sha1() {
    let hasher = Hasher::new().unwrap();
    let mut digest = hasher.digest(SHA1, TEST_INPUT.len(), 16);
    let mut input_buffer = digest.input_buffer();

    while let Some(frame) = input_buffer.next_frame() {
        frame.copy_from_slice(TEST_INPUT.as_bytes());
    }

    digest.run();

    let mut output_buffer = digest.output_buffer();

    assert_eq!(output_buffer.num_frames(), 16);

    while let Some(output) = output_buffer.next_frame() {
        assert_eq!(output, TEST_OUTPUT);
    }
}

