#![no_main]

use json_event_parser::{
    JsonEvent, JsonSyntaxError, LowLevelJsonParser, LowLevelJsonParserResult, WriterJsonSerializer,
};
use libfuzzer_sys::fuzz_target;

fn parse_chunks(chunks: &[&[u8]]) -> (String, Option<JsonSyntaxError>) {
    let mut input_buffer = Vec::new();
    let mut input_cursor = 0;
    let mut output_buffer = Vec::new();
    let mut reader = LowLevelJsonParser::new();
    let mut writer = WriterJsonSerializer::new(&mut output_buffer);
    let mut error = None;
    for (i, chunk) in chunks.iter().enumerate() {
        input_buffer.extend_from_slice(chunk);
        loop {
            let LowLevelJsonParserResult {
                event,
                consumed_bytes,
            } = reader.parse_next(&input_buffer[input_cursor..], i == chunks.len() - 1);
            input_cursor += consumed_bytes;
            match event {
                Some(Ok(JsonEvent::Eof)) => {
                    if error.is_none() {
                        writer.finish().unwrap();
                    }
                    return (String::from_utf8(output_buffer).unwrap(), error);
                }
                Some(Ok(event)) => {
                    if error.is_none() {
                        writer.serialize_event(event).unwrap();
                    } else {
                        let _ = writer.serialize_event(event); // We don't know if we write ok structure
                    }
                }
                Some(Err(e)) => {
                    if error.is_none() {
                        error = Some(e)
                    }
                }
                None => break,
            }
        }
    }
    panic!("Should not be reached")
}

fn merge<'a>(slices: impl IntoIterator<Item = &'a [u8]>) -> Vec<u8> {
    let mut buf = Vec::new();
    for slice in slices {
        buf.extend_from_slice(slice);
    }
    buf
}

fuzz_target!(|data: &[u8]| {
    // We parse with separators
    let (with_separators, with_separators_error) =
        parse_chunks(&data.split(|c| *c == 0xFF).collect::<Vec<_>>());
    let (without_separators, without_separators_error) =
        parse_chunks(&[&merge(data.split(|c| *c == 0xFF))]);
    assert_eq!(
        with_separators_error
            .as_ref()
            .map_or_else(String::new, |e| e.to_string()),
        without_separators_error
            .as_ref()
            .map_or_else(String::new, |e| e.to_string()),
        "{with_separators_error:?} vs {without_separators_error:?}"
    );
    assert_eq!(with_separators, without_separators);

    if with_separators_error.is_none() {
        let (again, again_error) = parse_chunks(&[with_separators.as_bytes()]);
        assert!(
            again_error.is_none(),
            "Failed to parse '{with_separators}' with error {}",
            again_error.unwrap()
        );
        assert_eq!(with_separators, again);
    }
});
