use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use super::utils::raw_mode;

static CSI_U_ENABLED: AtomicBool = AtomicBool::new(false);

fn probe() {
    #[cfg(unix)]
    {
        let mut stdout = std::io::stdout().lock();
        let _ = write!(stdout, "\x1b[?u");
        let _ = stdout.flush();
        drop(stdout);

        std::thread::sleep(std::time::Duration::from_millis(50));

        use std::io::Read;
        use std::os::fd::AsRawFd;

        let stdin = std::io::stdin();
        let fd = stdin.as_raw_fd();

        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL, 0) };
        if flags != -1 {
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        }

        let mut buf = [0u8; 32];
        let mut stdin = stdin.lock();
        if let Ok(n) = stdin.read(&mut buf) {
            if n > 0 && buf[..n].windows(5).any(|w| w == b"\x1b[?0u") {
                CSI_U_ENABLED.store(true, Ordering::Relaxed);
            }
        }

        if flags != -1 {
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags) };
        }
    }
}

fn enable_csi_u() {
    let _ = write!(std::io::stdout(), "\x1b[=5u");
    let _ = std::io::stdout().flush();
}

fn disable_csi_u() {
    let _ = write!(std::io::stdout(), "\x1b[=0u");
    let _ = std::io::stdout().flush();
}

pub fn setup() -> anyhow::Result<()> {
    raw_mode::setup()?;
    probe();
    if CSI_U_ENABLED.load(Ordering::Relaxed) {
        enable_csi_u();
    }
    raw_mode::set_panic_hook();
    Ok(())
}

pub fn teardown() {
    CSI_U_ENABLED.store(false, Ordering::Relaxed);
    disable_csi_u();
    let _ = raw_mode::restore();
}

#[cfg(unix)]
pub fn hold(on: bool) -> anyhow::Result<()> {
    if on {
        raw_mode::restore()?;
        super::app::FULL_RENDER.notify_one();
    } else {
        raw_mode::setup()?
    }
    Ok(())
}

pub fn suspend() {
    disable_csi_u();
    let _ = raw_mode::restore();
}

pub fn resume() -> anyhow::Result<()> {
    raw_mode::setup()?;
    if CSI_U_ENABLED.load(Ordering::Relaxed) {
        enable_csi_u();
    }
    raw_mode::set_panic_hook();
    Ok(())
}
