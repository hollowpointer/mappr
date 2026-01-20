use colored::*;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::fmt::format::{self, Writer};
use tracing_subscriber::registry::LookupSpan;

pub struct MapprFormatter;

impl<S, N> FormatEvent<S, N> for MapprFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> format::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();

        if meta.target() == "mappr::print" {
            let mut visitor = RawVisitor::new(writer.by_ref());
            event.record(&mut visitor);
            return write!(writer, "\r\n");
        }

        let mut status_visitor = StatusVisitor::default();
        event.record(&mut status_visitor);

        let (symbol, color_func): (&str, fn(ColoredString) -> ColoredString) = match *meta.level() {
            Level::TRACE => ("[ ]", |s| s.dimmed()),
            Level::DEBUG => ("[?]", |s| s.blue()),
            Level::INFO => match status_visitor.status.as_deref() {
                Some("info") => ("[»]", |s| s.cyan().bold()),
                _ => ("[+]", |s| s.green().bold()),
            },
            Level::WARN => ("[*]", |s| s.yellow().bold()),
            Level::ERROR => ("[-]", |s| s.red().bold()),
        };

        write!(writer, "{} ", color_func(symbol.into()))?;

        let mut output_visitor = OutputVisitor::new(writer.by_ref());
        event.record(&mut output_visitor);

        write!(writer, "\r\n")
    }
}

// Visitors

#[derive(Default)]
struct StatusVisitor {
    status: Option<String>,
}

impl Visit for StatusVisitor {
    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "status" {
            self.status = Some(value.to_string());
        }
    }
}

struct OutputVisitor<'a> {
    writer: Writer<'a>,
}

impl<'a> OutputVisitor<'a> {
    fn new(writer: Writer<'a>) -> Self {
        Self { writer }
    }
}

impl<'a> Visit for OutputVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "status" { return; }

        if field.name() == "message" {
            let _ = write!(self.writer, "{:?}", value);
        } else {
            let _ = write!(self.writer, " {}={:?}", field.name().italic(), value);
        }
    }
}

/// Handles raw printing
struct RawVisitor<'a> {
    writer: Writer<'a>,
}

impl<'a> RawVisitor<'a> {
    fn new(writer: Writer<'a>) -> Self {
        Self { writer }
    }
}

impl<'a> Visit for RawVisitor<'a> {
    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "raw_msg" {
            let replaced = value.replace('\n', "\r\n");
            let _ = write!(self.writer, "{}", replaced);
        }
    }
}