//! Starts the server like `CommandStarter` of `fs-interfaces`, but keeps what the server says on its stderr in a file, so a
//! start that fails can show the REASON in the window (the server's own sentence, e.g. a data folder that is not private)
//! and not only "No such file or directory" of the missing socket.
//!
//! Pattern: Strategy (a `ServerStarter` of the program) + Decorator over the command of `CommandStarter`: same detached
//! command, but its stderr goes to a file next to the socket instead of nowhere.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use fs_interfaces::rpc::ServerStarter;

/// The command that starts the server and the file its stderr goes to.
pub struct LoggedStarter {
    program: PathBuf,
    args: Vec<OsString>,
    log: PathBuf,
}

impl LoggedStarter {
    /// `program args...`, its stderr kept in `log` (made new at each start).
    pub fn new(program: PathBuf, args: Vec<OsString>, log: PathBuf) -> Self {
        Self { program, args, log }
    }
}

impl ServerStarter for LoggedStarter {
    fn start(&self) -> io::Result<()> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        // No file to write (the folder of the socket is the problem itself): the server then says nothing, as before.
        match std::fs::File::create(&self.log) {
            Ok(file) => command.stderr(file),
            Err(_) => command.stderr(Stdio::null()),
        };
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0000_0208);
        let mut child = command.spawn()?;
        // The server outlives this call; the thread collects the child's exit state if it ends early (no zombie).
        std::thread::spawn(move || drop(child.wait()));
        Ok(())
    }
}

/// What the server said on its stderr (the last lines, at most about 600 characters); none = it said nothing.
pub fn said_by_server(log: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log).ok()?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let tail = lines[lines.len().saturating_sub(4)..].join(" / ");
    let tail = tail.trim();
    if tail.is_empty() {
        return None;
    }
    let skip = tail.chars().count().saturating_sub(600);
    Some(tail.chars().skip(skip).collect())
}
