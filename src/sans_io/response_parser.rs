use std::mem::replace;
use std::str::{from_utf8, Utf8Error};

use crate::{RedisError, RedisValue};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ParseError {
    CannotParseInteger(std::num::ParseIntError),
    CannotConvertToUtf8(Utf8Error),
    InvalidBulkStringLength(i64),
    InvalidArrayLength(i64),
    InvalidResponseTypePrefix(char),
    ParserIsInAnErrorState,
}

#[derive(Debug)]
enum ResponseParserState {
    Waiting,
    Errored,
    ParsingInteger {
        start: usize,
    },
    ParsingSimpleString {
        start: usize,
    },
    ParsingError {
        start: usize,
    },
    ParsingBulkStringSize {
        start: usize,
    },
    ParsingBulkString {
        start: usize,
        size: usize,
    },
    ParsingArraySize {
        start: usize,
    },
    ParsingArray {
        elements: Vec<RedisValue>,
        cur_state: Box<ResponseParserState>,
    },
}

fn max_needed_buffer(state: &ResponseParserState, current: usize) -> usize {
    match state {
        ResponseParserState::Waiting => 0,
        ResponseParserState::Errored => 0,
        ResponseParserState::ParsingInteger { start } => current - start,
        ResponseParserState::ParsingSimpleString { start } => current - start,
        ResponseParserState::ParsingError { start } => current - start,
        ResponseParserState::ParsingBulkStringSize { start } => current - start,
        ResponseParserState::ParsingBulkString { start, .. } => current - start,
        ResponseParserState::ParsingArraySize { start } => current - start,
        ResponseParserState::ParsingArray { cur_state, .. } => {
            max_needed_buffer(cur_state, current)
        }
    }
}

fn parse_integer(data: &[u8], start: usize, ptr: &mut usize) -> Option<Result<i64, ParseError>> {
    match data[*ptr] as char {
        '\r' => Some(
            from_utf8(&data[start..*ptr])
                .map_err(ParseError::CannotConvertToUtf8)
                .and_then(|str| str.parse().map_err(ParseError::CannotParseInteger)),
        ),
        _ => None,
    }
}

fn parse_simple_string(
    data: &[u8],
    start: usize,
    ptr: &mut usize,
) -> Option<Result<String, ParseError>> {
    match data[*ptr] as char {
        '\r' => Some(
            from_utf8(&data[start..*ptr])
                .map_err(ParseError::CannotConvertToUtf8)
                .map(ToString::to_string),
        ),
        _ => None,
    }
}

