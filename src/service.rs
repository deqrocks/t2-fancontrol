use std::process::Command;

use crate::error::{FanControlError, Result};

const SERVICE_NAME: &str = "t2-fancontrol.service";

pub fn autostart_enabled() -> bool {
    Command::new("systemctl")
        .args(["is-enabled", "--quiet", SERVICE_NAME])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    if enabled {
        install_service()?;
    } else {
        uninstall_service()?;
    }
    Ok(())
}

fn install_service() -> Result<()> {
    run_systemctl(["daemon-reload"])?;
    run_systemctl(["enable", "--now", SERVICE_NAME])?;
    Ok(())
}

fn uninstall_service() -> Result<()> {
    run_systemctl(["disable", "--now", SERVICE_NAME])?;
    run_systemctl(["daemon-reload"])?;
    Ok(())
}

fn run_systemctl<const N: usize>(args: [&str; N]) -> Result<()> {
    let output = Command::new("systemctl")
        .args(args)
        .output()
        .map_err(FanControlError::ProcessSpawn)?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(FanControlError::CommandFailed {
        command: format!("systemctl {}", args.join(" ")),
        stderr,
    })
}
