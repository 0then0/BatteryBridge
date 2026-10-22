#![forbid(unsafe_code)]
use batterybridge_protocol::*;
#[cfg(not(windows))]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
pub use interprocess::local_socket::Stream;
pub struct Listener {
    inner: interprocess::local_socket::Listener,
    _lock: Option<std::fs::File>,
}
impl Listener {
    pub fn incoming(&self) -> impl Iterator<Item = std::io::Result<Stream>> + '_ {
        self.inner.incoming()
    }
}
use interprocess::local_socket::{ListenerOptions, prelude::*};
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
const TIMEOUT: Duration = Duration::from_secs(3);

pub fn config_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/BatteryBridge/config.toml")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/var/lib/batterybridge/config.toml")
    }
    #[cfg(windows)]
    {
        PathBuf::from(std::env::var_os("ProgramData").unwrap_or_else(|| r"C:\ProgramData".into()))
            .join("BatteryBridge")
            .join("config.toml")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        PathBuf::from("config.toml")
    }
}
pub fn endpoint() -> String {
    #[cfg(target_os = "macos")]
    {
        "/var/run/batterybridge/control.sock".into()
    }
    #[cfg(target_os = "linux")]
    {
        "/run/batterybridge/control.sock".into()
    }
    #[cfg(windows)]
    {
        "BatteryBridge.v1".into()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        "batterybridge.sock".into()
    }
}
fn name(value: &str) -> std::io::Result<interprocess::local_socket::Name<'_>> {
    #[cfg(windows)]
    {
        value.to_ns_name::<GenericNamespaced>()
    }
    #[cfg(not(windows))]
    {
        value.to_fs_name::<GenericFilePath>()
    }
}
pub fn listen(endpoint: &str) -> Result<Listener> {
    #[cfg(unix)]
    let lock = {
        use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
        let path = std::path::Path::new(endpoint);
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path.with_extension("lock"))
            .map_err(|e| Error::io("open daemon instance lock", e))?;
        lock.try_lock().map_err(|e| {
            Error::BackendUnavailable(format!(
                "daemon already running or instance lock unavailable: {e}"
            ))
        })?;
        match std::fs::symlink_metadata(path) {
            Ok(meta) => {
                if !meta.file_type().is_socket() {
                    return Err(Error::PermissionDenied(
                        "refusing to replace a non-socket IPC path".into(),
                    ));
                }
                match Stream::connect(name(endpoint).map_err(|e| Error::io("IPC name", e))?) {
                    Ok(_) => {
                        return Err(Error::BackendUnavailable(
                            "IPC endpoint belongs to a live listener".into(),
                        ));
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                        ) => {}
                    Err(e) => return Err(Error::io("inspect existing IPC endpoint", e)),
                }
                std::fs::remove_file(path).map_err(|e| Error::io("reclaim stale IPC socket", e))?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io("inspect IPC path", e)),
        }
        Some(lock)
    };
    #[cfg(windows)]
    let lock = None;

    let options =
        ListenerOptions::new().name(name(endpoint).map_err(|e| Error::io("IPC name", e))?);
    #[cfg(windows)]
    let options = {
        use interprocess::os::windows::{
            local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
        };
        let descriptor = widestring::U16CString::from_str(
            "D:P(D;;GA;;;NU)(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)",
        )
        .map_err(|_| Error::ConfigurationError("invalid pipe ACL".into()))?;
        options.security_descriptor(
            SecurityDescriptor::deserialize(&descriptor).map_err(|e| Error::io("pipe ACL", e))?,
        )
    };
    let listener = options
        .create_sync()
        .map_err(|e| Error::io("bind local IPC (another daemon or stale socket?)", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(endpoint, std::fs::Permissions::from_mode(0o660))
            .map_err(|e| Error::io("secure socket", e))?;
    }
    Ok(Listener {
        inner: listener,
        _lock: lock,
    })
}
fn transfer<F: FnMut() -> std::io::Result<usize>>(until: Instant, mut f: F) -> Result<usize> {
    loop {
        if Instant::now() >= until {
            return Err(Error::ProtocolError("IPC frame deadline exceeded".into()));
        }
        match f() {
            Ok(v) => return Ok(v),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(e) => return Err(Error::io("local IPC I/O", e)),
        }
    }
}
fn read_exact(stream: &mut Stream, buffer: &mut [u8], until: Instant) -> Result<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        if Instant::now() >= until {
            return Err(Error::ProtocolError("IPC frame timeout".into()));
        }
        let count = transfer(until, || stream.read(&mut buffer[offset..]))?;
        if count == 0 {
            return Err(Error::ProtocolError("truncated IPC frame".into()));
        }
        offset += count;
    }
    Ok(())
}
fn write_all(stream: &mut Stream, buffer: &[u8], until: Instant) -> Result<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        if Instant::now() >= until {
            return Err(Error::ProtocolError("IPC write timeout".into()));
        }
        let count = transfer(until, || stream.write(&buffer[offset..]))?;
        if count == 0 {
            return Err(Error::ProtocolError("closed IPC stream".into()));
        }
        offset += count;
    }
    Ok(())
}
pub fn receive<T: serde::de::DeserializeOwned>(stream: &mut Stream) -> Result<T> {
    stream
        .set_nonblocking(true)
        .map_err(|e| Error::io("configure IPC", e))?;
    let until = Instant::now() + TIMEOUT;
    let mut header = [0; 4];
    read_exact(stream, &mut header, until)?;
    let size = u32::from_be_bytes(header) as usize;
    if size == 0 || size > MAX_FRAME {
        return Err(Error::ProtocolError("invalid frame length".into()));
    }
    let mut bytes = vec![0; size];
    read_exact(stream, &mut bytes, until)?;
    serde_json::from_slice(&bytes).map_err(|_| Error::ProtocolError("invalid JSON frame".into()))
}
pub fn send<T: serde::Serialize>(stream: &mut Stream, value: &T) -> Result<()> {
    stream
        .set_nonblocking(true)
        .map_err(|e| Error::io("configure IPC", e))?;
    let bytes =
        serde_json::to_vec(value).map_err(|_| Error::ProtocolError("serialize frame".into()))?;
    if bytes.len() > MAX_FRAME {
        return Err(Error::ProtocolError("frame too large".into()));
    }
    let until = Instant::now() + TIMEOUT;
    write_all(stream, &(bytes.len() as u32).to_be_bytes(), until)?;
    write_all(stream, &bytes, until)
}
pub fn call(command: Command) -> Result<Response> {
    call_at(&endpoint(), command)
}
pub fn call_at(endpoint: &str, command: Command) -> Result<Response> {
    let request = Request::new(command);
    request.validate()?;
    let mut stream = Stream::connect(name(endpoint).map_err(|e| Error::io("IPC name", e))?)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::io("connect BatteryBridge service", e)
            } else {
                Error::ServiceUnavailable(format!("connect BatteryBridge service: {e}"))
            }
        })?;
    send(&mut stream, &request)?;
    let response = receive(&mut stream)?;
    match response {
        Response::Error(e) => Err(e),
        other => Ok(other),
    }
}
