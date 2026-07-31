//! Server-sent events frame parser.
//!
//! Pure and byte-oriented: a network chunk can split a frame anywhere, so the
//! parser buffers and only emits complete events. No network types appear here,
//! which is what makes it unit-testable.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseEvent {
    /// One dispatched `data:` payload (multi-line data joined with `\n`).
    Data(String),
    /// The OpenAI-style `data: [DONE]` sentinel.
    Done,
}

#[derive(Debug, Default)]
pub struct SseParser {
    /// Bytes not yet forming a complete line.
    buf: Vec<u8>,
    /// `data:` lines of the event currently being accumulated.
    data: Vec<String>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk of bytes; returns every event that became complete.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buf.extend_from_slice(chunk);
        let mut events = Vec::new();

        while let Some(pos) = self.buf.iter().position(|b| *b == b'\n') {
            let mut line: Vec<u8> = self.buf.drain(..=pos).collect();
            line.pop(); // '\n'
            if line.last() == Some(&b'\r') {
                line.pop(); // CRLF
            }
            // Invalid UTF-8 cannot be recovered from mid-stream; replacing is
            // better than dropping the rest of the answer.
            let line = String::from_utf8_lossy(&line).into_owned();
            if let Some(event) = self.line(&line) {
                events.push(event);
            }
        }

        events
    }

    /// Flush an event that the stream ended without a trailing blank line.
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&std::mem::take(&mut self.buf)).into_owned();
            if let Some(event) = self.line(&line) {
                events.push(event);
            }
        }
        if let Some(event) = self.dispatch() {
            events.push(event);
        }
        events
    }

    fn line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            return self.dispatch();
        }
        // Comments (heartbeats such as `: keep-alive`) are ignored.
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            // A field with no colon has an empty value; none of those interest us.
            None => (line, ""),
        };
        if field == "data" {
            self.data.push(value.to_string());
        }
        // `event:`, `id:` and `retry:` carry nothing we act on.
        None
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        if self.data.is_empty() {
            return None;
        }
        let payload = std::mem::take(&mut self.data).join("\n");
        if payload.trim() == "[DONE]" {
            Some(SseEvent::Done)
        } else {
            Some(SseEvent::Data(payload))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(chunks: &[&str]) -> Vec<SseEvent> {
        let mut parser = SseParser::new();
        let mut events = Vec::new();
        for chunk in chunks {
            events.extend(parser.push(chunk.as_bytes()));
        }
        events.extend(parser.finish());
        events
    }

    #[test]
    fn parses_simple_frames() {
        assert_eq!(
            collect(&["data: one\n\ndata: two\n\n"]),
            vec![SseEvent::Data("one".into()), SseEvent::Data("two".into())]
        );
    }

    #[test]
    fn handles_frames_split_mid_line() {
        assert_eq!(
            collect(&["da", "ta: hel", "lo\n", "\nda", "ta: [DO", "NE]\n\n"]),
            vec![SseEvent::Data("hello".into()), SseEvent::Done]
        );
    }

    #[test]
    fn handles_a_split_inside_the_line_terminator() {
        assert_eq!(
            collect(&["data: x\r", "\n\r\n"]),
            vec![SseEvent::Data("x".into())]
        );
    }

    #[test]
    fn handles_crlf() {
        assert_eq!(
            collect(&["data: a\r\n\r\ndata: b\r\n\r\n"]),
            vec![SseEvent::Data("a".into()), SseEvent::Data("b".into())]
        );
    }

    #[test]
    fn ignores_comments_and_other_fields() {
        assert_eq!(
            collect(&[": keep-alive\n\nevent: message\nid: 7\ndata: a\n\n"]),
            vec![SseEvent::Data("a".into())]
        );
    }

    #[test]
    fn joins_multi_line_data() {
        assert_eq!(
            collect(&["data: {\ndata: \"a\": 1}\n\n"]),
            vec![SseEvent::Data("{\n\"a\": 1}".into())]
        );
    }

    #[test]
    fn tolerates_a_missing_space_after_the_colon() {
        assert_eq!(collect(&["data:a\n\n"]), vec![SseEvent::Data("a".into())]);
    }

    #[test]
    fn flushes_a_final_frame_without_a_blank_line() {
        assert_eq!(
            collect(&["data: tail"]),
            vec![SseEvent::Data("tail".into())]
        );
    }

    #[test]
    fn done_sentinel_is_recognised_with_surrounding_space() {
        assert_eq!(collect(&["data:  [DONE] \n\n"]), vec![SseEvent::Done]);
    }

    #[test]
    fn a_realistic_openai_chunk_sequence() {
        let stream = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"He\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"llo\"}}]}\n\n",
            ": ping\n\n",
            "data: [DONE]\n\n"
        );
        let events = collect(&[stream]);
        assert_eq!(events.len(), 3);
        assert_eq!(events[2], SseEvent::Done);
    }
}
