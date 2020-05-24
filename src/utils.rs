use std::error::Error;

use backtrace::Backtrace;

pub struct Value<T>(pub T);

pub trait BacktraceExt<T> {
    // Add backtrace to original error
    fn debug(self) -> Result<T, String>;
}

impl<T, E: Error> BacktraceExt<T> for Result<Value<T>, E> {
    fn debug(self) -> Result<T, String> {
        self.map(|v| v.0).map_err(|err| {
            let bt = Backtrace::new();
            let symbol = &bt.frames()[3].symbols()[0];

            let symbol_name = symbol
                .name()
                .map(|name| name.to_string())
                .unwrap_or_else(|| String::from("<unknown>"));

            let symbol_path =
                if let (Some(filename), Some(line_number)) = (symbol.filename(), symbol.lineno()) {
                    format!("{}:{}", filename.to_string_lossy(), line_number)
                } else {
                    String::from("<unknown>")
                };

            format!(
                "{}\n\nOriginal error line:\n{}\n\tat {}",
                err, symbol_name, symbol_path
            )
        })
    }
}
