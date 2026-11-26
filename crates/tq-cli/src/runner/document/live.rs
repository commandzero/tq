use std::{
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel},
    },
    thread,
    time::Duration,
};

use tq_core::{EffectSink, Value, Vm, VmError, VmObservations};

use super::{DocumentOutput, RunError, cancellation, inputs::ProxyHandoff};

enum Message {
    Value { value: Value, ready: SyncSender<()> },
    Complete(Completion),
}

#[derive(Default)]
struct Completion {
    runtime_error: Option<VmError>,
    observations: VmObservations,
    trace: Vec<String>,
}

const POLL_INTERVAL: Duration = Duration::from_millis(5);

pub(super) fn run<W: Write, E: Write>(
    vm: Vm,
    output: &mut DocumentOutput<'_, W>,
    stderr: &mut E,
    proxies: &mut Option<Receiver<ProxyHandoff>>,
) -> Result<(Option<VmError>, VmObservations), RunError> {
    let stop = cancellation().unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let vm = vm.with_cancellation(Arc::clone(&stop));
    let sink = vm.effect_sink();
    let vm = vm.with_effect_acknowledgements();
    let (sender, receiver) = sync_channel(0);
    let completion = thread::scope(|scope| {
        scope.spawn(move || drive_worker(vm, &sender));
        let mut consumer = WorkerOutput {
            output,
            stderr,
            sink: &sink,
            proxies,
        };
        let result = consumer.drain(&receiver);
        if result.is_err() {
            stop.store(true, Ordering::Relaxed);
            sink.cancel();
            drop(receiver);
            let _ = proxies.take();
        }
        result
    })?;
    for entry in completion.trace {
        writeln!(stderr, "trace: {entry}")?;
    }
    Ok((completion.runtime_error, completion.observations))
}

fn drive_worker(mut vm: Vm, output: &SyncSender<Message>) {
    let runtime_error = loop {
        match vm.next_result() {
            Ok(Some(value)) => {
                if !send_value(output, value) {
                    return;
                }
            }
            Ok(None) => break None,
            Err(error) => break Some(error),
        }
    };
    let _ = output.send(Message::Complete(Completion {
        runtime_error,
        observations: vm.observations(),
        trace: vm.trace().to_vec(),
    }));
}

fn send_value(output: &SyncSender<Message>, value: Value) -> bool {
    let (ready, acknowledged) = sync_channel(0);
    output.send(Message::Value { value, ready }).is_ok() && acknowledged.recv().is_ok()
}

struct WorkerOutput<'a, 'out, W, E> {
    output: &'a mut DocumentOutput<'out, W>,
    stderr: &'a mut E,
    sink: &'a EffectSink,
    proxies: &'a Option<Receiver<ProxyHandoff>>,
}

impl<W: Write, E: Write> WorkerOutput<'_, '_, W, E> {
    fn drain(&mut self, worker: &Receiver<Message>) -> Result<Completion, RunError> {
        let completion = loop {
            self.sink.write_to(self.stderr)?;
            match worker.recv_timeout(POLL_INTERVAL) {
                Ok(Message::Value { value, ready }) => {
                    self.output.emit(value)?;
                    ready.send(()).map_err(|_| RunError::Interrupted)?;
                }
                Ok(Message::Complete(completion)) => break completion,
                Err(RecvTimeoutError::Timeout) => self.receive_proxy()?,
                Err(RecvTimeoutError::Disconnected) => break Completion::default(),
            }
        };
        self.sink.write_to(self.stderr)?;
        Ok(completion)
    }

    fn receive_proxy(&mut self) -> Result<(), RunError> {
        let Some(receiver) = self.proxies else {
            return Ok(());
        };
        let Ok(ProxyHandoff {
            bytes,
            acknowledged,
        }) = receiver.recv_timeout(POLL_INTERVAL)
        else {
            return Ok(());
        };
        self.output.results.proxy(&bytes)?;
        if let Some(acknowledged) = acknowledged {
            let _ = acknowledged.send(());
        }
        Ok(())
    }
}
