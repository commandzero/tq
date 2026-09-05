//! Header-shaped document decoding over logical row framing.

use std::{collections::BTreeSet, io::BufRead, sync::Arc};

use tq_core::{Object, Value};

use crate::{
    DecodeOptions, Document, FormatError, InputFormat,
    delimited_framing::{DelimitedError, DelimitedFramer, Field},
};

pub(crate) struct DelimitedInput<R> {
    rows: DelimitedFramer<R>,
    header: Option<Vec<Arc<str>>>,
    format: InputFormat,
    identity: String,
    index: u64,
}

impl<R: BufRead> DelimitedInput<R> {
    pub(crate) fn new(reader: R, identity: String, options: DecodeOptions) -> Self {
        Self {
            rows: DelimitedFramer::new(
                reader,
                if options.format == InputFormat::Csv {
                    b','
                } else {
                    b'\t'
                },
                options.maximum_line_bytes,
                options.maximum_token_bytes,
                options.maximum_fields,
            ),
            header: None,
            format: options.format,
            identity,
            index: 0,
        }
    }

    pub(crate) fn next_document(&mut self) -> Result<Option<Document>, FormatError> {
        if self.header.is_none() {
            let Some(fields) = self.next_row()? else {
                return Ok(None);
            };
            let mut seen = BTreeSet::new();
            let mut header = Vec::with_capacity(fields.len());
            for field in fields {
                let key: Arc<str> = field.text.into();
                if !seen.insert(Arc::clone(&key)) {
                    return Err(self.profile_error("duplicate header key"));
                }
                header.push(key);
            }
            self.header = Some(header);
        }
        let Some(fields) = self.next_row()? else {
            return Ok(None);
        };
        let header = self.header.as_ref().expect("header was established");
        if fields.len() > header.len() {
            return Err(self.profile_error("row has more fields than its header"));
        }
        let mut fields = fields.into_iter();
        let mut values = Object::new();
        for key in header {
            let value = fields.next().map_or(Value::Null, |field| {
                if field.quoted {
                    Value::string(field.text)
                } else {
                    crate::delimited_profile::scalar(&field.text)
                }
            });
            values.insert(Arc::clone(key), value);
        }
        let document = Document {
            value: Value::object(values),
            identity: self.identity.clone(),
            format: self.format,
            index: self.index,
        };
        self.index = self.index.saturating_add(1);
        Ok(Some(document))
    }

    fn next_row(&mut self) -> Result<Option<Vec<Field>>, FormatError> {
        self.rows.next_row().map_err(|error| match error {
            DelimitedError::Io(error) => FormatError::Io(error),
            DelimitedError::Resource(resource) => FormatError::Resource(resource),
            DelimitedError::Syntax(message) => self.profile_error(message),
        })
    }

    fn profile_error(&self, message: &str) -> FormatError {
        FormatError::Parse {
            format: self.format,
            message: format!("{}: row {}: {message}", self.identity, self.index),
        }
    }
}
