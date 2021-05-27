use std::io::{
    Bytes,
    Read,
};

use anyhow::{
    Result,
    Error,
};
use unicode_reader::CodePoints;

pub struct PeekableCodePoints<R>
    where R: Read
{
    codepoints: CodePoints<Bytes<R>>,
    buffer: Vec<char>,
}

impl<R: Read> PeekableCodePoints<R> {
    pub fn new(reader: R) -> Self {
        PeekableCodePoints {
            codepoints: CodePoints::from(reader),
            buffer: Vec::new(),
        }
    }

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

    fn discard_buffer(&mut self, count: usize) {
        let actual_count =
            if count > self.buffer.len() {
                self.buffer.len()
            } else {
                count
            };

        self.buffer.drain(0..actual_count);
    }

    pub fn pop(&mut self, count: usize) -> Result<String> {
        let pop_str = self.peek(count)?;
        self.discard_buffer(pop_str.len());

        Ok(pop_str)
    }

    pub fn peek_char(&mut self, index: usize) -> Result<Option<char>> {
        if index >= self.buffer.len() {
            self.feed_buffer(index + 1 - self.buffer.len())?;
        }

        if self.buffer.len() <= index {
            return Ok(None);
        }

        Ok(Some(self.buffer[index]))
    }

    pub fn skip(&mut self, count: usize) -> Result<()> {
        let feed_count = count - self.buffer.len();
        if feed_count > 0 {
            self.feed_buffer(feed_count)?;
        }

        self.discard_buffer(count);

        Ok(())
    }
}