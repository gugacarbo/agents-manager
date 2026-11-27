#![allow(clippy::unwrap_used)]
use agents_manager_core::domain::{SshConnection, SshEnvironmentDraft};
use agents_manager_remote::ssh;
use tempfile::tempdir;
#[test]
fn registry_persists_and_rejects_invalid_before_saving() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("data/ssh.json");
    let invalid = SshEnvironmentDraft {
        name: "server".into(),
        connection: SshConnection::Alias {
            alias: "-F/etc/passwd".into(),
        },
    };
    assert!(ssh::save_environment(&path, None, &invalid).is_err());
    assert!(!path.exists());
    let good = SshEnvironmentDraft {
        name: "server".into(),
        connection: SshConnection::Manual {
            host: "example.org".into(),
            user: "alice".into(),
            port: 22,
        },
    };
    let saved = ssh::save_environment(&path, None, &good).unwrap();
    assert_eq!(ssh::list_environments(&path).unwrap(), vec![saved.clone()]);
    ssh::remove_environment(&path, &saved.id).unwrap();
    assert!(ssh::list_environments(&path).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn errors_are_typed_and_timeout_does_not_wait_for_children() {
    use agents_manager_core::{domain::SshErrorKind, error::CoreError};
    use std::{
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };
    let tmp = tempdir().unwrap();
    let script = tmp.path().join("ssh");
    std::fs::write(&script, "#!/bin/sh\nsleep 2 &\nwait\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let draft = SshEnvironmentDraft {
        name: "x".into(),
        connection: SshConnection::Alias {
            alias: "lab".into(),
        },
    };
    let transport = ssh::SshTransport::new(script, None, Duration::from_millis(100));
    let start = Instant::now();
    assert!(matches!(
        transport.inventory(&draft),
        Err(CoreError::SshError {
            kind: SshErrorKind::Timeout,
            ..
        })
    ));
    assert!(start.elapsed() < Duration::from_secs(1));
    let missing = ssh::SshTransport::new(tmp.path().join("missing"), None, Duration::from_secs(1));
    assert!(matches!(
        missing.inventory(&draft),
        Err(CoreError::SshError {
            kind: SshErrorKind::ClientUnavailable,
            ..
        })
    ));
    for draft in [
        SshEnvironmentDraft {
            name: "x".into(),
            connection: SshConnection::Manual {
                host: "host".into(),
                user: "--option".into(),
                port: 22,
            },
        },
        SshEnvironmentDraft {
            name: "x".into(),
            connection: SshConnection::Manual {
                host: "host".into(),
                user: "user".into(),
                port: 0,
            },
        },
    ] {
        assert!(matches!(
            transport.inventory(&draft),
            Err(CoreError::SshError {
                kind: SshErrorKind::InvalidInput,
                ..
            })
        ));
    }
}

#[cfg(unix)]
#[test]
fn real_openssh_alias_and_manual_auth_identity_and_host_key_failures() {
    use agents_manager_core::{domain::SshErrorKind, error::CoreError};
    use std::{
        fs,
        net::{TcpListener, TcpStream},
        os::unix::fs::{PermissionsExt, symlink},
        process::{Child, Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    struct Server(Child);
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn run_keygen(path: &std::path::Path) {
        let status = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", ""])
            .arg("-f")
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success());
    }
    let temp = tempdir().unwrap();
    let root = temp.path();
    let server_key = root.join("host_key");
    let client_key = root.join("client_key");
    let wrong_key = root.join("wrong_key");
    run_keygen(&server_key);
    run_keygen(&client_key);
    run_keygen(&wrong_key);
    fs::copy(root.join("client_key.pub"), root.join("authorized_keys")).unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let user = String::from_utf8(Command::new("id").arg("-un").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_owned();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    symlink("/usr/bin/hostname", bin.join("hostname")).unwrap();
    symlink("/usr/bin/id", bin.join("id")).unwrap();
    let claude = bin.join("claude");
    let marker = root.join("agent_was_executed");
    fs::write(
        &claude,
        format!("#!/bin/sh\nprintf executed > '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o700)).unwrap();
    let forced = root.join("command.sh");
    fs::write(
        &forced,
        format!(
            "#!/bin/sh\nPATH='{}'; export PATH\nexec /bin/sh -c \"$SSH_ORIGINAL_COMMAND\"\n",
            bin.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&forced, fs::Permissions::from_mode(0o700)).unwrap();
    let sshd = std::env::var_os("AGENTS_MANAGER_SSHD")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/usr/sbin/sshd".into());
    assert!(
        sshd.is_file(),
        "Install openssh-server or set AGENTS_MANAGER_SSHD to a disposable sshd binary"
    );
    let mut extra = String::new();
    if let Some(base) = sshd.parent().and_then(|p| p.parent()) {
        let session = base.join("lib/openssh/sshd-session");
        let auth = base.join("lib/openssh/sshd-auth");
        if session.exists() && auth.exists() {
            extra = format!(
                "SshdSessionPath {}\nSshdAuthPath {}\n",
                session.display(),
                auth.display()
            );
        }
    }
    let server_config = root.join("sshd_config");
    fs::write(&server_config,format!("Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nStrictModes no\nAllowUsers {user}\nForceCommand {}\nSetEnv HOME={}\n{extra}",server_key.display(),root.join("pid").display(),root.join("authorized_keys").display(),forced.display(),root.display())).unwrap();
    let server = Command::new(&sshd)
        .arg("-D")
        .arg("-e")
        .arg("-f")
        .arg(&server_config)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut server = Server(server);
    let start = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(
            server.0.try_wait().unwrap().is_none(),
            "sshd exited; verify isolated sshd dependencies"
        );
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "sshd did not start"
        );
        thread::sleep(Duration::from_millis(20));
    }
    let hostkey = fs::read_to_string(root.join("host_key.pub")).unwrap();
    let known = root.join("known_hosts");
    fs::write(&known, format!("[127.0.0.1]:{port} {hostkey}")).unwrap();
    let config = root.join("ssh_config");
    let make_config = |key: &std::path::Path, known: &std::path::Path| {
        format!(
            "Host lab\n  HostName 127.0.0.1\n  User {user}\n  Port {port}\nHost *\n  IdentityFile {}\n  IdentitiesOnly yes\n  UserKnownHostsFile {}\n  GlobalKnownHostsFile /dev/null\n  IdentityAgent none\n",
            key.display(),
            known.display()
        )
    };
    fs::write(&config, make_config(&client_key, &known)).unwrap();
    let transport =
        ssh::SshTransport::new("ssh".into(), Some(config.clone()), Duration::from_secs(5));
    let alias = SshEnvironmentDraft {
        name: "alias".into(),
        connection: SshConnection::Alias {
            alias: "lab".into(),
        },
    };
    let manual = SshEnvironmentDraft {
        name: "manual".into(),
        connection: SshConnection::Manual {
            host: "127.0.0.1".into(),
            user: user.clone(),
            port,
        },
    };
    for draft in [&alias, &manual] {
        let result = transport.inventory(draft).unwrap();
        assert_eq!(result.user, user);
        assert!(!result.host.is_empty());
        assert_eq!(result.port, port);
        assert!(result.claude);
        assert!(!result.codex);
    }
    assert!(!marker.exists(), "inventory must not execute agents");
    fs::remove_file(claude).unwrap();
    let absent = transport.inventory(&alias).unwrap();
    assert!(!absent.claude && !absent.codex);
    let local_bin = root.join(".local/bin");
    fs::create_dir_all(&local_bin).unwrap();
    let local_claude = local_bin.join("claude");
    let local_codex = local_bin.join("codex");
    for path in [&local_claude, &local_codex] {
        fs::write(
            path,
            format!("#!/bin/sh\nprintf executed > '{}'\n", marker.display()),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let outside_path = transport.inventory(&alias).unwrap();
    assert!(outside_path.claude && outside_path.codex);
    assert!(!marker.exists(), "inventory must not execute agents");
    fs::set_permissions(&local_codex, fs::Permissions::from_mode(0o600)).unwrap();
    let not_executable = transport.inventory(&alias).unwrap();
    assert!(not_executable.claude && !not_executable.codex);
    let empty = root.join("empty_known_hosts");
    fs::write(&empty, "").unwrap();
    fs::write(&config, make_config(&client_key, &empty)).unwrap();
    assert!(matches!(
        transport.inventory(&alias),
        Err(CoreError::SshError {
            kind: SshErrorKind::HostKey,
            ..
        })
    ));
    let changed = root.join("changed_known_hosts");
    fs::write(
        &changed,
        format!(
            "[127.0.0.1]:{port} {}",
            fs::read_to_string(root.join("wrong_key.pub")).unwrap()
        ),
    )
    .unwrap();
    fs::write(&config, make_config(&client_key, &changed)).unwrap();
    assert!(matches!(
        transport.inventory(&alias),
        Err(CoreError::SshError {
            kind: SshErrorKind::HostKey,
            ..
        })
    ));
    fs::write(&config, make_config(&wrong_key, &known)).unwrap();
    assert!(matches!(
        transport.inventory(&alias),
        Err(CoreError::SshError {
            kind: SshErrorKind::Authentication,
            ..
        })
    ));
}
