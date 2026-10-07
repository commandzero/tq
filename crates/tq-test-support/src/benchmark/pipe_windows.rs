//! Real overlapped readers; child stdio never uses Tokio's blocking anonymous pipes.

use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::OpenOptionsExt as _,
};
use tokio::net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions};

const SECURITY_IDENTIFICATION: u32 = 0x0001_0000;

pub(super) struct OutputReader {
    pub server: NamedPipeServer,
    _namespace: tempfile::TempDir,
}

impl Drop for OutputReader {
    fn drop(&mut self) {
        // Mio can retain pending overlapped I/O internally. Explicit disconnect
        // severs even an outside-job client before closing the server handle.
        let _ = self.server.disconnect();
    }
}

pub(super) struct OutputPipe {
    reader: OutputReader,
    child: File,
}

impl OutputPipe {
    pub(super) async fn prepare() -> io::Result<Self> {
        let namespace = tempfile::Builder::new()
            .prefix("tq-benchmark-pipe-")
            .rand_bytes(24)
            .tempdir()?;
        let reserved = namespace
            .path()
            .file_name()
            .ok_or_else(|| io::Error::other("missing pipe namespace"))?
            .to_string_lossy();
        let name = format!(r"\\.\pipe\{reserved}");
        let server = ServerOptions::new()
            .pipe_mode(PipeMode::Byte)
            .max_instances(1)
            .in_buffer_size(16 * 1024)
            .out_buffer_size(16 * 1024)
            .access_inbound(true)
            .access_outbound(false)
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&name)?;
        // The server exists before opening its local synchronous writer. Only
        // this File is passed through Stdio, where std duplicates it for the child.
        // The async reader stays overlapped; no Blocking<Arc<File>> is introduced.
        // Match the compatibility collector's local-only, fail-closed setup.
        // Identification-level QOS prevents a server from impersonating a client.

        let child = OpenOptions::new()
            .write(true)
            .security_qos_flags(SECURITY_IDENTIFICATION)
            .open(&name)?;
        server.connect().await?;
        Ok(Self {
            reader: OutputReader {
                server,
                _namespace: namespace,
            },
            child,
        })
    }

    pub(super) fn into_parts(self) -> (OutputReader, File) {
        (self.reader, self.child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write as _,
        process::Stdio,
        time::{Duration, Instant},
    };
    use tokio::io::AsyncReadExt as _;

    #[test]
    fn held_open_child_end_does_not_delay_read_abort_or_runtime_drop() {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "benchmark::pipe_windows::tests::held_open_reader_helper",
                "--ignored",
                "--nocapture",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "held-open pipe helper failed: {status}");
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("held-open client stalled reader abort or runtime shutdown");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    #[ignore = "helper for the bounded held-open pipe watchdog test"]
    fn held_open_reader_helper() {
        let started = Instant::now();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut child = runtime.block_on(async {
            let (mut server, mut child) = OutputPipe::prepare().await.unwrap().into_parts();
            child.write_all(b"binary\0\xff\r\n").unwrap();
            let mut expected = [0_u8; 10];
            server.server.read_exact(&mut expected).await.unwrap();
            assert_eq!(&expected, b"binary\0\xff\r\n");
            let reader = tokio::spawn(async move {
                let mut byte = [0_u8];
                server.server.read(&mut byte).await
            });
            tokio::time::sleep(Duration::from_millis(20)).await;
            assert!(!reader.is_finished(), "held writer must prevent EOF");
            reader.abort();
            assert!(reader.await.unwrap_err().is_cancelled());
            child
        });
        // Keep the writer alive across Runtime::drop: a blocking anonymous pipe
        // adapter would wait for EOF here, even after its async task was aborted.
        drop(runtime);
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(
            child.write_all(b"disconnected").is_err(),
            "cancelled reader kept its client connected"
        );
        drop(child);
    }
}
