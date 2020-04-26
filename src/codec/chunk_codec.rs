use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

use bytes::{Buf, BufMut, BytesMut};
use std::{cmp, fmt, io, str, usize};

/// A simple [`Decoder`] and [`Encoder`] implementation that splits up data into chunks.
///
/// [`Decoder`]: crate::codec::Decoder
/// [`Encoder`]: crate::codec::Encoder
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ChunkCodec {
    // the byte to use to end a chunk (message)
    delimiter: u8,

    // Stored index of the next index to examine for a `\n` character.
    // This is used to optimize searching.
    // For example, if `decode` was called with `abc`, it would hold `3`,
    // because that is the next index to examine.
    // The next time `decode` is called with `abcde\n`, the method will
    // only look at `de\n` before returning.
    next_index: usize,

    /// The maximum length for a given chunk. If `usize::MAX`, chunks will be
    /// read until a `\n` character is reached.
    max_length: usize,

    /// Are we currently discarding the remainder of a chunk which was over
    /// the length limit?
    is_discarding: bool,
}

impl ChunkCodec {
    /// Returns a `ChunkCodec` for splitting up data into chunks.
    ///
    /// # Note
    ///
    /// The returned `ChunkCodec` will not have an upper bound on the length
    /// of a buffered chunk. See the documentation for [`new_with_max_length`]
    /// for information on why this could be a potential security risk.
    ///
    /// [`new_with_max_length`]: crate::codec::ChunkCodec::new_with_max_length()
    pub fn new(delimiter: u8) -> ChunkCodec {
        ChunkCodec {
            delimiter,
            next_index: 0,
            max_length: usize::MAX,
            is_discarding: false,
        }
    }

    /// Returns a `ChunkCodec` with a maximum chunk length limit.
    ///
    /// If this is set, calls to `ChunkCodec::decode` will return a
    /// [`ChunkCodecError`] when a chunk exceeds the length limit. Subsequent calls
    /// will discard up to `limit` bytes from that chunk until a newchunk
    /// character is reached, returning `None` until the chunk over the limit
    /// has been fully discarded. After that point, calls to `decode` will
    /// function as normal.
    ///
    /// # Note
    ///
    /// Setting a length limit is highly recommended for any `ChunkCodec` which
    /// will be exposed to untrusted input. Otherwise, the size of the buffer
    /// that holds the chunk currently being read is unbounded. An attacker could
    /// exploit this unbounded buffer by sending an unbounded amount of input
    /// without any `\n` characters, causing unbounded memory consumption.
    ///
    /// [`ChunkCodecError`]: crate::codec::ChunkCodecError
    #[allow(dead_code)]
    pub fn new_with_max_length(delimiter: u8, max_length: usize) -> Self {
        ChunkCodec {
            max_length,
            ..ChunkCodec::new(delimiter)
        }
    }
}

fn utf8(buf: &[u8]) -> Result<&str, io::Error> {
    str::from_utf8(buf)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Unable to decode input as UTF8"))
}

impl Decoder for ChunkCodec {
    type Item = String;
    type Error = ChunkCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<String>, ChunkCodecError> {
        loop {
            // Determine how far into the buffer we'll search for a newchunk. If
            // there's no max_length set, we'll read to the end of the buffer.
            let read_to = cmp::min(self.max_length.saturating_add(1), buf.len());

            let newchunk_offset = buf[self.next_index..read_to]
                .iter()
                .position(|b| *b == self.delimiter);

            match (self.is_discarding, newchunk_offset) {
                (true, Some(offset)) => {
                    // If we found a newchunk, discard up to that offset and
                    // then stop discarding. On the next iteration, we'll try
                    // to read a chunk normally.
                    buf.advance(offset + self.next_index + 1);
                    self.is_discarding = false;
                    self.next_index = 0;
                }
                (true, None) => {
                    // Otherwise, we didn't find a newchunk, so we'll discard
                    // everything we read. On the next iteration, we'll continue
                    // discarding up to max_len bytes unless we find a newchunk.
                    buf.advance(read_to);
                    self.next_index = 0;
                    if buf.is_empty() {
                        return Err(ChunkCodecError::MaxChunkLengthExceeded);
                    }
                }
                (false, Some(offset)) => {
                    // Found a chunk!
                    let newchunk_index = offset + self.next_index;
                    self.next_index = 0;
                    let chunk = buf.split_to(newchunk_index + 1);
                    let chunk = &chunk[..chunk.len() - 1];
                    let chunk = utf8(chunk)?;
                    return Ok(Some(chunk.to_string()));
                }
                (false, None) if buf.len() > self.max_length => {
                    // Reached the maximum length without finding a
                    // newchunk, return an error and start discarding on the
                    // next call.
                    self.is_discarding = true;
                    return Err(ChunkCodecError::MaxChunkLengthExceeded);
                }
                (false, None) => {
                    // We didn't find a chunk or reach the length limit, so the next
                    // call will resume searching at the current offset.
                    self.next_index = read_to;
                    return Ok(None);
                }
            }
        }
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<String>, ChunkCodecError> {
        Ok(match self.decode(buf)? {
            Some(frame) => Some(frame),
            None => {
                // No terminating newchunk - return remaining data, if any
                if buf.is_empty() || buf == &b"\r"[..] {
                    None
                } else {
                    let chunk = buf.split_to(buf.len());
                    let chunk = utf8(&chunk)?;
                    self.next_index = 0;
                    Some(chunk.to_string())
                }
            }
        })
    }
}

impl<T> Encoder<T> for ChunkCodec
where
    T: AsRef<str>,
{
    type Error = ChunkCodecError;

    fn encode(&mut self, chunk: T, buf: &mut BytesMut) -> Result<(), ChunkCodecError> {
        let chunk = chunk.as_ref();
        buf.reserve(chunk.len() + 1);
        buf.put(chunk.as_bytes());
        buf.put_u8(self.delimiter);
        Ok(())
    }
}

impl Default for ChunkCodec {
    fn default() -> Self {
        Self::new(b'\n')
    }
}

/// An error occured while encoding or decoding a chunk.
#[derive(Debug)]
pub enum ChunkCodecError {
    /// The maximum chunk length was exceeded.
    MaxChunkLengthExceeded,
    /// An IO error occured.
    Io(io::Error),
}

impl fmt::Display for ChunkCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkCodecError::MaxChunkLengthExceeded => write!(f, "max chunk length exceeded"),
            ChunkCodecError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl From<io::Error> for ChunkCodecError {
    fn from(e: io::Error) -> ChunkCodecError {
        ChunkCodecError::Io(e)
    }
}

impl std::error::Error for ChunkCodecError {}