fn parse_response(
    data: &[u8],
    ptr: &mut usize,
    state: &mut ResponseParserState,
) -> Result<Option<RedisValue>, ParseError> {
    while *ptr < data.len() {
        match state {
            ResponseParserState::Waiting => {
                *state = match data[*ptr] as char {
                    ':' => ResponseParserState::ParsingInteger { start: *ptr + 1 },
                    '+' => ResponseParserState::ParsingSimpleString { start: *ptr + 1 },
                    '-' => ResponseParserState::ParsingError { start: *ptr + 1 },
                    '$' => ResponseParserState::ParsingBulkStringSize { start: *ptr + 1 },
                    '*' => ResponseParserState::ParsingArraySize { start: *ptr + 1 },
                    any => {
                        *state = ResponseParserState::Errored;
                        return Err(ParseError::InvalidResponseTypePrefix(any));
                    }
                };
            }
            ResponseParserState::ParsingInteger { start } => {
                match parse_integer(data, *start, ptr) {
                    Some(Ok(int)) => {
                        *state = ResponseParserState::Waiting;
                        *ptr += 2;
                        return Ok(Some(RedisValue::Integer(int)));
                    }
                    Some(Err(err)) => {
                        *state = ResponseParserState::Errored;
                        return Err(err);
                    }
                    None => {}
                }
            }
            ResponseParserState::ParsingSimpleString { start } => {
                match parse_simple_string(data, *start, ptr) {
                    Some(Ok(string)) => {
                        *state = ResponseParserState::Waiting;
                        *ptr += 2;
                        return Ok(Some(RedisValue::String(string)));
                    }
                    Some(Err(err)) => {
                        *state = ResponseParserState::Errored;
                        return Err(err);
                    }
                    None => {}
                }
            }
            ResponseParserState::ParsingError { start } => {
                match parse_simple_string(data, *start, ptr) {
                    Some(Ok(string)) => {
                        *state = ResponseParserState::Waiting;
                        *ptr += 2;
                        return Ok(Some(RedisValue::Error(RedisError::new(string))));
                    }
                    Some(Err(err)) => {
                        *state = ResponseParserState::Errored;
                        return Err(err);
                    }
                    None => {}
                }
            }
            ResponseParserState::ParsingBulkStringSize { start } => {
                match parse_integer(data, *start, ptr) {
                    Some(Ok(int @ 1...std::i64::MAX)) => {
                        *ptr += 2;
                        *state = ResponseParserState::ParsingBulkString {
                            start: *ptr,
                            size: int as usize,
                        };
                    }
                    Some(Ok(0)) => {
                        *ptr += 4;
                        *state = ResponseParserState::Waiting;
                        return Ok(Some(RedisValue::String(String::new())));
                    }
                    Some(Ok(-1)) => {
                        *ptr += 2;
                        *state = ResponseParserState::Waiting;
                        return Ok(Some(RedisValue::Null));
                    }
                    Some(Ok(int)) => {
                        *state = ResponseParserState::Errored;
                        return Err(ParseError::InvalidBulkStringLength(int));
                    }
                    Some(Err(err)) => {
                        *state = ResponseParserState::Errored;
                        return Err(err);
                    }
                    None => {}
                }
            }
            ResponseParserState::ParsingBulkString { start, size } => {
                if *start + *size == *ptr {
                    let data = from_utf8(&data[*start..*ptr])
                        .map_err(ParseError::CannotConvertToUtf8)
                        .map(|string| Some(RedisValue::String(string.to_string())));
                    *ptr += 1;
                    return data;
                }
            }
            ResponseParserState::ParsingArraySize { start } => {
                match parse_integer(data, *start, ptr) {
                    Some(Ok(int @ 1...std::i64::MAX)) => {
                        *ptr += 2;
                        *state = ResponseParserState::ParsingArray {
                            elements: Vec::with_capacity(int as usize),
                            cur_state: Box::new(ResponseParserState::Waiting),
                        };
                        // skip the ptr increment at the end of each loop iteration,
                        // jump straight to parsing the first element of this array
                        continue;
                    }
                    Some(Ok(0)) => {
                        *ptr += 2;
                        *state = ResponseParserState::Waiting;
                        return Ok(Some(RedisValue::Array(Vec::new())));
                    }
                    Some(Ok(-1)) => {
                        *ptr += 2;
                        *state = ResponseParserState::Waiting;
                        return Ok(Some(RedisValue::Null));
                    }
                    Some(Ok(int)) => {
                        *state = ResponseParserState::Errored;
                        return Err(ParseError::InvalidArrayLength(int));
                    }
                    Some(Err(err)) => {
                        *state = ResponseParserState::Errored;
                        return Err(err);
                    }
                    None => {}
                }
            }
            ResponseParserState::ParsingArray {
                elements,
                cur_state,
            } => match parse_response(data, ptr, cur_state) {
                Ok(Some(element)) => {
                    elements.push(element);
                    if elements.len() == elements.capacity() {
                        if let ResponseParserState::ParsingArray { elements, .. } =
                            replace(state, ResponseParserState::Waiting)
                        {
                            return Ok(Some(RedisValue::Array(elements)));
                        } else {
                            panic!("This point should be unreachable");
                        }
                    } else {
                        // skip the ptr increment here as well, for some reason it just
                        // doesn't seem to like arrays...  :/
                        continue;
                    }
                }
                Ok(None) => {
                    {};
                }
                Err(error) => {
                    *state = ResponseParserState::Errored;
                    return Err(error);
                }
            },
            ResponseParserState::Errored => return Err(ParseError::ParserIsInAnErrorState),
        }

        *ptr += 1;
    }

    Ok(None)
}

#[derive(Debug)]
pub struct ResponseParser {
    buffer: String,
    ptr: usize,
    state: ResponseParserState,
}

