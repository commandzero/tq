use std::{
    io::Read,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread,
};

use tq_core::{InputCursor, InputValue, Value, VmError};
use tq_formats::{Document, InputFormat};

use super::super::{
    RunError,
    input::{
        LoadedInputs, RemainingInputMessage, StructuredInput, for_each_structured_input,
        load_inputs, produce_proxy_remaining_inputs, produce_remaining_inputs,
        pull_remaining_input, selected_input_format,
    },
};
use crate::RunOptions;

pub(super) struct ProxyHandoff {
    pub(super) bytes: Vec<u8>,
    pub(super) acknowledged: Option<SyncSender<()>>,
}

pub(super) struct DocumentInputs<'a> {
    pub(super) options: &'a RunOptions,
    pub(super) whole_input: bool,
    pub(super) proxy_sender: SyncSender<ProxyHandoff>,
}

type Evaluate<'a> = dyn FnMut(StructuredInput, Option<InputCursor>) -> Result<bool, RunError> + 'a;

impl DocumentInputs<'_> {
    pub(super) fn drive<R: Read + Send>(
        self,
        stdin: &mut R,
        diagnostics: &mut Vec<String>,
        evaluate: &mut Evaluate<'_>,
    ) -> Result<(), RunError> {
        let options = self.options;
        if options.null_input && !self.whole_input {
            evaluate(StructuredInput::Value(Value::Null), None)?;
        } else if options.slurp && !options.raw_input {
            if options.proxy_on_error && !options.null_input {
                proxy_slurp(options, stdin, diagnostics, evaluate)?;
            } else {
                structured_slurp(options, stdin, diagnostics, evaluate)?;
            }
        } else if self.whole_input && !options.slurp && !options.raw_input {
            if options.proxy_on_error && !options.stream {
                proxy_cursor(options, stdin, self.proxy_sender, evaluate)?;
            } else {
                streaming_cursor(options, stdin, evaluate)?;
            }
        } else if options.slurp || options.raw_input {
            loaded_cursor(options, stdin, diagnostics, evaluate)?;
        } else {
            for_each_structured_input(options, stdin, diagnostics, &mut |input| {
                evaluate(input, None)
            })?;
        }
        Ok(())
    }
}

fn proxy_slurp(
    options: &RunOptions,
    stdin: &mut impl Read,
    diagnostics: &mut Vec<String>,
    evaluate: &mut Evaluate<'_>,
) -> Result<(), RunError> {
    match load_inputs(options, stdin, false, diagnostics)? {
        LoadedInputs::Proxy(bytes) => {
            evaluate(StructuredInput::Proxy(bytes), None)?;
        }
        LoadedInputs::Documents(inputs) => {
            let cursor = InputCursor::from_input_values(vec![slurped_document(inputs)]);
            let input = cursor.next_value()?.expect("slurp supplies one input");
            evaluate(StructuredInput::Value(input), Some(cursor))?;
        }
    }
    Ok(())
}

fn slurped_document(inputs: Vec<Document>) -> InputValue {
    let (identity, line_number) = inputs.last().map_or_else(
        || ("<stdin>".to_owned(), 0),
        |document| (document.identity.clone(), document.line_number),
    );
    InputValue {
        value: Value::array(
            inputs
                .into_iter()
                .map(|document| document.value)
                .collect::<Vec<_>>(),
        ),
        identity: Arc::from(identity),
        line_number,
    }
}

