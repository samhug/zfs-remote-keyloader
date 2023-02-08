use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context};

static ZFS_COMMAND: &str = "zfs";

pub fn load_key(dataset: &str, key: &str) -> anyhow::Result<()> {
    let mut child = Command::new(ZFS_COMMAND)
        .args(["load-key", "-L", "file:///proc/self/fd/0", "--"])
        .arg(dataset)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn zfs command")?;

    write!(child.stdin.as_mut().unwrap(), "{}", key)?;

    let output = child.wait_with_output()?;

    if !output.status.success() {
        anyhow::bail!(
            "zfs load-key failed: {}",
            std::str::from_utf8(&output.stderr)?
        );
    }

    Ok(())
}

pub fn get_keystatus(dataset: &str) -> anyhow::Result<KeyStatus> {
    let output = Command::new(ZFS_COMMAND)
        .args(["get", "-H", "-o", "value", "keystatus", "--"])
        .arg(dataset)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn zfs command")?
        .wait_with_output()?;

    if !output.status.success() {
        anyhow::bail!(
            "failed to get keystatus: {}",
            std::str::from_utf8(&output.stderr)?
        );
    }

    match std::str::from_utf8(&output.stdout)?.trim() {
        "available" => Ok(KeyStatus::Available),
        "unavailable" => Ok(KeyStatus::Unavailable),
        "-" => Err(anyhow!("dataset is not encrypted")),
        v => Err(anyhow!("unexpected value for keystatus: {v:?}")),
    }
}

pub enum KeyStatus {
    Available,
    Unavailable,
}