impl ResponseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            ptr: 0,
            state: ResponseParserState::Waiting,
        }
    }

    pub fn feed(&mut self, response: &str) {
        self.buffer.push_str(response)
    }

    pub fn get_response(&mut self) -> Result<Option<RedisValue>, ParseError> {
        let Self { buffer, ptr, state } = self;
        let response = parse_response(buffer.as_bytes(), ptr, state);
        if let Ok(Some(response)) = response {
            let mut needed_buffer_start = *ptr - max_needed_buffer(state, *ptr);
            while !buffer.is_char_boundary(needed_buffer_start) {
                if needed_buffer_start <= 1 {
                    return Ok(Some(response));
                }
                needed_buffer_start -= 1;
            }
            buffer.replace_range(0..needed_buffer_start, "");
            *ptr -= needed_buffer_start;
            Ok(Some(response))
        } else {
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RedisError;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    #[test]
    fn can_parse_numbers_from_redis_response() {
        let mut parser = ResponseParser::new();
        parser.feed(":42\r\n");
        assert_eq!(Ok(Some(RedisValue::Integer(42))), parser.get_response());
    }

    #[quickcheck]
    fn qc_can_parse_any_number_from_redis_response(num: i64) {
        let mut parser = ResponseParser::new();
        parser.feed(&format!(":{}\r\n", num));
        assert_eq!(Ok(Some(RedisValue::Integer(num))), parser.get_response());
    }

    #[test]
    fn can_parse_multiple_numbers_in_a_row() {
        let mut parser = ResponseParser::new();
        parser.feed(":42\r\n:123\r\n");
        assert_eq!(Ok(Some(RedisValue::Integer(42))), parser.get_response());
        assert_eq!(Ok(Some(RedisValue::Integer(123))), parser.get_response());
    }

    #[test]
    fn parsing_can_be_resumed_at_any_time() {
        let mut parser = ResponseParser::new();
        parser.feed(":4");
        assert_eq!(Ok(None), parser.get_response());

        parser.feed("12\r");
        assert_eq!(Ok(Some(RedisValue::Integer(412))), parser.get_response());

        parser.feed("\n:1\r\n");
        assert_eq!(Ok(Some(RedisValue::Integer(1))), parser.get_response());

        parser.feed(":");
        assert_eq!(Ok(None), parser.get_response());

        parser.feed("412");
        assert_eq!(Ok(None), parser.get_response());

        parser.feed("\r\n");
        assert_eq!(Ok(Some(RedisValue::Integer(412))), parser.get_response());
    }

    #[test]
    fn doesnt_fail_on_an_empty_string() {
        let mut parser = ResponseParser::new();
        parser.feed("");
        assert_eq!(Ok(None), parser.get_response())
    }

    #[test]
    fn can_parse_a_simple_string() {
        let mut parser = ResponseParser::new();
        parser.feed("+OK\r\n");
        assert_eq!(
            Ok(Some(RedisValue::String("OK".to_string()))),
            parser.get_response()
        );
    }

    #[quickcheck]
    fn qc_can_parse_any_simple_string(text: String) -> TestResult {
        if text.contains('\r') || text.contains('\n') {
            // these are not valid simple strings
            return TestResult::discard();
        }

        let mut parser = ResponseParser::new();
        parser.feed(&format!("+{}\r\n", text));
        assert_eq!(Ok(Some(RedisValue::String(text))), parser.get_response());
        TestResult::passed()
    }

    #[test]
    fn can_parse_an_error() {
        let mut parser = ResponseParser::new();
        parser.feed("-OK\r\n");
        assert_eq!(
            Ok(Some(RedisValue::Error(RedisError::new("OK")))),
            parser.get_response()
        );
    }

    #[quickcheck]
    fn qc_can_parse_any_error(text: String) -> TestResult {
        if text.contains('\r') || text.contains('\n') {
            // these are not valid errors
            return TestResult::discard();
        }

        let mut parser = ResponseParser::new();
        parser.feed(&format!("-{}\r\n", text));
        assert_eq!(
            Ok(Some(RedisValue::Error(RedisError::new(text)))),
            parser.get_response()
        );
        TestResult::passed()
    }

    #[test]
    fn can_parse_a_bulk_string() {
        let mut parser = ResponseParser::new();
        parser.feed("$2\r\nOK\r\n");
        assert_eq!(
            Ok(Some(RedisValue::String("OK".to_string()))),
            parser.get_response()
        );
    }

    #[test]
    fn can_parse_an_empty_bulk_string() {
        // probably caught already in the quickcheck test, but we might as well
        // double check here that it's recognised.  This is currently special cased
        // in the parser.
        let mut parser = ResponseParser::new();
        parser.feed("$0\r\n\r\n");
        assert_eq!(
            Ok(Some(RedisValue::String("".to_string()))),
            parser.get_response()
        );
    }

    #[test]
    fn doesnt_fail_on_a_negative_bulk_string_size() {
        let mut parser = ResponseParser::new();
        parser.feed("$-100\r\n");
        assert_eq!(
            Err(ParseError::InvalidBulkStringLength(-100)),
            parser.get_response()
        )
    }

    #[quickcheck]
    fn qc_can_parse_any_bulk_string(text: String) {
        let mut parser = ResponseParser::new();
        parser.feed(&format!("${}\r\n{}\r\n", text.len(), text));
        assert_eq!(Ok(Some(RedisValue::String(text))), parser.get_response());
    }

    #[test]
    fn can_parse_the_null_bulk_string() {
        let mut parser = ResponseParser::new();
        parser.feed("$-1\r\n");
        assert_eq!(Ok(Some(RedisValue::Null)), parser.get_response());
    }

    #[test]
    fn can_parse_an_array() {
        let mut parser = ResponseParser::new();
        parser.feed(
            "*2\r\n\
             *3\r\n\
             :1\r\n\
             :2\r\n\
             :3\r\n\
             *2\r\n\
             +Foo\r\n\
             -Bar\r\n",
        );
        assert_eq!(
            Ok(Some(RedisValue::Array(vec![
                RedisValue::Array(vec![
                    RedisValue::Integer(1),
                    RedisValue::Integer(2),
                    RedisValue::Integer(3),
                ]),
                RedisValue::Array(vec![
                    RedisValue::String("Foo".to_string()),
                    RedisValue::Error(RedisError::new("Bar")),
                ])
            ]))),
            parser.get_response()
        );
    }

    #[test]
    fn can_parse_an_empty_array() {
        let mut parser = ResponseParser::new();
        parser.feed("*0\r\n");
        assert_eq!(
            Ok(Some(RedisValue::Array(Vec::new()))),
            parser.get_response()
        );
    }

    #[test]
    fn can_parse_a_null_array() {
        let mut parser = ResponseParser::new();
        parser.feed("*-1\r\n");
        assert_eq!(Ok(Some(RedisValue::Null)), parser.get_response());
    }

    #[test]
    fn doesnt_fail_on_a_negative_array_size() {
        let mut parser = ResponseParser::new();
        parser.feed("*-100\r\n");
        assert_eq!(
            Err(ParseError::InvalidArrayLength(-100)),
            parser.get_response()
        )
    }

    #[test]
    fn doesnt_fail_with_array_of_one_zero_int() {
        let mut parser = ResponseParser::new();
        parser.feed("*1\r\n:0\r\n");
        assert_eq!(
            Ok(Some(RedisValue::Array(vec![RedisValue::Integer(0),]))),
            parser.get_response()
        )
    }

    #[test]
    fn doesnt_fail_with_array_of_two_zero_ints() {
        let mut parser = ResponseParser::new();
        parser.feed("*2\r\n:0\r\n:0\r\n");
        assert_eq!(
            Ok(Some(RedisValue::Array(vec![
                RedisValue::Integer(0),
                RedisValue::Integer(0),
            ]))),
            parser.get_response()
        )
    }

    #[quickcheck]
    fn qc_can_parse_any_list_of_integers(ints: Vec<i64>) {
        let mut text = String::new();
        text.push_str(&format!("*{}\r\n", ints.len()));
        for int in &ints {
            text.push_str(&format!(":{}\r\n", int));
        }

        let redis_array = ints.iter().map(|int| RedisValue::Integer(*int)).collect();

        let mut parser = ResponseParser::new();
        parser.feed(&text);
        assert_eq!(
            Ok(Some(RedisValue::Array(redis_array))),
            parser.get_response()
        );
    }

    #[quickcheck]
    fn qc_can_attempt_to_parse_anything_without_panicking(input: String) {
        let mut parser = ResponseParser::new();
        parser.feed(&input);
        loop {
            match parser.get_response() {
                Ok(Some(_)) => continue,
                Ok(None) => break, // input is used up
                Err(_) => break,   // parsing encountered an error
            }
        }
    }
}
