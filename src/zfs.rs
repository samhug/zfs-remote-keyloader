use std::io::{Error, ErrorKind, Result, Write};
use std::process::{Command, Stdio};

pub fn load_key(dataset: &str, key: &str) -> Result<()> {
    let mut child = Command::new("zfs")
        .arg("load-key")
        .arg("-L")
        .arg("file:///proc/self/fd/0")
        .arg(dataset)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    write!(child.stdin.as_mut().unwrap(), "{}", key)?;

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let error_message = std::str::from_utf8(&output.stderr).unwrap();
        return Err(Error::new(ErrorKind::Other, error_message));
    }

    Ok(())
}
