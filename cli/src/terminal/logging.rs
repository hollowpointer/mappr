use colored::*;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::fmt::format::{self, Writer};
use tracing_subscriber::registry::LookupSpan;
use tracing::field::{Visit, Field};

pub struct MapprFormatter;

impl<S, N> FormatEvent<S, N> for MapprFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> format::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();

        if meta.target() == "mappr::print" {
            let mut visitor = RawVisitor::new(writer.by_ref());
            event.record(&mut visitor);
            return write!(writer, "\r\n"); 
        }

        let (symbol, color_func): (&str, fn(ColoredString) -> ColoredString) = match *meta.level() {
            Level::TRACE => ("[ ]", |s| s.dimmed()),
            Level::DEBUG => ("[?]", |s| s.blue()),
            Level::INFO => ("[+]", |s| s.green().bold()),
            Level::WARN => ("[*]", |s| s.yellow().bold()),
            Level::ERROR => ("[-]", |s| s.red().bold()),
        };

        write!(writer, "{} ", color_func(symbol.into()))?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        write!(writer, "\r\n")
    }
}

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
            let _ = write!(self.writer, "{}", value);
        }
    }
}