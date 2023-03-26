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
pub struct GronWriter<'writer, W: Write> {
    writer: &'writer mut W,
    options: GronWriterOptions,
}

impl<'writer, W: Write> GronWriter<'writer, W> {
    pub fn new(writer: &'writer mut W, options: GronWriterOptions) -> Self {
        Self { writer, options }
    }
}

#[derive(Debug)]
pub struct GronWriterOptions {
    pub only_scalars: bool,
}

impl Default for GronWriterOptions {
    fn default() -> Self {
        Self { only_scalars: true }
    }
}

impl<'writer, W: Write> PathValueSink for GronWriter<'writer, W> {
    #[inline]
    fn handle_pathvalue(&mut self, pathvalue: &PathValue) -> Result<()> {
        let should_write = if self.options.only_scalars {
            is_scalar(pathvalue.value)
        } else {
            true
        };

        let should_write = should_write && !pathvalue.path_components.is_empty();

        if should_write {
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
                        let mut buf = itoa::Buffer::new();
                        let out = buf.format(*i);
                        self.writer.write_all(out.as_bytes())?;
                        self.writer.write_all(b"]")?;
                    }
                }
            }

            self.writer.write_all(b" = ")?;

            serde_json::to_writer(&mut *self.writer, pathvalue.value)?;

            self.writer.write_all(b";\n")?;
        }

        Ok(())
    }
}

/// Write `PathValue`s to the given `writer` as
/// JSON Pointers.
/// See https://datatracker.ietf.org/doc/html/rfc6901
#[derive(Debug)]
pub struct JSONPointerWriter<'writer, W: Write> {
    writer: &'writer mut W,
    options: JSONPointerWriterOptions<'writer>,
}

impl<'writer, W: Write> JSONPointerWriter<'writer, W> {
    pub fn new(writer: &'writer mut W, options: JSONPointerWriterOptions<'writer>) -> Self {
        Self { writer, options }
    }
}

#[derive(Debug)]
pub struct JSONPointerWriterOptions<'options> {
    pub only_scalars: bool,
    pub separator: &'options str,
}

impl Default for JSONPointerWriterOptions<'_> {
    fn default() -> Self {
        Self {
            only_scalars: true,
            separator: "\t",
        }
    }
}

const TILDE: char = '~';
const FORWARD_SLASH: char = '/';
const JSON_POINTER_SPECIAL_CHARS: &[char] = &[TILDE, FORWARD_SLASH];

impl<'writer, W: Write> PathValueSink for JSONPointerWriter<'writer, W> {
    #[inline]
    fn handle_pathvalue(&mut self, pathvalue: &PathValue) -> Result<()> {
        let should_write = if self.options.only_scalars {
            is_scalar(pathvalue.value)
        } else {
            true
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
                        let mut buf = itoa::Buffer::new();
                        let out = buf.format(*i);
                        self.writer.write_all(out.as_bytes())?;
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
/// like `{"path_components":["some","paths"],"value":"foo"}
#[derive(Debug)]
pub struct JSONWriter<'writer, W: Write> {
    writer: &'writer mut W,
    options: JsonWriterOptions,
}

impl<'writer, W: Write> JSONWriter<'writer, W> {
    pub fn new(writer: &'writer mut W, options: JsonWriterOptions) -> Self {
        Self { writer, options }
    }
}

#[derive(Debug)]
pub struct JsonWriterOptions {
    pub only_scalars: bool,
}

impl Default for JsonWriterOptions {
    fn default() -> Self {
        Self { only_scalars: true }
    }
}

impl<'writer, W: Write> PathValueSink for JSONWriter<'writer, W> {
    #[inline]
    fn handle_pathvalue(&mut self, pathvalue: &PathValue) -> Result<()> {
        let should_write = if self.options.only_scalars {
            is_scalar(pathvalue.value)
        } else {
            true
        };

        let should_write = should_write && !pathvalue.path_components.is_empty();

        if should_write {
            serde_json::to_writer(&mut *self.writer, pathvalue)?;
            self.writer.write_all(b"\n")?;
        }

        Ok(())
    }
}

#[inline]
fn is_scalar(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Bool(_)
        | serde_json::Value::Null => true,
        serde_json::Value::Array(a) if a.is_empty() => true,
        serde_json::Value::Object(o) if o.is_empty() => true,
        _ => false,
    }
}