fn structured_slurp(
    options: &RunOptions,
    stdin: &mut impl Read,
    diagnostics: &mut Vec<String>,
    evaluate: &mut Evaluate<'_>,
) -> Result<(), RunError> {
    let mut values = Vec::new();
    let mut had_proxy = false;
    for_each_structured_input(
        &remaining_options(options),
        stdin,
        diagnostics,
        &mut |input| match input {
            StructuredInput::Value(value) => {
                values.push(value);
                Ok(true)
            }
            StructuredInput::Document(document) => {
                values.push(document.value);
                Ok(true)
            }
            input @ StructuredInput::Proxy(_) => {
                had_proxy = true;
                evaluate(input, None)
            }
            input => evaluate(input, None),
        },
    )?;
    if !had_proxy || !values.is_empty() {
        let cursor = InputCursor::new(vec![Value::array(values)]);
        let input = if options.null_input {
            Value::Null
        } else {
            cursor.next_value()?.expect("slurp supplies one input")
        };
        evaluate(StructuredInput::Value(input), Some(cursor))?;
    }
    Ok(())
}

fn remaining_options(options: &RunOptions) -> RunOptions {
    let mut remaining = options.clone();
    remaining.null_input = false;
    remaining
}

fn streaming_cursor<R: Read + Send>(
    options: &RunOptions,
    stdin: &mut R,
    evaluate: &mut Evaluate<'_>,
) -> Result<(), RunError> {
    thread::scope(|scope| {
        let (request_sender, request_receiver) = sync_channel(0);
        let (sender, receiver) = sync_channel(0);
        scope.spawn(move || produce_remaining_inputs(options, stdin, &request_receiver, &sender));
        let cursor = InputCursor::from_provider(move || {
            if request_sender.send(()).is_err() {
                return Ok(None);
            }
            let Ok(message) = receiver.recv() else {
                return Ok(None);
            };
            unpack_remaining(message)
        });
        evaluate_cursor(options, cursor, evaluate, pull_remaining_input)
    })
}

fn unpack_remaining(message: RemainingInputMessage) -> Result<Option<InputValue>, VmError> {
    match message {
        RemainingInputMessage::Value(value) => Ok(Some(value)),
        RemainingInputMessage::Failure(failure) => Err(VmError::RecoverableInput {
            context: format!(
                "in '{}' at recovery segment {}",
                failure.identity, failure.recovery_segment_index
            )
            .into(),
            message: failure.message.into(),
        }),
        RemainingInputMessage::Proxy(_) => Err(VmError::Runtime {
            message: "proxy input reached a non-proxy cursor".into(),
        }),
        RemainingInputMessage::Error(error) => Err(error),
        RemainingInputMessage::Done => Ok(None),
    }
}

fn evaluate_cursor(
    options: &RunOptions,
    cursor: InputCursor,
    evaluate: &mut Evaluate<'_>,
    mut pull: impl FnMut(&InputCursor) -> Result<Option<StructuredInput>, RunError>,
) -> Result<(), RunError> {
    if options.null_input {
        evaluate(StructuredInput::Value(Value::Null), Some(cursor))?;
        return Ok(());
    }
    while let Some(input) = pull(&cursor)? {
        if !evaluate(input, Some(cursor.clone()))? {
            break;
        }
    }
    Ok(())
}

fn proxy_cursor<R: Read + Send>(
    options: &RunOptions,
    stdin: &mut R,
    proxy_sender: SyncSender<ProxyHandoff>,
    evaluate: &mut Evaluate<'_>,
) -> Result<(), RunError> {
    let remaining_options = remaining_options(options);
    thread::scope(|scope| {
        let (request_sender, request_receiver) = sync_channel(0);
        let (sender, receiver) = sync_channel(0);
        scope.spawn(move || {
            produce_proxy_remaining_inputs(&remaining_options, stdin, &request_receiver, &sender);
        });
        let proxy = Arc::new(TopLevelProxy::default());
        let provider_proxy = Arc::clone(&proxy);
        let cursor = InputCursor::from_provider(move || {
            provider_proxy.provide(&request_sender, &receiver, &proxy_sender)
        });
        if options.null_input {
            evaluate(StructuredInput::Value(Value::Null), Some(cursor))?;
            return Ok(());
        }
        proxy.evaluate(&cursor, evaluate)
    })
}

