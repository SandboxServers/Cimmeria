//! A logger provider that redacts every string a log record carries before
//! the SDK sees it.
//!
//! Why a wrapper and not a `LogProcessor`: the SDK hands processors a
//! `&mut SdkLogRecord` but has no way to change or remove an attribute
//! already on it, and no public constructor for a fresh record. The
//! `opentelemetry-appender-tracing` bridge, though, is generic over the
//! `LoggerProvider`, so it writes the body and each field through
//! [`RedactingRecord`]'s setters, and nothing unredacted is ever stored.

use std::borrow::Cow;
use std::sync::Arc;
use std::time::SystemTime;

use opentelemetry::logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity};
use opentelemetry::{InstrumentationScope, Key, SpanId, StringValue, TraceFlags, TraceId};
use opentelemetry_sdk::logs::{SdkLogRecord, SdkLogger, SdkLoggerProvider};

use super::redact::Redactor;

/// [`SdkLoggerProvider`] whose loggers redact through a shared [`Redactor`].
#[derive(Debug, Clone)]
pub(crate) struct RedactingProvider {
    inner: SdkLoggerProvider,
    redactor: Arc<Redactor>,
}

impl RedactingProvider {
    pub(crate) fn new(inner: SdkLoggerProvider, redactor: Redactor) -> Self {
        Self {
            inner,
            redactor: Arc::new(redactor),
        }
    }
}

impl LoggerProvider for RedactingProvider {
    type Logger = RedactingLogger;

    fn logger_with_scope(&self, scope: InstrumentationScope) -> Self::Logger {
        RedactingLogger {
            inner: self.inner.logger_with_scope(scope),
            redactor: self.redactor.clone(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RedactingLogger {
    inner: SdkLogger,
    redactor: Arc<Redactor>,
}

impl Logger for RedactingLogger {
    type LogRecord = RedactingRecord;

    fn create_log_record(&self) -> Self::LogRecord {
        RedactingRecord {
            inner: self.inner.create_log_record(),
            redactor: self.redactor.clone(),
        }
    }

    fn emit(&self, record: Self::LogRecord) {
        self.inner.emit(record.inner);
    }

    fn event_enabled(&self, level: Severity, target: &str, name: Option<&str>) -> bool {
        self.inner.event_enabled(level, target, name)
    }
}

/// An [`SdkLogRecord`] that only ever stores redacted strings.
#[derive(Debug)]
pub(crate) struct RedactingRecord {
    inner: SdkLogRecord,
    redactor: Arc<Redactor>,
}

impl RedactingRecord {
    fn clean(&self, value: AnyValue) -> AnyValue {
        match value {
            AnyValue::String(s) => match self.redactor.redact(s.as_str()) {
                Cow::Borrowed(_) => AnyValue::String(s),
                Cow::Owned(o) => AnyValue::String(StringValue::from(o)),
            },
            AnyValue::ListAny(list) => {
                AnyValue::ListAny(Box::new(list.into_iter().map(|v| self.clean(v)).collect()))
            }
            AnyValue::Map(map) => AnyValue::Map(Box::new(
                map.into_iter().map(|(k, v)| (k, self.clean(v))).collect(),
            )),
            other => other,
        }
    }
}

impl LogRecord for RedactingRecord {
    /// tracing names an event `event <file!()>:<line>`, and `file!()` of a
    /// dependency is its absolute path in the cargo registry, under
    /// `C:\Users\<name>\.cargo`. The name is `&'static`, so a redacted copy
    /// cannot be stored: a name that holds a path is left off instead.
    fn set_event_name(&mut self, name: &'static str) {
        if matches!(self.redactor.redact(name), Cow::Borrowed(_)) {
            self.inner.set_event_name(name);
        }
    }

    fn set_target<T>(&mut self, target: T)
    where
        T: Into<Cow<'static, str>>,
    {
        self.inner.set_target(target);
    }

    fn set_timestamp(&mut self, timestamp: SystemTime) {
        self.inner.set_timestamp(timestamp);
    }

    fn set_observed_timestamp(&mut self, timestamp: SystemTime) {
        self.inner.set_observed_timestamp(timestamp);
    }

    fn set_severity_text(&mut self, text: &'static str) {
        self.inner.set_severity_text(text);
    }

    fn set_severity_number(&mut self, number: Severity) {
        self.inner.set_severity_number(number);
    }

    fn set_body(&mut self, body: AnyValue) {
        let body = self.clean(body);
        self.inner.set_body(body);
    }

    fn add_attributes<I, K, V>(&mut self, attributes: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<Key>,
        V: Into<AnyValue>,
    {
        for (k, v) in attributes {
            self.add_attribute(k, v);
        }
    }

    fn add_attribute<K, V>(&mut self, key: K, value: V)
    where
        K: Into<Key>,
        V: Into<AnyValue>,
    {
        let value = self.clean(value.into());
        self.inner.add_attribute(key, value);
    }

    fn set_trace_context(
        &mut self,
        trace_id: TraceId,
        span_id: SpanId,
        trace_flags: Option<TraceFlags>,
    ) {
        self.inner.set_trace_context(trace_id, span_id, trace_flags);
    }
}
