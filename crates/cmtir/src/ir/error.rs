use ariadne::{Color, ColorGenerator, Fmt, Label, Report, ReportKind, Source};
use json::JsonValue;
use std::fmt::Display;

use super::*;

/// A span with file, start line, start column, end line, end column, start byte, and end byte.
#[derive(Debug, Clone)]
pub struct MySpan {
  pub file: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
  pub start_byte: usize,
  pub end_byte: usize,
}

impl MySpan {
  /// Create a span from a JSON value.
  pub fn from_json(v: &JsonValue) -> Option<Self> {
    match v {
      JsonValue::Object(obj) => {
        let file = obj.get("file")?.as_str()?.to_string();
        let start_line = obj.get("start_line")?.as_u64()? as usize;
        let start_column = obj.get("start_column")?.as_u64()? as usize;
        let end_line = obj.get("end_line")?.as_u64()? as usize;
        let end_column = obj.get("end_column")?.as_u64()? as usize;
        let start_byte = obj.get("start_byte")?.as_u64()? as usize;
        let end_byte = obj.get("end_byte")?.as_u64()? as usize;
        Some(Self {
          file,
          start_line,
          start_column,
          end_line,
          end_column,
          start_byte,
          end_byte,
        })
      }
      _ => None,
    }
  }
}

impl Display for MySpan {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}:{}:{}:{}:{}:{}:{}",
      self.file,
      self.start_line,
      self.start_column,
      self.end_line,
      self.end_column,
      self.start_byte,
      self.end_byte
    )
  }
}

impl Into<JsonValue> for MySpan {
  fn into(self) -> JsonValue {
    let mut span = json::object::Object::new();
    span.insert("file", json::JsonValue::String(self.file));
    span.insert(
      "start_line",
      json::JsonValue::Number(self.start_line.into()),
    );
    span.insert(
      "start_column",
      json::JsonValue::Number(self.start_column.into()),
    );
    span.insert("end_line", json::JsonValue::Number(self.end_line.into()));
    span.insert(
      "end_column",
      json::JsonValue::Number(self.end_column.into()),
    );
    span.insert(
      "start_byte",
      json::JsonValue::Number(self.start_byte.into()),
    );
    span.insert("end_byte", json::JsonValue::Number(self.end_byte.into()));
    JsonValue::Object(span)
  }
}

/// A message with a span.
#[derive(Debug, Clone)]
pub struct SpannedMessage {
  pub message: String,
  pub span: Option<MySpan>,
}

impl Display for SpannedMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{} at {}",
      self.message,
      self
        .span
        .as_ref()
        .map_or("".to_string(), |span| format!("{}", span))
    )
  }
}

/// A collection of messages with spans.
#[derive(Debug, Clone)]
pub struct CmtError {
  pub messages: Vec<SpannedMessage>,
}

impl CmtError {
  /// Create a new empty error.
  pub fn new() -> Self {
    Self { messages: vec![] }
  }
  /// Append a message to the error.
  pub fn append(&mut self, msg: String, span: Option<MySpan>) {
    self.messages.push(SpannedMessage { message: msg, span });
  }
  /// Create an error from a raw message.
  pub fn raw_message(s: String) -> Self {
    Self {
      messages: vec![SpannedMessage {
        message: s,
        span: None,
      }],
    }
  }
  /// Print the error.
  pub fn print(&self) {
    log::warn!("Printing cmt error");
    let mut colors = ColorGenerator::new();

    for SpannedMessage { message, span } in self.messages.iter() {
      let color = colors.next();
      if let Some(MySpan {
        file,
        start_byte,
        end_byte,
        start_line,
        end_line,
        start_column,
        end_column,
      }) = span
      {
        let mut start_byte = *start_byte;
        let mut end_byte = *end_byte;

        log::warn!(
          "Printing cmt error {} at file {} byte range {:?}",
          message,
          file,
          (start_byte..end_byte)
        );

        let file_content = std::fs::read_to_string(file).unwrap();
        let source = Source::from(file_content);

        // calculate the start_byte by the line and column
        if start_byte == 0 {
          let line = source.line(start_line - 1);
          if let Some(line) = line {
            start_byte = line.offset() + start_column;
          }
        }

        // calculate the end_byte by the line and column
        if end_byte == 0 {
          let line = source.line(end_line - 1);
          if let Some(line) = line {
            end_byte = line.offset() + end_column;
          }
        }

        Report::build(ReportKind::Error, (file, start_byte..end_byte))
          .with_message(message.fg(color))
          .with_label(
            Label::new((file, start_byte..end_byte))
              .with_message(format!("here"))
              .with_color(color),
          )
          .finish()
          .eprint((file, source))
          .unwrap();
      } else {
        eprintln!(
          "{}{} {}",
          ReportKind::Error.to_string().fg(Color::Red),
          ":".fg(Color::Red),
          message.fg(color)
        );
      }
    }
  }
}

impl Display for CmtError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for msg in self.messages.iter() {
      writeln!(f, "{}", msg)?;
    }
    Ok(())
  }
}

impl std::error::Error for CmtError {}
