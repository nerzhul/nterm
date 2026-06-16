use std::os::unix::io::AsRawFd;
use vte4::Terminal;
use vte4::prelude::*;

/// Check if a terminal has a foreground process running (not just the shell).
pub fn has_foreground_process(terminal: &Terminal) -> Option<String> {
    if let Some(pty) = terminal.pty() {
        let fd = pty.fd().as_raw_fd();
        let fg_pgid = unsafe { libc::tcgetpgrp(fd) };
        if fg_pgid > 0 {
            let stat_path = format!("/proc/{}/stat", fg_pgid);
            if let Ok(stat_content) = std::fs::read_to_string(&stat_path) {
                if let Some(start) = stat_content.find('(') {
                    if let Some(end) = stat_content.rfind(')') {
                        let cmd_name = &stat_content[start + 1..end];
                        let is_shell = matches!(
                            cmd_name,
                            "bash" | "zsh" | "sh" | "fish" | "dash" | "ksh" | "csh" | "tcsh"
                        );
                        if !is_shell {
                            return Some(cmd_name.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}
