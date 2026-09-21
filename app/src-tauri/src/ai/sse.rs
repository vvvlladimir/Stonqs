//! Server-Sent Events framing, nothing else: `event:`/`data:` lines, blocks separated by a blank
//! line. What the event *names* mean is `openai.rs`'s business, not this file's — a second
//! SSE-based provider reuses this reader unchanged.

use std::io::{BufRead, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
}

pub struct SseReader<R> {
    lines: std::io::Lines<R>,
}

impl<R: BufRead> SseReader<R> {
    pub fn new(reader: R) -> Self {
        SseReader {
            lines: reader.lines(),
        }
    }
}

impl<R: BufRead> Iterator for SseReader<R> {
    type Item = Result<SseEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut event = SseEvent::default();
        let mut data_lines: Vec<String> = Vec::new();
        loop {
            match self.lines.next() {
                None => {
                    return if data_lines.is_empty() {
                        None
                    } else {
                        event.data = data_lines.join("\n");
                        Some(Ok(event))
                    };
                }
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(line)) => {
                    if line.is_empty() {
                        // The blank line ends the block; an empty block (stray keep-alive
                        // newline) is not a message.
                        if data_lines.is_empty() {
                            continue;
                        }
                        event.data = data_lines.join("\n");
                        return Some(Ok(event));
                    } else if let Some(rest) = line.strip_prefix("event:") {
                        event.event = rest.trim().to_string();
                    } else if let Some(rest) = line.strip_prefix("data:") {
                        data_lines.push(rest.trim_start().to_string());
                    }
                    // `id:`, `retry:` and comment lines (`:`) carry nothing this reader needs.
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_named_events_split_on_blank_lines() {
        let raw = "event: response.created\ndata: {\"a\":1}\n\nevent: response.output_text.delta\ndata: {\"delta\":\"Hi\"}\n\n";
        let events: Vec<SseEvent> = SseReader::new(Cursor::new(raw)).collect::<Result<_>>().unwrap();
        assert_eq!(
            events,
            vec![
                SseEvent {
                    event: "response.created".into(),
                    data: "{\"a\":1}".into()
                },
                SseEvent {
                    event: "response.output_text.delta".into(),
                    data: "{\"delta\":\"Hi\"}".into()
                },
            ]
        );
    }

    #[test]
    fn joins_a_data_field_split_across_several_lines() {
        let raw = "event: x\ndata: line one\ndata: line two\n\n";
        let events: Vec<SseEvent> = SseReader::new(Cursor::new(raw)).collect::<Result<_>>().unwrap();
        assert_eq!(
            events,
            vec![SseEvent {
                event: "x".into(),
                data: "line one\nline two".into()
            }]
        );
    }

    #[test]
    fn a_trailing_block_with_no_final_blank_line_still_reads() {
        let raw = "event: x\ndata: {}";
        let events: Vec<SseEvent> = SseReader::new(Cursor::new(raw)).collect::<Result<_>>().unwrap();
        assert_eq!(
            events,
            vec![SseEvent {
                event: "x".into(),
                data: "{}".into()
            }]
        );
    }
}
