//! Sleep notifications. The policy thread releases adapter isolation before acknowledging sleep.
use crate::PowerEvent;
use batterybridge_core::*;
use std::{
    ffi::c_void,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};
type Cf = *const c_void;
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IORegisterForSystemPower(
        context: *mut c_void,
        port: *mut *mut c_void,
        callback: extern "C" fn(*mut c_void, u32, u32, *mut c_void),
        notifier: *mut u32,
    ) -> u32;
    fn IONotificationPortGetRunLoopSource(port: *mut c_void) -> Cf;
    fn IONotificationPortDestroy(port: *mut c_void);
    fn IODeregisterForSystemPower(notifier: *mut u32) -> i32;
    fn IOServiceClose(connection: u32) -> i32;
    fn IOAllowPowerChange(connection: u32, token: isize) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopDefaultMode: Cf;
    fn CFRunLoopGetCurrent() -> Cf;
    fn CFRunLoopAddSource(runloop: Cf, source: Cf, mode: Cf);
    fn CFRunLoopRun();
}
/// Owns a notification acknowledgement, not the connection. The connection lives
/// on the power thread until process exit. Drop acknowledges once, including send failures.
pub struct SleepAck {
    connection: u32,
    token: isize,
}
impl Drop for SleepAck {
    fn drop(&mut self) {
        // SAFETY: only the callback constructs tokens delivered by IOKit;
        // the registration connection outlives all events and each ack is owned once.
        unsafe {
            IOAllowPowerChange(self.connection, self.token);
        }
    }
}
struct Context {
    callback: Box<dyn Fn(PowerEvent) + Send>,
    sleeping: Arc<AtomicBool>,
    connection: u32,
}
extern "C" fn notify(context: *mut c_void, _service: u32, message: u32, argument: *mut c_void) {
    // SAFETY: a stable Box<Context> is kept alive for the registered run loop.
    let context = unsafe { &*context.cast::<Context>() };
    match message {
        0xe0000270 => drop(SleepAck {
            connection: context.connection,
            token: argument as isize,
        }),
        0xe0000280 => {
            context.sleeping.store(true, Ordering::SeqCst);
            (context.callback)(PowerEvent::Sleep(SleepAck {
                connection: context.connection,
                token: argument as isize,
            }));
        }
        0xe0000300 => {
            context.sleeping.store(false, Ordering::SeqCst);
            (context.callback)(PowerEvent::Wake);
        }
        _ => {}
    }
}
pub fn watch(callback: impl Fn(PowerEvent) + Send + 'static) -> Result<Arc<AtomicBool>> {
    let sleeping = Arc::new(AtomicBool::new(false));
    let state = sleeping.clone();
    let (ready, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut context = Box::new(Context {
            callback: Box::new(callback),
            sleeping: state,
            connection: 0,
        });
        let mut port = ptr::null_mut();
        let mut notifier = 0;
        // SAFETY: exclusive initialized out parameters and stable context pointer;
        // notification processing starts only after the connection field is set.
        let connection = unsafe {
            IORegisterForSystemPower(
                (&mut *context as *mut Context).cast(),
                &mut port,
                notify,
                &mut notifier,
            )
        };
        if connection == 0 {
            let _ = ready.send(false);
            return;
        }
        context.connection = connection;
        // SAFETY: registration returned a live notification port and runloop source.
        unsafe {
            CFRunLoopAddSource(
                CFRunLoopGetCurrent(),
                IONotificationPortGetRunLoopSource(port),
                kCFRunLoopDefaultMode,
            );
        }
        let _ = ready.send(true);
        // SAFETY: processes callbacks while context and registration remain alive.
        unsafe {
            CFRunLoopRun();
        }
        // SAFETY: run loop stopped; unregister callbacks before dropping context.
        unsafe {
            IODeregisterForSystemPower(&mut notifier);
            IONotificationPortDestroy(port);
            IOServiceClose(connection);
        }
    });
    if rx.recv().unwrap_or(false) {
        Ok(sleeping)
    } else {
        Err(Error::BackendUnavailable(
            "IOKit sleep notifications unavailable; service refuses SMC writes".into(),
        ))
    }
}
