use std::mem::replace;
use std::str::{from_utf8, Utf8Error};

use crate::types::RedisErrorValue;
use crate::types::RedisResult;
use std::cmp;
use std::iter::empty;

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
        elements: Vec<RedisResult>,
        cur_state: Box<ResponseParserState>,
    },
}

fn max_needed_buffer(state: &ResponseParserState, current: usize) -> usize {
    match state {
        ResponseParserState::Waiting => 0,
        ResponseParserState::Errored => 0,
        ResponseParserState::ParsingInteger { start } => current - *start,
        ResponseParserState::ParsingSimpleString { start } => current - *start,
        ResponseParserState::ParsingError { start } => current - *start,
        ResponseParserState::ParsingBulkStringSize { start } => current - *start,
        ResponseParserState::ParsingBulkString { start, .. } => current - *start,
        ResponseParserState::ParsingArraySize { start } => current - *start,
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

fn parse_simple_string(data: &[u8], start: usize, ptr: &mut usize) -> Option<Vec<u8>> {
    match data[*ptr] as char {
        '\r' => Some((&data[start..*ptr]).to_vec()),
        _ => None,
    }
}

fn parse_response(
    data: &[u8],
    ptr: &mut usize,
    state: &mut ResponseParserState,
) -> Result<Option<RedisResult>, ParseError> {
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
                *ptr += 1;
            }
            ResponseParserState::ParsingInteger { start } => {
                match parse_integer(data, *start, ptr) {
                    Some(Ok(int)) => {
                        *state = ResponseParserState::Waiting;
                        *ptr += 2;
                        return Ok(Some(RedisResult::Integer(int)));
                    }
                    Some(Err(err)) => {
                        *state = ResponseParserState::Errored;
                        return Err(err);
                    }
                    None => {
                        *ptr += 1;
                    }
                }
            }
            ResponseParserState::ParsingSimpleString { start } => {
                match parse_simple_string(data, *start, ptr) {
                    Some(vec) => {
                        *state = ResponseParserState::Waiting;
                        *ptr += 2;
                        return Ok(Some(RedisResult::String(vec)));
                    }
                    None => {
                        *ptr += 1;
                    }
                }
            }
            ResponseParserState::ParsingError { start } => {
                match parse_simple_string(data, *start, ptr) {
                    Some(bytes) => {
                        *state = ResponseParserState::Waiting;
                        *ptr += 2;
                        let error_string =
                            from_utf8(&bytes).map_err(|err| ParseError::CannotConvertToUtf8(err));
                        return match error_string {
                            Ok(s) => Ok(Some(RedisResult::Error(RedisErrorValue::new(s)))),
                            Err(error) => {
                                *state = ResponseParserState::Errored;
                                Err(error)
                            }
                        };
                    }
                    None => {
                        *ptr += 1;
                    }
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
                        return Ok(Some(RedisResult::String(Vec::new())));
                    }
                    Some(Ok(-1)) => {
                        *ptr += 2;
                        *state = ResponseParserState::Waiting;
                        return Ok(Some(RedisResult::Null));
                    }
                    Some(Ok(int)) => {
                        *state = ResponseParserState::Errored;
                        return Err(ParseError::InvalidBulkStringLength(int));
                    }
                    Some(Err(err)) => {
                        *state = ResponseParserState::Errored;
                        return Err(err);
                    }
                    None => {
                        *ptr += 1;
                    }
                }
            }
            ResponseParserState::ParsingBulkString { start, size } => {
                if *start + *size == *ptr {
                    *ptr += 2;

                    if let ResponseParserState::ParsingBulkString { start, .. } =
                        replace(state, ResponseParserState::Waiting)
                    {
                        return Ok(Some(RedisResult::String(data[start..(*ptr - 2)].to_vec())));
                    } else {
                        panic!("This point should be unreachable");
                    }
                } else if *start + *size < *ptr {
                    unreachable!("The current position pointer is beyond the end of the string we are trying to parser.  This is Bad News.")
                }
                *ptr += 1;
            }
            ResponseParserState::ParsingArraySize { start } => {
                match parse_integer(data, *start, ptr) {
                    Some(Ok(int @ 1...std::i64::MAX)) => {
                        *ptr += 2;
                        *state = ResponseParserState::ParsingArray {
                            elements: Vec::with_capacity(int as usize),
                            cur_state: Box::new(ResponseParserState::Waiting),
                        };
                    }
                    Some(Ok(0)) => {
                        *ptr += 2;
                        *state = ResponseParserState::Waiting;
                        return Ok(Some(RedisResult::Array(Vec::new())));
                    }
                    Some(Ok(-1)) => {
                        *ptr += 2;
                        *state = ResponseParserState::Waiting;
                        return Ok(Some(RedisResult::Null));
                    }
                    Some(Ok(int)) => {
                        *state = ResponseParserState::Errored;
                        return Err(ParseError::InvalidArrayLength(int));
                    }
                    Some(Err(err)) => {
                        *state = ResponseParserState::Errored;
                        return Err(err);
                    }
                    None => {
                        *ptr += 1;
                    }
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
                            return Ok(Some(RedisResult::Array(elements)));
                        } else {
                            panic!("This point should be unreachable");
                        }
                    }
                }
                Ok(None) => {
                    // Let the recursed-into parser handle incrementing the current pointer
                }
                Err(error) => {
                    *state = ResponseParserState::Errored;
                    return Err(error);
                }
            },
            ResponseParserState::Errored => return Err(ParseError::ParserIsInAnErrorState),
        }
    }

    Ok(None)
}

