//! The TPM 2.0 emulator used until the beta version.
//!
//! Until the program reaches its beta version it works only with
//! `swtpm`, a TPM 2.0 emulator, and never touches the real TPM of
//! the computer (`docs/todo.md`, step 10 brings the real TPM). The
//! program starts its own emulator and stops it when it ends:
//!
//! ```text
//! <data directory>/tpm-emulator/   the state of the emulated TPM
//!     tpm2-00.permall              its persistent memory
//!     swtpm.pid                    the process of the emulator
//! ```
//!
//! The emulator keeps its state in the data directory, so the
//! sealed secret and the marker of the installation survive a
//! restart, as they would in a real TPM. Each data directory has
//! its own emulated TPM.
//!
//! The emulator listens on two TCP ports of 127.0.0.1 (commands,
//! and a control channel on the next port). Other programs of
//! this computer could reach them; that is acceptable only for
//! development, and one more reason why the release uses the real
//! TPM.

// TODO (step 8 of docs/todo.md): the TPM is an optional factor that
//       is not switched on yet; until then only the tests use this
//       file.
#![allow(dead_code)]

use std::fs::{self, DirBuilder};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, kill_process};

use super::tpm::TpmError;

/// Name of the folder of the emulator in the data directory.
pub const EMULATOR_DIRECTORY: &str = "tpm-emulator";

/// Name of the file with the process id of the emulator.
const PID_FILE: &str = "swtpm.pid";

/// How long the emulator may take to start.
const START_TIMEOUT: Duration = Duration::from_secs(5);

/// A running emulator. Dropping it stops the emulator.
#[derive(Debug)]
pub struct Emulator {
    process: Child,
    port: u16,
    state_directory: PathBuf,
}

impl Emulator {
    /// Starts the emulator of a data directory.
    ///
    /// An emulator left running by an earlier run of the program
    /// (for example after it was killed) is stopped first: it
    /// would hold the state of the TPM locked.
    pub fn start(data_dir: &Path) -> Result<Emulator, TpmError> {
        let state_directory = data_dir.join(EMULATOR_DIRECTORY);

        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&state_directory)
            .map_err(|source| TpmError::io(&state_directory, source))?;

        stop_leftover(&state_directory);

        let port = free_port_pair().map_err(|source| TpmError::io(&state_directory, source))?;

        let process = Command::new("swtpm")
            .arg("socket")
            .arg("--tpm2")
            .arg("--tpmstate")
            .arg(format!("dir={},mode=0600", state_directory.display()))
            .arg("--server")
            .arg(format!("type=tcp,port={port}"))
            .arg("--ctrl")
            .arg(format!("type=tcp,port={}", port + 1))
            .arg("--pid")
            .arg(format!("file={}", state_directory.join(PID_FILE).display()))
            // Ready for commands without a separate start-up step.
            .arg("--flags")
            .arg("not-need-init,startup-clear")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => TpmError::EmulatorMissing,
                _ => TpmError::Emulator(format!("cannot start swtpm: {error}")),
            })?;

        let mut emulator = Emulator {
            process,
            port,
            state_directory,
        };

        emulator.wait_until_ready()?;

        Ok(emulator)
    }

    /// The port on which the emulator takes commands.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Waits until the emulator accepts connections, or fails when
    /// it ended or did not start in time.
    fn wait_until_ready(&mut self) -> Result<(), TpmError> {
        let started = Instant::now();
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, self.port));

        loop {
            if TcpStream::connect(address).is_ok() {
                return Ok(());
            }

            if let Ok(Some(status)) = self.process.try_wait() {
                return Err(TpmError::Emulator(format!("swtpm ended at start ({status})")));
            }

            if started.elapsed() > START_TIMEOUT {
                return Err(TpmError::Emulator("swtpm did not start in time".to_owned()));
            }

            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Emulator {
    /// Stops the emulator. Its state stays on disk.
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        let _ = fs::remove_file(self.state_directory.join(PID_FILE));
    }
}

/// Finds two free neighbouring ports on 127.0.0.1: one for the
/// commands, the next one for the control channel. The system
/// picks the first one.
fn free_port_pair() -> io::Result<u16> {
    for _ in 0..20 {
        let first = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = first.local_addr()?.port();

        if port == u16::MAX {
            continue;
        }

        // Both are free when both can be taken; they are let go
        // just before the emulator takes them.
        if TcpListener::bind((Ipv4Addr::LOCALHOST, port + 1)).is_ok() {
            return Ok(port);
        }
    }

    Err(io::Error::other("no two free neighbouring ports"))
}

/// Stops an emulator that an earlier run left behind for this
/// state directory.
///
/// Only a process that is really `swtpm` with this state directory
/// is stopped: the process id in the file may meanwhile belong to
/// another program.
fn stop_leftover(state_directory: &Path) {
    let pid_file = state_directory.join(PID_FILE);

    let Some(pid) = fs::read_to_string(&pid_file)
        .ok()
        .and_then(|text| text.trim().parse::<i32>().ok())
        .and_then(Pid::from_raw)
    else {
        return;
    };

    // The command line of the process, its words separated by
    // zero bytes.
    let command_line = fs::read(format!("/proc/{}/cmdline", pid.as_raw_nonzero())).unwrap_or_default();
    let command_line = String::from_utf8_lossy(&command_line);

    let is_ours = command_line.starts_with("swtpm")
        && command_line.contains(&state_directory.display().to_string());

    if is_ours {
        let _ = kill_process(pid, Signal::TERM);

        // Give it a moment to write its state and let go of it.
        let started = Instant::now();

        while Path::new(&format!("/proc/{}", pid.as_raw_nonzero())).exists()
            && started.elapsed() < Duration::from_secs(2)
        {
            thread::sleep(Duration::from_millis(20));
        }
    }

    let _ = fs::remove_file(pid_file);
}
