#![forbid(unsafe_code)]
use batterybridge_core::*;
use batterybridge_ipc as ipc;
use batterybridge_platform::{Platform, PowerEvent};
use batterybridge_service::Service;
use clap::Parser;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(version, about = "BatteryBridge privileged charge-policy service")]
struct Args {
    /// Override the platform's default configuration path.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Override the platform's local IPC endpoint.
    #[arg(long)]
    socket: Option<String>,
    /// Increase log verbosity (-v or -vv).
    #[arg(short,action=clap::ArgAction::Count)]
    verbose: u8,
}
enum Event {
    Connection(ipc::Stream),
    Shutdown,
    Power(PowerEvent),
}
fn run() -> Result<()> {
    let args = Args::parse();
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        match args.verbose {
            0 => "warn",
            1 => "info",
            _ => "debug",
        }
        .into()
    });
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    let endpoint = args.socket.unwrap_or_else(ipc::endpoint);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let parent = std::path::Path::new(&endpoint)
            .parent()
            .ok_or_else(|| Error::ConfigurationError("socket parent missing".into()))?;
        let meta = std::fs::metadata(parent)
            .map_err(|e| Error::io("socket directory must be created by service manager", e))?;
        if meta.mode() & 0o022 != 0 {
            return Err(Error::PermissionDenied(
                "socket directory must not be group/world writable".into(),
            ));
        }
    }
    let mut service = Service::new(
        Platform::discover()?,
        args.config.unwrap_or_else(ipc::config_path),
    )?;
    let listener = ipc::listen(&endpoint)?;
    let (tx, rx) = mpsc::channel();
    let stop = tx.clone();
    ctrlc::set_handler(move || {
        let _ = stop.send(Event::Shutdown);
    })
    .map_err(|e| Error::PlatformError(format!("signal handler: {e}")))?;
    let power_tx = tx.clone();
    let sleeping = batterybridge_platform::watch_power(move |event| {
        let _ = power_tx.send(Event::Power(event));
    })?;
    let pending = Arc::new(AtomicUsize::new(0));
    let accepted = pending.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    // Only this accept thread increments; the policy thread only
                    // decrements, so this check cannot over-reserve a queue slot.
                    if accepted.load(Ordering::Acquire) >= 8 {
                        continue;
                    }
                    accepted.fetch_add(1, Ordering::AcqRel);
                    if tx.send(Event::Connection(stream)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(error=%e,"IPC accept failed");
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
    });
    service.tick();
    let interval = Duration::from_secs(30);
    let mut next_tick = Instant::now() + interval;
    loop {
        if Instant::now() >= next_tick {
            if !sleeping.load(Ordering::SeqCst) {
                service.tick();
            }
            next_tick = Instant::now() + interval;
        }
        match rx.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
            Ok(Event::Shutdown) => break,
            Ok(Event::Power(PowerEvent::Wake)) => {
                service.tick();
                next_tick = Instant::now() + interval;
            }
            #[cfg(target_os = "macos")]
            Ok(Event::Power(PowerEvent::Sleep(ack))) => {
                if let Err(e) = service.prepare_sleep() {
                    tracing::error!(error=%e,"failed to prepare charging for sleep");
                }
                drop(ack);
            }
            Ok(Event::Connection(mut stream)) => {
                pending.fetch_sub(1, Ordering::SeqCst);
                if sleeping.load(Ordering::SeqCst) {
                    continue;
                }
                let response = match ipc::receive::<Request>(&mut stream) {
                    Ok(request) => service.handle(request),
                    Err(e) => Response::Error(e),
                };
                if let Err(e) = ipc::send(&mut stream, &response) {
                    tracing::debug!(error=%e,"IPC response failed");
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let result = service.shutdown();
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(endpoint);
    }
    result
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
