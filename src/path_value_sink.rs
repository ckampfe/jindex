use std::io::Write;

use crate::{PathComponent, PathValue};
use anyhow::Result;

/// `jindex` will call this trait's `handle_pathvalue` method
/// exactly once for each `PathValue` in the given JSON document.
/// This trait is not specific about what implementors
/// should do with each `PathValue`:
/// they could format `PathValue`s and write them as bytes (as `GronWriter` does),
/// collect them in an internal buffer for further processing,
/// discard them, filter specific ones out, or anything else.
///
/// Note that `handle_pathvalue` is on the hot path of `jindex`,
/// so the performance of `jindex` will depend heavily on how a
/// given type implements `handle_pathvalue`.
pub trait PathValueSink {
    fn handle_pathvalue(&mut self, pathvalue: &PathValue) -> Result<()>;
}

/// Write `PathValue`s to the given `writer` in the style of
/// https://github.com/tomnomnom/gron
#[derive(Debug)]
pub struct GronWriter<'a, W: Write> {
    writer: &'a mut W,
}

impl<'a, W: Write> GronWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }
}

impl<'a, W: Write> PathValueSink for GronWriter<'a, W> {
    #[inline]
    fn handle_pathvalue(&mut self, pathvalue: &PathValue) -> Result<()> {
        self.writer.write_all(b"json")?;

        for path_component in &pathvalue.path_components {
            match path_component {
                PathComponent::Identifier(s) => {
                    self.writer.write_all(b".")?;
                    self.writer.write_all(s.as_bytes())?;
                }
                PathComponent::NonIdentifier(s) => {
                    self.writer.write_all(b"[\"")?;
                    self.writer.write_all(s.as_bytes())?;
                    self.writer.write_all(b"\"]")?;
                }
                PathComponent::Index(i) => {
                    self.writer.write_all(b"[")?;
                    itoa::write(&mut *self.writer, *i)?;
                    self.writer.write_all(b"]")?;
                }
            }
        }

        self.writer.write_all(b" = ")?;

        match pathvalue.value {
            serde_json::Value::Array(_) => self.writer.write_all(b"[]")?,
            serde_json::Value::Object(_) => self.writer.write_all(b"{}")?,
            _ => serde_json::to_writer(&mut *self.writer, pathvalue.value)?,
        }

        self.writer.write_all(b";\n")?;

        Ok(())
    }
}

/// Write `PathValue`s to the given `writer` as
/// JSON Pointers.
/// See https://datatracker.ietf.org/doc/html/rfc6901
#[derive(Debug)]
pub struct JSONPointerWriter<'a, W: Write> {
    writer: &'a mut W,
    options: JSONPointerWriterOptions<'a>,
}

impl<'a, W: Write> JSONPointerWriter<'a, W> {
    pub fn new(writer: &'a mut W, options: JSONPointerWriterOptions<'a>) -> Self {
        Self { writer, options }
    }
}

#[derive(Debug)]
pub struct JSONPointerWriterOptions<'a> {
    pub separator: &'a str,
    pub only_terminals: bool,
}

impl Default for JSONPointerWriterOptions<'_> {
    fn default() -> Self {
        Self {
            separator: "\t",
            only_terminals: true,
        }
    }
}

const TILDE: char = '~';
const FORWARD_SLASH: char = '/';
const JSON_POINTER_SPECIAL_CHARS: &[char] = &[TILDE, FORWARD_SLASH];

impl<'a, W: Write> PathValueSink for JSONPointerWriter<'a, W> {
    fn handle_pathvalue(&mut self, pathvalue: &PathValue) -> Result<()> {
        let should_write = match (self.options.only_terminals, pathvalue.value) {
            (true, serde_json::Value::Array(a)) if !a.is_empty() => false,
            (true, serde_json::Value::Object(o)) if !o.is_empty() => false,
            _ => true,
        };
        let should_write = should_write && !pathvalue.path_components.is_empty();

        if should_write {
            for path_component in &pathvalue.path_components {
                self.writer.write_all(b"/")?;
                match path_component {
                    PathComponent::Identifier(s) | PathComponent::NonIdentifier(s) => {
                        // this conditional exists because `replace` allocates even
                        // if it doesn't find any matches, and I've benchmarked this conditional
                        // as increasing throughput by ~30-50%.
                        if s.contains(JSON_POINTER_SPECIAL_CHARS) {
                            let s = s.replace(TILDE, "~0");
                            let s = s.replace(FORWARD_SLASH, "~1");
                            self.writer.write_all(s.as_bytes())?
                        } else {
                            self.writer.write_all(s.as_bytes())?
                        }
                    }
                    PathComponent::Index(i) => {
                        itoa::write(&mut *self.writer, *i)?;
                    }
                }
            }

            self.writer.write_all(self.options.separator.as_bytes())?;
            serde_json::to_writer(&mut *self.writer, pathvalue.value)?;
            self.writer.write_all(b"\n")?;
        }

        Ok(())
    }
}

/// Write `PathValue`s to the given `writer` as
/// JSON objects separated by newlines,
/// like `{"value":"foo","path_components":["some","paths"]}
#[derive(Debug)]
pub struct JsonWriter<'a, W: Write> {
    writer: &'a mut W,
}

impl<'a, W: Write> JsonWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }
}

impl<'a, W: Write> PathValueSink for JsonWriter<'a, W> {
    fn handle_pathvalue(&mut self, pathvalue: &PathValue) -> Result<()> {
        serde_json::to_writer(&mut *self.writer, pathvalue)?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }
}
