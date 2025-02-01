use std::str::FromStr;
pub(crate) mod messages_queue;
pub(crate) mod task_pool;
mod custom_stream;
pub(crate) mod fused_reader;
pub(crate) mod equal_reader;
pub mod refined_tcp_stream;
pub mod sequential;

/// Parses a the value of a header.
/// Suitable for `Accept-*`, `TE`, etc.
///
/// For example with `text/plain, image/png; q=1.5` this function would
/// return `[ ("text/plain", 1.0), ("image/png", 1.5) ]`
pub fn parse_header_value(input: &str) -> Vec<(&str, f32)> {
    input
        .split(',')
        .filter_map(|elem| {
            let mut params = elem.split(';');
            let t = params.next()?;
            let mut value = 1.0_f32;

            for p in params {
                if p.trim_start().starts_with("q=") {
                    if let Ok(val) = f32::from_str(p.trim_start()[2..].trim()) {
                        value = val;
                        break;
                    }
                }
            }
            Some((t.trim(), value))
        })
        .collect()
}