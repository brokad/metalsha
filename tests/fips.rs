use metalsha::*;

const TEST_INPUT_SHORT: &'static str = "All work and no play makes Jack a dull boy";
const TEST_OUTPUT_SHORT: &'static [u8; 20] = &[
    166, 44, 167, 32, 251, 171, 131, 12, 136, 144, 4, 78, 172, 190, 172, 33, 111, 28, 162, 228
];

const TEST_INPUT_LONG: &'static str = "\
There are many pleasant fictions of the law in constant operation, but there is not one so pleasant \
or practically humorous as that which supposes every man to be of equal value in its impartial eye, \
and the benefits of all laws to be equally attainable by all men, without the smallest reference to \
the furniture of their pockets.";
const TEST_OUTPUT_LONG: &'static [u8; 20] = &[
    44, 16, 5, 192, 116, 84, 14, 235, 38, 98, 216, 67, 1, 30, 46, 236, 50, 51, 133, 71
];

fn sha1(test_input: &str, test_output: &[u8; 20]) {
    let hasher = Hasher::new().unwrap();

    let mut digest = hasher.digest(
        SHA1,
        test_input.len(),
        16
    );
    let mut input_buffer = digest.input_buffer();

    while let Some(frame) = input_buffer.next_frame() {
        frame.copy_from_slice(test_input.as_bytes());
    }

    digest.run();

    let mut output_buffer = digest.output_buffer();

    assert_eq!(output_buffer.num_frames(), 16);

    while let Some(output) = output_buffer.next_frame() {
        assert_eq!(output, test_output);
    }
}

#[test]
fn sha1_short() {
    sha1(TEST_INPUT_SHORT, TEST_OUTPUT_SHORT);
}

#[test]
fn sha1_multi_block() {
    sha1(TEST_INPUT_LONG, TEST_OUTPUT_LONG);
}

