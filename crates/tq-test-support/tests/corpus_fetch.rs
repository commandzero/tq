//! HTTP acquisition, cache validation, and atomicity contract tests.

use std::{
    io::{self, Cursor, Read},
    sync::Mutex,
};

use tq_test_support::corpus::{
    FetchError, FetchOutcome, FetchRequest, HttpRequest, HttpResponse, Transport, fetch,
};

struct FakeTransport {
    response: Mutex<Option<Result<HttpResponse, FetchError>>>,
    request: Mutex<Option<HttpRequest>>,
}

impl FakeTransport {
    fn responding(response: HttpResponse) -> Self {
        Self {
            response: Mutex::new(Some(Ok(response))),
            request: Mutex::new(None),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            response: Mutex::new(Some(Err(FetchError::Transport(message.to_owned())))),
            request: Mutex::new(None),
        }
    }
}

impl Transport for FakeTransport {
    fn get(&self, request: &HttpRequest) -> Result<HttpResponse, FetchError> {
        *self.request.lock().expect("request lock") = Some(request.clone());
        self.response
            .lock()
            .expect("response lock")
            .take()
            .expect("one fake response")
    }
}

fn response(
    status: u16,
    final_url: &str,
    content_type: Option<&str>,
    body: impl Read + Send + 'static,
) -> HttpResponse {
    HttpResponse {
        status,
        final_url: final_url.to_owned(),
        content_type: content_type.map(str::to_owned),
        etag: Some("\"etag-2\"".to_owned()),
        last_modified: Some("Thu, 30 Jul 2026 12:00:00 GMT".to_owned()),
        body: Box::new(body),
    }
}

fn request() -> FetchRequest {
    FetchRequest {
        url: "https://example.test/source.geojson".to_owned(),
        expected_content_types: vec![
            "application/geo+json".to_owned(),
            "application/json".to_owned(),
        ],
        redirect_limit: 5,
        if_none_match: None,
        if_modified_since: None,
        expected_sha256: None,
    }
}

#[test]
fn successful_fetch_records_redirect_and_atomically_replaces_destination() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let destination = temp.path().join("source.geojson");
    std::fs::write(&destination, b"previous bytes").expect("old destination");
    let transport = FakeTransport::responding(response(
        200,
        "https://cdn.example.test/source.geojson",
        Some("application/geo+json; charset=utf-8"),
        Cursor::new(b"{\"type\":\"FeatureCollection\"}"),
    ));

    let outcome = fetch(&transport, &request(), &destination).expect("successful fetch");
    let FetchOutcome::Downloaded(metadata) = outcome else {
        panic!("expected a download");
    };

    assert_eq!(
        metadata.final_url,
        "https://cdn.example.test/source.geojson"
    );
    assert!(
        metadata.retrieved_at.parse::<jiff::Timestamp>().is_ok(),
        "retrieval timestamp must be RFC 3339 UTC"
    );
    assert_eq!(metadata.bytes, 28);
    assert_eq!(
        metadata.sha256,
        "3ca92cc8e99803203d90f8c34844412c177f52cf24154fb0088671bd0e514309"
    );
    assert_eq!(
        std::fs::read(&destination).expect("downloaded destination"),
        b"{\"type\":\"FeatureCollection\"}"
    );

    let sent = transport
        .request
        .lock()
        .expect("request lock")
        .clone()
        .expect("captured request");
    assert_eq!(sent.redirect_limit, 5);
}

#[test]
fn conditional_validators_are_sent_and_not_modified_preserves_bytes() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let destination = temp.path().join("source.geojson");
    std::fs::write(&destination, b"cached").expect("cached destination");
    let transport = FakeTransport::responding(response(
        304,
        "https://example.test/source.geojson",
        None,
        Cursor::new(Vec::<u8>::new()),
    ));
    let mut fetch_request = request();
    fetch_request.if_none_match = Some("\"etag-1\"".to_owned());
    fetch_request.if_modified_since = Some("Wed, 29 Jul 2026 12:00:00 GMT".to_owned());

    let outcome = fetch(&transport, &fetch_request, &destination).expect("not modified");
    assert!(matches!(outcome, FetchOutcome::NotModified { .. }));
    assert_eq!(
        std::fs::read(&destination).expect("cached bytes"),
        b"cached"
    );

    let sent = transport
        .request
        .lock()
        .expect("request lock")
        .clone()
        .expect("captured request");
    assert_eq!(sent.if_none_match.as_deref(), Some("\"etag-1\""));
    assert_eq!(
        sent.if_modified_since.as_deref(),
        Some("Wed, 29 Jul 2026 12:00:00 GMT")
    );
}

struct Interrupted {
    bytes: Cursor<&'static [u8]>,
    interrupted: bool,
}

impl Read for Interrupted {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.interrupted {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "simulated interruption",
            ));
        }
        self.interrupted = true;
        let read_limit = buffer.len().min(4);
        self.bytes.read(&mut buffer[..read_limit])
    }
}

#[test]
fn interrupted_download_preserves_previous_destination_and_removes_temporary_file() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let destination = temp.path().join("source.geojson");
    std::fs::write(&destination, b"previous bytes").expect("old destination");
    let transport = FakeTransport::responding(response(
        200,
        "https://example.test/source.geojson",
        Some("application/json"),
        Interrupted {
            bytes: Cursor::new(b"partial body"),
            interrupted: false,
        },
    ));

    assert!(matches!(
        fetch(&transport, &request(), &destination),
        Err(FetchError::Io(_))
    ));
    assert_eq!(
        std::fs::read(&destination).expect("old destination remains"),
        b"previous bytes"
    );
    assert_eq!(
        std::fs::read_dir(temp.path())
            .expect("temporary directory")
            .count(),
        1
    );
}

#[test]
fn wrong_content_type_is_rejected_before_destination_changes() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let destination = temp.path().join("source.geojson");
    std::fs::write(&destination, b"previous bytes").expect("old destination");
    let transport = FakeTransport::responding(response(
        200,
        "https://example.test/source.geojson",
        Some("text/html"),
        Cursor::new(b"<html>not data</html>"),
    ));

    assert!(matches!(
        fetch(&transport, &request(), &destination),
        Err(FetchError::UnexpectedContentType { .. })
    ));
    assert_eq!(
        std::fs::read(&destination).expect("old destination remains"),
        b"previous bytes"
    );
}

#[test]
fn digest_mismatch_is_rejected_without_admitting_download() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let destination = temp.path().join("source.geojson");
    let transport = FakeTransport::responding(response(
        200,
        "https://example.test/source.geojson",
        Some("application/json"),
        Cursor::new(b"payload"),
    ));
    let mut fetch_request = request();
    fetch_request.expected_sha256 =
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned());

    assert!(matches!(
        fetch(&transport, &fetch_request, &destination),
        Err(FetchError::DigestMismatch { .. })
    ));
    assert!(!destination.exists());
}

#[test]
fn redirect_transport_failures_remain_classified_transport_errors() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let destination = temp.path().join("source.geojson");
    let transport = FakeTransport::failing("too many redirects");

    assert!(matches!(
        fetch(&transport, &request(), &destination),
        Err(FetchError::Transport(message)) if message == "too many redirects"
    ));
    assert!(!destination.exists());
}
