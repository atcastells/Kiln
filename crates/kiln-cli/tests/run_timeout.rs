//! A tool that hangs is cut at `[tools] timeout_s`, and the run leaves its record.
#![cfg(unix)]
mod common;

use common::temp_project;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::Instant;

#[test]
fn a_hung_blender_is_cut_at_the_timeout_and_logged() {
    let p = temp_project("");
    let fake = p.path().join("fake-blender");
    // no `exec`: the sleep outlives the killed shell and keeps its pipes open
    std::fs::write(&fake, "#!/bin/sh\necho started\nsleep 8\n").unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let toml = p.read("kiln.toml").replace(
        "[tools]\n",
        &format!("[tools]\ntimeout_s = 1\nblender = \"{}\"\n", fake.display()),
    );
    std::fs::write(p.path().join("kiln.toml"), toml).unwrap();

    let started = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["build", "--asset", "hoist", "--project"])
        .arg(p.path())
        .output()
        .expect("kiln build");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{err}");
    assert!(started.elapsed().as_secs() < 6, "no se cortó a tiempo");
    assert!(err.contains("superó el plazo de 1 s"), "{err}");
    let log = p.read("kiln/build/hoist/.kiln/logs/assemble.log");
    assert!(log.contains("exit: timeout") && log.contains("started"), "{log}");
}
