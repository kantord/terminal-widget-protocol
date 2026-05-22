// Byte-level state machine that recognizes APC sequences (ESC _ ... ST) and
// dispatches those whose payload starts with the `twp;` namespace prefix.
//
// We don't use the `vte` crate here: vte 0.15's Perform trait does not expose
// APC bytes (they fall through to the SosPmApcString state and are silently
// dropped). A small custom scanner is sufficient because the only sequences
// we interpret are APC framing; everything else is forwarded verbatim.

const ESC: u8 = 0x1B;
const BEL: u8 = 0x07;
const CAN: u8 = 0x18;
const SUB: u8 = 0x1A;
const ST_FINAL: u8 = b'\\';
const APC_INTRO: u8 = b'_';

const TWP_PREFIX: &[u8] = b"twp;";

enum State {
    Normal,
    EscPending,
    Apc,
    ApcEsc,
}

pub struct Filter {
    state: State,
    apc_payload: Vec<u8>,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            state: State::Normal,
            apc_payload: Vec::with_capacity(256),
        }
    }

    /// Feeds `input` through the filter, appending bytes to forward to `out`
    /// and invoking `on_twp(payload, out)` for each complete `twp;`-prefixed
    /// APC, where `payload` is the bytes after `twp;`.
    pub fn process<F>(&mut self, input: &[u8], out: &mut Vec<u8>, mut on_twp: F)
    where
        F: FnMut(&[u8], &mut Vec<u8>),
    {
        for &b in input {
            match self.state {
                State::Normal => match b {
                    ESC => self.state = State::EscPending,
                    _ => out.push(b),
                },
                State::EscPending => match b {
                    APC_INTRO => {
                        self.state = State::Apc;
                        self.apc_payload.clear();
                    }
                    ESC => {
                        // Previous ESC didn't start an APC; flush it and stay pending.
                        out.push(ESC);
                    }
                    _ => {
                        out.push(ESC);
                        out.push(b);
                        self.state = State::Normal;
                    }
                },
                State::Apc => match b {
                    BEL => {
                        self.dispatch(out, &mut on_twp);
                        self.state = State::Normal;
                    }
                    ESC => self.state = State::ApcEsc,
                    CAN | SUB => {
                        // Abort: drop the in-flight APC silently.
                        self.apc_payload.clear();
                        self.state = State::Normal;
                    }
                    _ => self.apc_payload.push(b),
                },
                State::ApcEsc => match b {
                    ST_FINAL => {
                        self.dispatch(out, &mut on_twp);
                        self.state = State::Normal;
                    }
                    ESC => {
                        // Abort current APC, treat new ESC as start of next sequence.
                        self.apc_payload.clear();
                        self.state = State::EscPending;
                    }
                    _ => {
                        // ESC followed by something other than ST_FINAL: treat
                        // both as APC body bytes and resume.
                        self.apc_payload.push(ESC);
                        self.apc_payload.push(b);
                        self.state = State::Apc;
                    }
                },
            }
        }
    }

    fn dispatch<F>(&mut self, out: &mut Vec<u8>, on_twp: &mut F)
    where
        F: FnMut(&[u8], &mut Vec<u8>),
    {
        if self.apc_payload.starts_with(TWP_PREFIX) {
            on_twp(&self.apc_payload[TWP_PREFIX.len()..], out);
        } else {
            // Unknown APC: pass it through unchanged so other terminal
            // protocols (Kitty Graphics, etc.) keep working.
            out.push(ESC);
            out.push(APC_INTRO);
            out.extend_from_slice(&self.apc_payload);
            out.push(ESC);
            out.push(ST_FINAL);
        }
        self.apc_payload.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
        let mut filter = Filter::new();
        let mut out = Vec::new();
        let mut hits = Vec::new();
        filter.process(input, &mut out, |payload, _| hits.push(payload.to_vec()));
        (out, hits)
    }

    #[test]
    fn passes_plain_bytes_through() {
        let (out, hits) = run(b"hello world\n");
        assert_eq!(out, b"hello world\n");
        assert!(hits.is_empty());
    }

    #[test]
    fn intercepts_twp_with_st() {
        let (out, hits) = run(b"a\x1b_twp;foo\x1b\\b");
        assert_eq!(out, b"ab");
        assert_eq!(hits, vec![b"foo".to_vec()]);
    }

    #[test]
    fn intercepts_twp_with_bel() {
        let (out, hits) = run(b"a\x1b_twp;bar\x07b");
        assert_eq!(out, b"ab");
        assert_eq!(hits, vec![b"bar".to_vec()]);
    }

    #[test]
    fn passes_unknown_apc_through() {
        // A Kitty Graphics APC; we don't recognize it but must forward intact.
        let (out, hits) = run(b"\x1b_Ga=T,i=1;Zm9v\x1b\\");
        assert_eq!(out, b"\x1b_Ga=T,i=1;Zm9v\x1b\\");
        assert!(hits.is_empty());
    }

    #[test]
    fn esc_not_followed_by_underscore_is_forwarded() {
        let (out, hits) = run(b"\x1b[1mbold\x1b[0m");
        assert_eq!(out, b"\x1b[1mbold\x1b[0m");
        assert!(hits.is_empty());
    }

    #[test]
    fn handles_split_chunks() {
        let mut filter = Filter::new();
        let mut out = Vec::new();
        let mut hits: Vec<Vec<u8>> = Vec::new();
        let chunks: &[&[u8]] = &[b"x\x1b", b"_twp;foo", b"\x1b\\y"];
        for chunk in chunks {
            filter.process(chunk, &mut out, |payload, _| hits.push(payload.to_vec()));
        }
        assert_eq!(out, b"xy");
        assert_eq!(hits, vec![b"foo".to_vec()]);
    }
}
