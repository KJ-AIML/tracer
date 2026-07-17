//! NDJSON transport helpers over arbitrary Read/Write streams.
//!
//! Stderr is **not** part of this transport — process diagnostics stay with
//! `tracer-process`.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use crate::codec::{
    decode_line, encode_message, encode_notification, encode_request, encode_response, FrameDecoder,
};
use crate::error::AcpError;
use crate::message::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

/// One decoded inbound frame outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum InboundFrame {
    /// Valid JSON-RPC message.
    Message(JsonRpcMessage),
    /// Malformed JSON or structural reject (protocol error, not process error).
    Malformed {
        /// Raw line (truncated for safety).
        raw: String,
        /// Error detail.
        error: AcpError,
    },
}

/// Writer for outbound ACP frames.
#[derive(Debug)]
pub struct NdjsonWriter<W> {
    inner: W,
}

impl<W: Write> NdjsonWriter<W> {
    /// Wrap a writer (typically process stdin).
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Write a request.
    pub fn write_request(&mut self, req: &JsonRpcRequest) -> Result<(), AcpError> {
        let bytes = encode_request(req)?;
        self.inner
            .write_all(&bytes)
            .map_err(|e| AcpError::write_failed(format!("stdin write failed: {e}")))?;
        self.inner
            .flush()
            .map_err(|e| AcpError::write_failed(format!("stdin flush failed: {e}")))?;
        Ok(())
    }

    /// Write a notification.
    pub fn write_notification(&mut self, n: &JsonRpcNotification) -> Result<(), AcpError> {
        let bytes = encode_notification(n)?;
        self.inner
            .write_all(&bytes)
            .map_err(|e| AcpError::write_failed(format!("stdin write failed: {e}")))?;
        self.inner
            .flush()
            .map_err(|e| AcpError::write_failed(format!("stdin flush failed: {e}")))?;
        Ok(())
    }

    /// Write a response (permission answer).
    pub fn write_response(&mut self, r: &JsonRpcResponse) -> Result<(), AcpError> {
        let bytes = encode_response(r)?;
        self.inner
            .write_all(&bytes)
            .map_err(|e| AcpError::write_failed(format!("stdin write failed: {e}")))?;
        self.inner
            .flush()
            .map_err(|e| AcpError::write_failed(format!("stdin flush failed: {e}")))?;
        Ok(())
    }

    /// Write an arbitrary message.
    pub fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<(), AcpError> {
        let bytes = encode_message(msg)?;
        self.inner
            .write_all(&bytes)
            .map_err(|e| AcpError::write_failed(format!("stdin write failed: {e}")))?;
        self.inner
            .flush()
            .map_err(|e| AcpError::write_failed(format!("stdin flush failed: {e}")))?;
        Ok(())
    }

    /// Consume the writer (e.g. drop to close stdin).
    pub fn into_inner(self) -> W {
        self.inner
    }

    /// Mutable access to the inner writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }
}

/// Reader that accumulates partial reads into NDJSON frames.
#[derive(Debug)]
pub struct NdjsonReader<R> {
    inner: R,
    decoder: FrameDecoder,
    eof: bool,
    read_buf: Vec<u8>,
}

impl<R: Read> NdjsonReader<R> {
    /// Wrap a reader (typically process stdout).
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            decoder: FrameDecoder::new(),
            eof: false,
            read_buf: vec![0u8; 8192],
        }
    }

    /// Whether EOF has been observed.
    pub fn is_eof(&self) -> bool {
        self.eof
    }

    /// Non-blocking-ish: read available data once and return any complete frames.
    ///
    /// On blocking readers this may block until some data arrives. Callers that
    /// need timeouts should use a thread + channel (see runtime adapter).
    pub fn read_frames(&mut self) -> Result<Vec<InboundFrame>, AcpError> {
        if self.eof {
            return Ok(Vec::new());
        }
        let n = self
            .inner
            .read(&mut self.read_buf)
            .map_err(|e| AcpError::unexpected_eof(format!("stdout read failed: {e}")))?;
        if n == 0 {
            self.eof = true;
            // leftover incomplete frame → malformed / unexpected
            if let Some(rem) = self.decoder.take_remainder() {
                return Ok(vec![InboundFrame::Malformed {
                    raw: rem.chars().take(512).collect(),
                    error: AcpError::parse("incomplete frame at EOF"),
                }]);
            }
            return Ok(Vec::new());
        }
        let lines = self.decoder.push(&self.read_buf[..n]);
        Ok(lines
            .into_iter()
            .map(|line| match decode_line(&line) {
                Ok(Some(msg)) => InboundFrame::Message(msg),
                Ok(None) => InboundFrame::Malformed {
                    raw: line,
                    error: AcpError::parse("empty frame after decode"),
                },
                Err(e) => InboundFrame::Malformed {
                    raw: line.chars().take(512).collect(),
                    error: e,
                },
            })
            .collect())
    }

    /// Read until at least one frame, EOF, or timeout (busy-loop with sleep).
    ///
    /// Prefer the adapter's threaded reader for production use; this helper is
    /// for unit tests with in-memory pipes.
    pub fn read_frames_until(
        &mut self,
        timeout: Duration,
        mut pred: impl FnMut(&[InboundFrame]) -> bool,
    ) -> Result<Vec<InboundFrame>, AcpError> {
        let deadline = Instant::now() + timeout;
        let mut acc = Vec::new();
        loop {
            let frames = self.read_frames()?;
            if !frames.is_empty() {
                acc.extend(frames);
                if pred(&acc) {
                    return Ok(acc);
                }
            }
            if self.eof {
                if acc.is_empty() {
                    return Err(AcpError::clean_eof());
                }
                return Ok(acc);
            }
            if Instant::now() >= deadline {
                if acc.is_empty() {
                    return Err(AcpError::timeout("timed out waiting for ACP frames"));
                }
                return Ok(acc);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Consume reader.
    pub fn into_inner(self) -> R {
        self.inner
    }
}
