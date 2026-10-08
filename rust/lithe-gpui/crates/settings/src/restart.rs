use std::ffi::OsString;
use std::process::Command;

use gpui::App;

pub fn restart_application(cx: &App) -> bool {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            eprintln!("S1_SETTINGS restart_failed error=current_exe:{error}");
            return false;
        }
    };

    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    match Command::new(&exe).args(&args).spawn() {
        Ok(child) => {
            println!(
                "S1_SETTINGS restarting pid={} exe={} args={}",
                child.id(),
                exe.display(),
                args.len()
            );
            cx.quit();
            true
        }
        Err(error) => {
            eprintln!(
                "S1_SETTINGS restart_failed exe={} error={error}",
                exe.display()
            );
            false
        }
    }
}
