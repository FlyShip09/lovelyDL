//! Runs a tool as a child process: argv only, no shell, no console window, both pipes read
//! line by line, and the whole process tree stoppable at any moment.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

/// A command for `program` with no console window and no inherited stdin.
pub fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    cmd
}

/// Stops a running process tree. Dropping it without calling [`Stop::stop`] leaves the
/// process running.
pub struct Stop(oneshot::Sender<Option<PathBuf>>);

impl Stop {
    /// Kills the process and its children. `cleanup`, if given, is a directory to delete once
    /// the process has really exited and released its files.
    pub fn stop(self, cleanup: Option<PathBuf>) {
        let _ = self.0.send(cleanup);
    }
}

async fn pump<R: AsyncRead + Unpin>(reader: R, tx: mpsc::UnboundedSender<String>) {
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            // Output is not guaranteed to be UTF-8; never let one odd byte lose a line.
            Ok(_) => {
                if tx.send(String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string()).is_err() {
                    break;
                }
            }
        }
    }
}

/// Spawns `program` and calls `on_line` for every line of stdout and stderr, then `on_exit`
/// with the exit code (`None` if it was killed). Must be called inside a tokio runtime.
pub fn start(
    program: &Path,
    args: &[String],
    mut on_line: impl FnMut(String) + Send + 'static,
    on_exit: impl FnOnce(Option<i32>) + Send + 'static,
) -> std::io::Result<Stop> {
    let mut child = command(program).args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let tree = tree::Tree::adopt(&child);
    let (line_tx, mut line_rx) = mpsc::unbounded_channel();
    if let Some(out) = child.stdout.take() {
        tokio::spawn(pump(out, line_tx.clone()));
    }
    if let Some(err) = child.stderr.take() {
        tokio::spawn(pump(err, line_tx.clone()));
    }
    drop(line_tx);
    let (stop_tx, mut stop_rx) = oneshot::channel::<Option<PathBuf>>();
    tokio::spawn(async move {
        let mut stopped = false;
        // A finished oneshot must not be polled again, so stop listening after it fires.
        let mut listening = true;
        let mut cleanup = None;
        loop {
            tokio::select! {
                line = line_rx.recv() => match line {
                    Some(line) if !stopped => on_line(line),
                    Some(_) => {}
                    None => break,
                },
                request = &mut stop_rx, if listening => {
                    listening = false;
                    // An `Err` means the handle was dropped without a stop: keep running.
                    if let Ok(dir) = request {
                        stopped = true;
                        cleanup = dir;
                        tree.kill(&mut child);
                    }
                }
            }
        }
        let code = child.wait().await.ok().and_then(|status| status.code());
        drop(tree);
        if let Some(dir) = cleanup {
            let _ = tokio::fs::remove_dir_all(dir).await;
        }
        on_exit(if stopped { None } else { code });
    });
    Ok(Stop(stop_tx))
}

#[derive(Debug)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a short command to completion and returns what it printed. Gives up after `secs`.
pub async fn output(program: &Path, args: &[String], secs: u64) -> std::io::Result<Output> {
    let child = command(program).args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let run = tokio::time::timeout(std::time::Duration::from_secs(secs), child.wait_with_output());
    match run.await {
        Ok(Ok(out)) => Ok(Output {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
        Ok(Err(error)) => Err(error),
        // `kill_on_drop` ends the process when the timed-out future is dropped.
        Err(_) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "the tool did not answer in time")),
    }
}

#[cfg(windows)]
mod tree {
    //! A Job Object: everything the child spawns (ffmpeg, Deno) belongs to it, one call ends
    //! them all, and Windows ends them if lovelyDL itself dies.

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
        TerminateJobObject,
    };

    pub struct Tree(HANDLE);

    // SAFETY: a job handle is a kernel object reference that any thread may use.
    unsafe impl Send for Tree {}

    impl Tree {
        // shortcut: a grandchild started before the assignment below would escape the job;
        // yt-dlp starts nothing in its first milliseconds. Upgrade to CREATE_SUSPENDED if a
        // tool ever does.
        pub fn adopt(child: &tokio::process::Child) -> Self {
            // SAFETY: plain Win32 calls with valid arguments; every result is checked.
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return Self(job);
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    (&raw const info).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if let Some(handle) = child.raw_handle() {
                    AssignProcessToJobObject(job, handle as HANDLE);
                }
                Self(job)
            }
        }

        pub fn kill(&self, child: &mut tokio::process::Child) {
            if !self.0.is_null() {
                // SAFETY: the handle is a live job object owned by `self`.
                unsafe { TerminateJobObject(self.0, 1) };
            }
            // Covers the case where the job could not be created.
            let _ = child.start_kill();
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: the handle is owned by `self` and closed exactly once.
                unsafe { CloseHandle(self.0) };
            }
        }
    }
}

#[cfg(not(windows))]
mod tree {
    //! Other systems: only the direct child is killed.

    pub struct Tree;

    impl Tree {
        pub fn adopt(_: &tokio::process::Child) -> Self {
            Self
        }

        pub fn kill(&self, child: &mut tokio::process::Child) {
            let _ = child.start_kill();
        }
    }
}