#[derive(Debug)]
pub(in crate::sans_io) struct ResponseParser {
    buffer: Vec<u8>,
    ptr: usize,
    state: ResponseParserState,
}

impl ResponseParser {
    pub(in crate::sans_io) fn new() -> Self {
        Self {
            buffer: Vec::new(),
            ptr: 0,
            state: ResponseParserState::Waiting,
        }
    }

    pub(in crate::sans_io) fn feed(&mut self, response: &[u8]) {
        self.buffer.extend_from_slice(response)
    }

    pub(in crate::sans_io) fn get_response(&mut self) -> Result<Option<RedisResult>, ParseError> {
        let Self { buffer, ptr, state } = self;
        let response = parse_response(&buffer, ptr, state);
        if let Ok(Some(response)) = response {
            let needed_buffer_start = *ptr - max_needed_buffer(state, *ptr);
            // occasionally the ptr ends up further beyond the buffer size - that's okay,
            // we'll just delete what is available, and worry about the rest later
            let needed_buffer_start = cmp::min(needed_buffer_start, buffer.len() - 1);
            buffer.splice(0..needed_buffer_start, empty());
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
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    #[test]
    fn can_parse_numbers_from_redis_response() {
        let mut parser = ResponseParser::new();
        parser.feed(":42\r\n".as_bytes());
        assert_eq!(Ok(Some(RedisResult::Integer(42))), parser.get_response());
    }

    #[quickcheck]
    fn qc_can_parse_any_number_from_redis_response(num: i64) {
        let mut parser = ResponseParser::new();
        parser.feed(format!(":{}\r\n", num).as_bytes());
        assert_eq!(Ok(Some(RedisResult::Integer(num))), parser.get_response());
    }

    #[test]
    fn can_parse_multiple_numbers_in_a_row() {
        let mut parser = ResponseParser::new();
        parser.feed(":42\r\n:123\r\n".as_bytes());
        assert_eq!(Ok(Some(RedisResult::Integer(42))), parser.get_response());
        assert_eq!(Ok(Some(RedisResult::Integer(123))), parser.get_response());
    }

    #[test]
    fn parsing_can_be_resumed_at_any_time() {
        let mut parser = ResponseParser::new();
        parser.feed(":4".as_bytes());
        assert_eq!(Ok(None), parser.get_response());

        parser.feed("12\r".as_bytes());
        assert_eq!(Ok(Some(RedisResult::Integer(412))), parser.get_response());

        parser.feed("\n:1\r\n".as_bytes());
        assert_eq!(Ok(Some(RedisResult::Integer(1))), parser.get_response());

        parser.feed(":".as_bytes());
        assert_eq!(Ok(None), parser.get_response());

        parser.feed("412".as_bytes());
        assert_eq!(Ok(None), parser.get_response());

        parser.feed("\r\n".as_bytes());
        assert_eq!(Ok(Some(RedisResult::Integer(412))), parser.get_response());
    }

    #[test]
    fn doesnt_fail_on_an_empty_string() {
        let mut parser = ResponseParser::new();
        parser.feed("".as_bytes());
        assert_eq!(Ok(None), parser.get_response())
    }

    #[test]
    fn can_parse_a_simple_string() {
        let mut parser = ResponseParser::new();
        parser.feed("+OK\r\n".as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::String(b"OK".to_vec()))),
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
        parser.feed(format!("+{}\r\n", text).as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::String(text.into_bytes()))),
            parser.get_response()
        );
        TestResult::passed()
    }

    #[test]
    fn can_parse_an_error() {
        let mut parser = ResponseParser::new();
        parser.feed("-OK\r\n".as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::Error(RedisErrorValue::new("OK")))),
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
        parser.feed(format!("-{}\r\n", text).as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::Error(RedisErrorValue::new(text)))),
            parser.get_response()
        );
        TestResult::passed()
    }

    #[test]
    fn can_parse_a_bulk_string() {
        let mut parser = ResponseParser::new();
        parser.feed("$2\r\nOK\r\n".as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::String(b"OK".to_vec()))),
            parser.get_response()
        );
    }

    #[test]
    fn can_parse_an_empty_bulk_string() {
        // probably caught already in the quickcheck test, but we might as well
        // double check here that it's recognised.  This is currently special cased
        // in the parser.
        let mut parser = ResponseParser::new();
        parser.feed("$0\r\n\r\n".as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::String(b"".to_vec()))),
            parser.get_response()
        );
    }

    #[test]
    fn doesnt_fail_on_a_negative_bulk_string_size() {
        let mut parser = ResponseParser::new();
        parser.feed("$-100\r\n".as_bytes());
        assert_eq!(
            Err(ParseError::InvalidBulkStringLength(-100)),
            parser.get_response()
        )
    }

    #[quickcheck]
    fn qc_can_parse_any_bulk_string(text: String) {
        let mut parser = ResponseParser::new();
        parser.feed(format!("${}\r\n{}\r\n", text.len(), text).as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::String(text.into_bytes()))),
            parser.get_response()
        );
    }

    #[test]
    fn can_parse_the_null_bulk_string() {
        let mut parser = ResponseParser::new();
        parser.feed("$-1\r\n".as_bytes());
        assert_eq!(Ok(Some(RedisResult::Null)), parser.get_response());
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
             -Bar\r\n"
                .as_bytes(),
        );
        assert_eq!(
            Ok(Some(RedisResult::Array(vec![
                RedisResult::Array(vec![
                    RedisResult::Integer(1),
                    RedisResult::Integer(2),
                    RedisResult::Integer(3),
                ]),
                RedisResult::Array(vec![
                    RedisResult::String(b"Foo".to_vec()),
                    RedisResult::Error(RedisErrorValue::new("Bar")),
                ])
            ]))),
            parser.get_response()
        );
    }

    #[test]
    fn can_parse_an_empty_array() {
        let mut parser = ResponseParser::new();
        parser.feed("*0\r\n".as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::Array(Vec::new()))),
            parser.get_response()
        );
    }

    #[test]
    fn can_parse_a_null_array() {
        let mut parser = ResponseParser::new();
        parser.feed("*-1\r\n".as_bytes());
        assert_eq!(Ok(Some(RedisResult::Null)), parser.get_response());
    }

    #[test]
    fn doesnt_fail_on_a_negative_array_size() {
        let mut parser = ResponseParser::new();
        parser.feed("*-100\r\n".as_bytes());
        assert_eq!(
            Err(ParseError::InvalidArrayLength(-100)),
            parser.get_response()
        )
    }

    #[test]
    fn doesnt_fail_with_array_of_one_zero_int() {
        let mut parser = ResponseParser::new();
        parser.feed("*1\r\n:0\r\n".as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::Array(vec![RedisResult::Integer(0),]))),
            parser.get_response()
        )
    }

    #[test]
    fn doesnt_fail_with_array_of_two_zero_ints() {
        let mut parser = ResponseParser::new();
        parser.feed("*2\r\n:0\r\n:0\r\n".as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::Array(vec![
                RedisResult::Integer(0),
                RedisResult::Integer(0),
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

        let redis_array = ints.iter().map(|int| RedisResult::Integer(*int)).collect();

        let mut parser = ResponseParser::new();
        parser.feed(text.as_bytes());
        assert_eq!(
            Ok(Some(RedisResult::Array(redis_array))),
            parser.get_response()
        );
    }

    #[quickcheck]
    fn qc_can_attempt_to_parse_anything_without_panicking(input: String) {
        let mut parser = ResponseParser::new();
        parser.feed(input.as_bytes());
        loop {
            match parser.get_response() {
                Ok(Some(_)) => continue,
                Ok(None) => break, // input is used up
                Err(_) => break,   // parsing encountered an error
            }
        }
    }

    #[test]
    fn returning_ok_then_a_value_then_null() {
        let mut parser = ResponseParser::new();
        parser.feed("+OK\r\n$1\r\n0\r\n$-1\r\n".as_bytes());

        assert_eq!(
            Some(RedisResult::String("OK".into())),
            parser.get_response().unwrap(),
        );

        assert_eq!(
            Some(RedisResult::String("0".into())),
            parser.get_response().unwrap(),
        );

        // this is where the weird bug happens
        assert_eq!(Some(RedisResult::Null), parser.get_response().unwrap());
    }

    #[test]
    fn can_handle_very_long_arrays() {
        let mut parser = ResponseParser::new();
        parser.feed(
            "+OK\r\n\
             *28\r\n\
             $2\r\n\
             49\r\n\
             $3\r\n\
             -58\r\n\
             $2\r\n\
             59\r\n\
             $1\r\n\
             7\r\n\
             $2\r\n\
             23\r\n\
             $2\r\n\
             43\r\n\
             $2\r\n\
             77\r\n\
             $3\r\n\
             -36\r\n\
             $2\r\n\
             66\r\n\
             $3\r\n\
             -57\r\n\
             $3\r\n\
             -55\r\n\
             $2\r\n\
             95\r\n\
             $2\r\n\
             51\r\n\
             $3\r\n\
             -74\r\n\
             $3\r\n\
             -47\r\n\
             $3\r\n\
             -90\r\n\
             $3\r\n\
             -65\r\n\
             $2\r\n\
             41\r\n\
             $2\r\n\
             77\r\n\
             $2\r\n\
             47\r\n\
             $2\r\n\
             87\r\n\
             $2\r\n\
             24\r\n\
             $2\r\n\
             25\r\n\
             $2\r\n\
             -2\r\n\
             $3\r\n\
             -27\r\n\
             $2\r\n\
             63\r\n\
             $3\r\n\
             -20\r\n\
             $2\r\n\
             90\r\n"
                .as_bytes(),
        );

        assert_eq!(
            Some(RedisResult::String("OK".into())),
            parser.get_response().unwrap()
        );

        assert_eq!(
            Some(RedisResult::Array(vec![
                RedisResult::String("49".into()),
                RedisResult::String("-58".into()),
                RedisResult::String("59".into()),
                RedisResult::String("7".into()),
                RedisResult::String("23".into()),
                RedisResult::String("43".into()),
                RedisResult::String("77".into()),
                RedisResult::String("-36".into()),
                RedisResult::String("66".into()),
                RedisResult::String("-57".into()),
                RedisResult::String("-55".into()),
                RedisResult::String("95".into()),
                RedisResult::String("51".into()),
                RedisResult::String("-74".into()),
                RedisResult::String("-47".into()),
                RedisResult::String("-90".into()),
                RedisResult::String("-65".into()),
                RedisResult::String("41".into()),
                RedisResult::String("77".into()),
                RedisResult::String("47".into()),
                RedisResult::String("87".into()),
                RedisResult::String("24".into()),
                RedisResult::String("25".into()),
                RedisResult::String("-2".into()),
                RedisResult::String("-27".into()),
                RedisResult::String("63".into()),
                RedisResult::String("-20".into()),
                RedisResult::String("90".into()),
            ])),
            parser.get_response().unwrap()
        )
    }
}
