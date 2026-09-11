//! Native RSS scope checks for a process that waits its allocating child.

#![allow(missing_docs)]

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod native {
    use std::{path::PathBuf, time::Duration};

    use tq_test_support::benchmark::{
        BenchmarkInvocation, MeasuredStatus, RssProvenance, measure_process,
    };

    const ALLOCATION_BYTES: &str = "67108864";
    const RSS_DELTA_MARGIN: u64 = 16 * 1024 * 1024;

    fn probe(args: &[&str]) -> BenchmarkInvocation {
        BenchmarkInvocation {
            cancellation: None,
            executable: PathBuf::from(env!("CARGO_BIN_EXE_tq-bench-probe")),
            args: args.iter().map(|value| (*value).to_owned()).collect(),
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(5),
            output_limit: 16 * 1024 * 1024,
            rss_limit: None,
            retain_output: false,
        }
    }

    #[test]
    fn waited_allocation_is_in_authoritative_wait4_rss_scope() {
        let no_op = measure_process(&probe(&["noop"])).expect("no-op measurement");
        let direct = measure_process(&probe(&["allocate-burst", ALLOCATION_BYTES]))
            .expect("direct allocation measurement");
        let waited = measure_process(&probe(&["waited-allocation-child", ALLOCATION_BYTES]))
            .expect("waited allocation measurement");

        assert_eq!(no_op.status, MeasuredStatus::Exited);
        assert_eq!(direct.status, MeasuredStatus::Exited);
        assert_eq!(waited.status, MeasuredStatus::Exited);
        assert!(matches!(
            waited.rss_provenance,
            RssProvenance::DarwinWait4 | RssProvenance::LinuxWait4
        ));
        assert_eq!(
            waited.measurement_protocol.rss_scope,
            "wait4-child-including-waited-descendants-and-threads"
        );

        let no_op_rss = no_op.peak_rss_bytes.expect("no-op RSS");
        let direct_rss = direct.peak_rss_bytes.expect("direct allocation RSS");
        let waited_rss = waited.peak_rss_bytes.expect("waited allocation RSS");
        assert!(
            direct_rss > no_op_rss.saturating_add(RSS_DELTA_MARGIN),
            "direct allocation did not raise RSS: no-op={no_op_rss}, direct={direct_rss}"
        );
        assert!(
            waited_rss > no_op_rss.saturating_add(RSS_DELTA_MARGIN),
            "waited descendant allocation was omitted from RSS: no-op={no_op_rss}, waited={waited_rss}"
        );
        assert!(
            waited_rss.abs_diff(direct_rss) < RSS_DELTA_MARGIN,
            "waited and direct allocation RSS diverged: direct={direct_rss}, waited={waited_rss}"
        );
    }
}
