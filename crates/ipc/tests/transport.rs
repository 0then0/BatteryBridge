use batterybridge_ipc::*;
use batterybridge_protocol::*;
use std::{
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn address() -> String {
    let suffix = format!(
        "bb-ipc-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(suffix)
            .to_string_lossy()
            .into_owned()
    }
    #[cfg(windows)]
    {
        suffix
    }
}
#[test]
fn typed_round_trip_and_socket_permissions() {
    let endpoint = address();
    let listener = listen(&endpoint).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&endpoint).unwrap().permissions().mode() & 0o777,
            0o660
        );
    }
    let server = std::thread::spawn(move || {
        let mut connection = listener.incoming().next().unwrap().unwrap();
        let request: Request = receive(&mut connection).unwrap();
        request.validate().unwrap();
        assert!(matches!(request.command, Command::Status));
        send(&mut connection, &Response::Config(Config::default())).unwrap();
    });
    assert!(matches!(
        call_at(&endpoint, Command::Status).unwrap(),
        Response::Config(_)
    ));
    server.join().unwrap();
}
#[test]
fn oversized_frame_is_rejected_before_allocating_payload() {
    let endpoint = address();
    let listener = listen(&endpoint).unwrap();
    let server = std::thread::spawn(move || {
        let mut connection = listener.incoming().next().unwrap().unwrap();
        assert!(matches!(
            receive::<Request>(&mut connection),
            Err(Error::ProtocolError(_))
        ));
    });
    use interprocess::local_socket::prelude::*;
    #[cfg(unix)]
    let name = endpoint
        .as_str()
        .to_fs_name::<interprocess::local_socket::GenericFilePath>()
        .unwrap();
    #[cfg(windows)]
    let name = endpoint
        .as_str()
        .to_ns_name::<interprocess::local_socket::GenericNamespaced>()
        .unwrap();
    let mut stream = Stream::connect(name).unwrap();
    stream
        .write_all(&((MAX_FRAME + 1) as u32).to_be_bytes())
        .unwrap();
    server.join().unwrap();
}

#[cfg(unix)]
#[test]
fn crash_recovery_preserves_exclusive_instance_ownership() {
    use std::os::unix::net::UnixListener;
    let endpoint = address();
    let stale = UnixListener::bind(&endpoint).unwrap();
    drop(stale);
    let listener = listen(&endpoint).unwrap();
    assert!(
        listen(&endpoint).is_err(),
        "a live daemon must never be displaced"
    );
    drop(listener);
    let recovered = listen(&endpoint).unwrap();
    drop(recovered);
    let _ = std::fs::remove_file(std::path::Path::new(&endpoint).with_extension("lock"));
}
#[cfg(unix)]
#[test]
fn non_socket_endpoint_is_never_removed() {
    let endpoint = address();
    std::fs::write(&endpoint, "preserve me").unwrap();
    assert!(listen(&endpoint).is_err());
    assert_eq!(std::fs::read_to_string(&endpoint).unwrap(), "preserve me");
    std::fs::remove_file(&endpoint).unwrap();
    let _ = std::fs::remove_file(std::path::Path::new(&endpoint).with_extension("lock"));
}
