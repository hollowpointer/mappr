use colored::*;
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
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();

        let (symbol, color_func): (&str, fn(ColoredString) -> ColoredString) = match *meta.level() {
            Level::TRACE => ("[ ]", |s| s.dimmed()),
            Level::DEBUG => ("[?]", |s| s.blue()),
            Level::INFO => ("[+]", |s| s.green().bold()),
            Level::WARN => ("[*]", |s| s.yellow().bold()),
            Level::ERROR => ("[-]", |s| s.red().bold()),
        };

        write!(writer, "{} ", color_func(symbol.into()))?;

        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}
