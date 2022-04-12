//! A peekable codepoint reader.

use std::io::{Bytes, Read};

use anyhow::{Error, Result};
use unicode_reader::CodePoints;

/// A codepoint reader supports peeking.
/// Peek/pop char/string from internal char reader.
pub struct PeekableCodePoints<R>
where
    R: Read,
{
    /// internal char reader
    codepoints: CodePoints<Bytes<R>>,

    /// internal char buffer
    buffer: Vec<char>,
}

impl<R: Read> PeekableCodePoints<R> {
    /// Create a PeekableCodePoints from a instance that implements Read trait
    pub fn new(reader: R) -> Self {
        PeekableCodePoints {
            codepoints: CodePoints::from(reader),
            buffer: Vec::new(),
        }
    }

    /// Fill internal buffer with chars of desired count.
    /// Returns actual count of chars filled if not enough chars found.
    fn feed_buffer(&mut self, count: usize) -> Result<usize> {
        for i in 0..count {
            let item = self.codepoints.next();
            match item {
                None => return Ok(i),
                Some(Err(e)) => return Err(Error::new(e)),
                Some(Ok(c)) => self.buffer.push(c),
            }
        }

        Ok(count)
    }

    /// Peek a string, composed of count number of chars.
    /// If there are not enough chars, using actual number of chars remaining.
    pub fn peek(&mut self, count: usize) -> Result<String> {
        if count <= self.buffer.len() {
            let combined_str = self.buffer[..count].iter().collect();
            Ok(combined_str)
        } else {
            self.feed_buffer(count - self.buffer.len())?;
            let combined_str = self.buffer.iter().collect();
            Ok(combined_str)
        }
    }

    /// Drop count number of chars from internal buffer.
    /// If not enough chars in buffer, drop actual number remaining.
    fn discard_buffer(&mut self, count: usize) {
        let actual_count = if count > self.buffer.len() {
            self.buffer.len()
        } else {
            count
        };

        self.buffer.drain(0..actual_count);
    }

    /// Pop a string, composed of count number of chars.
    /// If not enough chars found, using actual number remaining.
    pub fn pop(&mut self, count: usize) -> Result<String> {
        let pop_str = self.peek(count)?;
        self.discard_buffer(pop_str.len());

        Ok(pop_str)
    }

    /// Peek a single char, at specific index.
    /// If index is out of the range of actual chars remaining, return None.
    pub fn peek_char(&mut self, index: usize) -> Result<Option<char>> {
        if index >= self.buffer.len() {
            self.feed_buffer(index + 1 - self.buffer.len())?;
        }

        if self.buffer.len() <= index {
            return Ok(None);
        }

        Ok(Some(self.buffer[index]))
    }

    /// Drop count number of chars and move ahead to the ones following.
    /// If not enough chars found, drop actual remaining.
    pub fn skip(&mut self, count: usize) -> Result<()> {
        if count > self.buffer.len() {
            self.feed_buffer(count - self.buffer.len())?;
        }

        self.discard_buffer(count);

        Ok(())
    }
}