#[derive(Default)]
struct TopLevelProxy {
    pulling: AtomicBool,
    slot: Mutex<Option<ProxyHandoff>>,
}

impl TopLevelProxy {
    fn provide(
        &self,
        request: &SyncSender<()>,
        receiver: &Receiver<RemainingInputMessage>,
        proxy: &SyncSender<ProxyHandoff>,
    ) -> Result<Option<InputValue>, VmError> {
        let top_level = self.pulling.swap(false, Ordering::Relaxed);
        while request.send(()).is_ok() {
            let Ok(message) = receiver.recv() else {
                break;
            };
            let RemainingInputMessage::Proxy(bytes) = message else {
                return unpack_remaining(message);
            };
            if top_level {
                self.store(bytes)?;
                break;
            }
            if !send_proxy(proxy, bytes)? {
                break;
            }
        }
        Ok(None)
    }

    fn store(&self, bytes: Vec<u8>) -> Result<(), VmError> {
        let mut slot = self.slot.lock().map_err(|_| proxy_slot_unavailable())?;
        if slot.is_some() {
            return Err(VmError::Runtime {
                message: "top-level proxy slot is already occupied".into(),
            });
        }
        *slot = Some(ProxyHandoff {
            bytes,
            acknowledged: None,
        });
        Ok(())
    }

    fn evaluate(&self, cursor: &InputCursor, evaluate: &mut Evaluate<'_>) -> Result<(), RunError> {
        loop {
            self.pulling.store(true, Ordering::Relaxed);
            if let Some(input) = cursor.next_value()? {
                if !evaluate(StructuredInput::Value(input), Some(cursor.clone()))? {
                    break;
                }
                continue;
            }
            let proxy = self
                .slot
                .lock()
                .map_err(|_| proxy_slot_unavailable())?
                .take();
            let Some(ProxyHandoff {
                bytes,
                acknowledged,
            }) = proxy
            else {
                break;
            };
            evaluate(StructuredInput::Proxy(bytes), None)?;
            if let Some(acknowledged) = acknowledged {
                let _ = acknowledged.send(());
            }
        }
        Ok(())
    }
}

fn proxy_slot_unavailable() -> VmError {
    VmError::Runtime {
        message: "top-level proxy slot is unavailable".into(),
    }
}

fn send_proxy(proxy: &SyncSender<ProxyHandoff>, bytes: Vec<u8>) -> Result<bool, VmError> {
    let (sender, receiver) = sync_channel(0);
    if proxy
        .send(ProxyHandoff {
            bytes,
            acknowledged: Some(sender),
        })
        .is_err()
    {
        return Ok(false);
    }
    receiver.recv().map_err(|_| VmError::Interrupted)?;
    Ok(true)
}

fn loaded_cursor(
    options: &RunOptions,
    stdin: &mut impl Read,
    diagnostics: &mut Vec<String>,
    evaluate: &mut Evaluate<'_>,
) -> Result<(), RunError> {
    let recover_sequence = options.slurp
        && selected_input_format(options, Path::new("-")) == InputFormat::JsonSequence;
    let loaded = load_inputs(
        &remaining_options(options),
        stdin,
        recover_sequence,
        diagnostics,
    )?;
    match loaded {
        LoadedInputs::Proxy(bytes) => {
            let input = if options.null_input {
                StructuredInput::Value(Value::Null)
            } else {
                StructuredInput::Proxy(bytes)
            };
            evaluate(input, None)?;
            Ok(())
        }
        LoadedInputs::Documents(inputs) => {
            let values = inputs.into_iter().map(|document| InputValue {
                value: document.value,
                identity: Arc::from(document.identity),
                line_number: document.line_number,
            });
            let cursor = InputCursor::from_input_values(values.collect());
            evaluate_cursor(options, cursor, evaluate, |cursor| {
                Ok(cursor.next_value()?.map(StructuredInput::Value))
            })
        }
    }
}
