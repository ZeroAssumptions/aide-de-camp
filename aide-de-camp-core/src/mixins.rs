use std::error::Error;

pub trait LogError {
    fn log_error(&self);
}

impl<T, E> LogError for Result<T, E>
where
    E: Error,
{
    fn log_error(&self) {
        if let Some(e) = self.as_ref().err() {
            tracing::error!("{}", &e);
        }
    }
}
