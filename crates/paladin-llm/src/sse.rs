//! Line reassembly for streamed Server-Sent Events.
//!
//! A vendor's SSE body reaches an adapter as a sequence of network reads whose boundaries have
//! nothing to do with event boundaries: one read can end in the middle of a `data:` line, or in
//! the middle of a multi-byte character. Parsing each read on its own drops the event that
//! straddles the boundary, and the phase's terminal-frame guarantees (refusal, empty completion,
//! held stop reason, metadata) are decided at exactly one frame -- so losing that frame, or a
//! content frame before it, makes the guard report the wrong thing or nothing at all.
//!
//! [`SseLines`] holds the bytes of an incomplete line until its newline arrives and decodes only
//! whole lines, so a character split across two reads is decoded intact. Every streaming adapter
//! feeds it the raw bytes and loops over the complete lines it returns.
//!
//! The stream's end is a boundary too: [`SseLines::finish`] returns a final line that had no
//! trailing newline, which a transport that omits the last newline would otherwise lose.

/// The most bytes held for a single unterminated line. A vendor event is far smaller; the bound
/// only stops a peer that never sends a newline from growing the buffer without limit.
const MAX_PENDING_BYTES: usize = 4 * 1024 * 1024;

/// Reassembles complete text lines from arbitrarily split network reads.
#[derive(Debug, Default)]
pub(crate) struct SseLines {
    pending: Vec<u8>,
}

impl SseLines {
    /// Feed one network read; return every line it completed, without its line terminator.
    ///
    /// A trailing `\r` is stripped so `\r\n` framing reads the same as `\n`. Bytes after the last
    /// newline stay buffered for the next call.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(bytes);
        let mut lines = Vec::new();
        while let Some(newline) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=newline).collect();
            lines.push(decode_line(&line));
        }
        if self.pending.len() > MAX_PENDING_BYTES {
            log::warn!(
                target: "paladin::sse",
                "discarding {} bytes of an unterminated stream line",
                self.pending.len()
            );
            self.pending.clear();
        }
        lines
    }

    /// The stream ended: return the held final line, if it had no trailing newline.
    pub(crate) fn finish(&mut self) -> Vec<String> {
        if self.pending.is_empty() {
            return Vec::new();
        }
        let line = std::mem::take(&mut self.pending);
        vec![decode_line(&line)]
    }
}

/// Decode one complete line, dropping its `\n` / `\r\n` terminator. Only a genuinely invalid
/// byte sequence is replaced; a character split across reads has been reassembled by now.
fn decode_line(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    text.trim_end_matches(['\r', '\n']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_split_across_reads_is_returned_once_complete() {
        let mut lines = SseLines::default();
        assert!(lines.push(b"data: {\"a\":").is_empty());
        assert_eq!(lines.push(b"1}\n\n"), vec!["data: {\"a\":1}", ""]);
    }

    #[test]
    fn several_lines_in_one_read_are_all_returned() {
        let mut lines = SseLines::default();
        assert_eq!(
            lines.push(b"data: one\r\ndata: two\n"),
            vec!["data: one", "data: two"]
        );
    }

    #[test]
    fn a_multibyte_character_split_across_reads_is_decoded_intact() {
        let bytes = "data: h\u{e9}llo\n".as_bytes();
        // Split inside the two-byte `e-acute`.
        let split = bytes.iter().position(|b| *b == 0xC3).unwrap_or(0) + 1;
        let mut lines = SseLines::default();
        assert!(lines.push(&bytes[..split]).is_empty());
        assert_eq!(lines.push(&bytes[split..]), vec!["data: h\u{e9}llo"]);
    }

    #[test]
    fn a_final_line_without_a_newline_is_returned_by_finish() {
        let mut lines = SseLines::default();
        assert!(lines.push(b"data: [DONE]").is_empty());
        assert_eq!(lines.finish(), vec!["data: [DONE]"]);
        assert!(lines.finish().is_empty());
    }

    #[test]
    fn an_unterminated_line_past_the_bound_is_discarded() {
        let mut lines = SseLines::default();
        let big = vec![b'x'; MAX_PENDING_BYTES + 1];
        assert!(lines.push(&big).is_empty());
        assert!(lines.finish().is_empty());
    }
}
