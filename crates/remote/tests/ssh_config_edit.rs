#![allow(clippy::unwrap_used)]
#[cfg(unix)]
#[path = "ssh_atomic_probe.rs"]
mod atomic_probe;
use agents_manager_core::{
    domain::{AgentKind, ConfigFormat, RemoteConfigDocument, RemoteConfigEntry, RemoteConfigId},
    error::CoreError,
};
use agents_manager_remote::remote_config;

/// Writes an executable with its final mode in a single `open` (no writable intermediate state).
fn write_executable(path: &std::path::Path, contents: &str) {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o700)
        .open(path)
        .unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    file.sync_all().unwrap();
    drop(file);
}
fn document() -> RemoteConfigDocument {
    RemoteConfigDocument {
        entry: RemoteConfigEntry {
            id: RemoteConfigId::ClaudeHome,
            agent: AgentKind::ClaudeCode,
            label: "Claude".into(),
            format: ConfigFormat::Json,
        },
        host: "remote".into(),
        user: "user".into(),
        port: 22,
        path: "/home/user/.claude.json".into(),
        content: r#"{"projects":{"/home/user/project":{"enabled":true}},"theme":"dark"}"#.into(),
        sha256: "hash".into(),
        secret_spans: vec![],
        host_key: "SHA256:test".into(),
        device: 1,
        inode: 2,
        mode: 0o600,
        uid: 1000,
        gid: 1000,
        metadata_hash: "meta".into(),
    }
}
#[test]
fn backend_rejects_invalid_syntax_and_edits_to_protected_project_data() {
    let doc = document();
    assert!(matches!(
        remote_config::validate_draft(&doc, "{"),
        Err(CoreError::SyntaxError { .. })
    ));
    assert!(matches!(
        remote_config::validate_draft(&doc, r#"{"projects":{},"theme":"dark"}"#),
        Err(CoreError::ProtectedField)
    ));
    assert!(
        remote_config::validate_draft(
            &doc,
            r#"{"projects":{"/home/user/project":{"enabled":true}},"theme":"light"}"#
        )
        .is_ok()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn real_openssh_read_save_conflict_symlinks_and_backup_fail_closed() {
    use agents_manager_core::domain::{SshConnection, SshEnvironmentDraft};
    use agents_manager_remote::ssh::SshTransport;
    use std::{
        fs,
        net::{TcpListener, TcpStream},
        os::unix::fs::{MetadataExt, PermissionsExt, symlink},
        process::{Child, Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    use tempfile::tempdir;
    struct Guard(Child);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn key(path: &std::path::Path) {
        assert!(
            Command::new("ssh-keygen")
                .args(["-q", "-t", "ed25519", "-N", ""])
                .arg("-f")
                .arg(path)
                .status()
                .unwrap()
                .success()
        );
    }
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let home = root.join("synthetic-home");
    fs::create_dir_all(home.join(".claude")).unwrap();
    fs::create_dir_all(home.join(".codex")).unwrap();
    let file = home.join(".claude.json");
    fs::write(&file, b"{\"projects\":{\"x\":1},\"theme\":\"dark\"}\n").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    let shim = root.join("pwshim.c");
    fs::write(&shim,r#"#define _GNU_SOURCE
#include <pwd.h>
#include <stdlib.h>
#include <dlfcn.h>
struct passwd *getpwuid(uid_t id) { static struct passwd copy; struct passwd *(*original)(uid_t)=dlsym(RTLD_NEXT,"getpwuid"); struct passwd *p=original(id); if (!p) return p; copy=*p; copy.pw_dir=getenv("AM_TEST_HOME"); return &copy; }
int getpwuid_r(uid_t id, struct passwd *pwd, char *buf, size_t len, struct passwd **result) { int (*original)(uid_t,struct passwd*,char*,size_t,struct passwd**)=dlsym(RTLD_NEXT,"getpwuid_r"); int ret=original(id,pwd,buf,len,result); if (!ret && *result) pwd->pw_dir=getenv("AM_TEST_HOME"); return ret; }
"#).unwrap();
    let so = root.join("pwshim.so");
    assert!(
        Command::new("cc")
            .args(["-shared", "-fPIC"])
            .arg(&shim)
            .args(["-o"])
            .arg(&so)
            .arg("-ldl")
            .status()
            .unwrap()
            .success()
    );
    let host = root.join("host");
    let client = root.join("client");
    key(&host);
    key(&client);
    fs::copy(root.join("client.pub"), root.join("authorized_keys")).unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let user = String::from_utf8(Command::new("id").arg("-un").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_owned();
    let drop_flag = root.join("drop-confirmation");
    let attempts = root.join("confirmed-writes");
    let proxy = root.join("drop_ack.py");
    fs::write(&proxy, r#"import json,os,subprocess,sys,threading
child=subprocess.Popen(os.environ['SSH_ORIGINAL_COMMAND'],shell=True,stdin=subprocess.PIPE,stdout=subprocess.PIPE)
def send():
    try:
        for line in sys.stdin.buffer:
            child.stdin.write(line); child.stdin.flush()
    except (BrokenPipeError, OSError): pass
threading.Thread(target=send,daemon=True).start()
for line in child.stdout:
    obj=json.loads(line)
    if 'sha256' in obj and 'content' not in obj:
        with open(os.environ['AM_TEST_ATTEMPTS'],'a') as f: f.write('x')
        child.kill()
        os._exit(0)
    sys.stdout.buffer.write(line); sys.stdout.buffer.flush()
"#).unwrap();
    let wrapper = root.join("force.sh");
    write_executable(
        &wrapper,
        &format!(
            "#!/bin/sh\nexport AM_TEST_HOME='{}' LD_PRELOAD='{}' AM_TEST_ATTEMPTS='{}'\n[ \"$(/usr/bin/python3 -c 'import os,pwd; print(pwd.getpwuid(os.getuid()).pw_dir)')\" = \"$AM_TEST_HOME\" ] || exit 99\nif [ -f '{}' ]; then exec /usr/bin/python3 -u '{}'; fi\nexec /bin/sh -c \"$SSH_ORIGINAL_COMMAND\"\n",
            home.display(),
            so.display(),
            attempts.display(),
            drop_flag.display(),
            proxy.display()
        ),
    );
    let sshd = std::env::var_os("AGENTS_MANAGER_SSHD")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/usr/sbin/sshd".into());
    assert!(
        sshd.is_file(),
        "install openssh-server or set AGENTS_MANAGER_SSHD"
    );
    let mut extra = String::new();
    if let Some(base) = sshd.parent().and_then(std::path::Path::parent) {
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
    let cfg = root.join("sshd_config");
    fs::write(&cfg,format!("Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nStrictModes no\nAllowUsers {user}\nForceCommand {}\nSetEnv HOME={}\n{extra}",host.display(),root.join("pid").display(),root.join("authorized_keys").display(),wrapper.display(),home.display())).unwrap();
    let child = Command::new(sshd)
        .args(["-D", "-e", "-f"])
        .arg(cfg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut server = Guard(child);
    let start = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(server.0.try_wait().unwrap().is_none());
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(20));
    }
    let known = root.join("known_hosts");
    fs::write(
        &known,
        format!(
            "[127.0.0.1]:{port} {}",
            fs::read_to_string(root.join("host.pub")).unwrap()
        ),
    )
    .unwrap();
    let cfg = root.join("ssh_config");
    fs::write(&cfg,format!("Host fixture\n HostName 127.0.0.1\n User {user}\n Port {port}\n IdentityFile {}\n IdentitiesOnly yes\n IdentityAgent none\n UserKnownHostsFile {}\n GlobalKnownHostsFile /dev/null\n",client.display(),known.display())).unwrap();
    let transport = SshTransport::new("ssh".into(), Some(cfg), Duration::from_secs(10));
    let draft = SshEnvironmentDraft {
        name: "fixture".into(),
        connection: SshConnection::Alias {
            alias: "fixture".into(),
        },
    };
    let xattr = Command::new("/usr/bin/python3")
        .args([
            "-c",
            "import os,sys; os.setxattr(sys.argv[1], 'user.fixture', b'preserved')",
        ])
        .arg(&file)
        .status()
        .unwrap();
    assert!(xattr.success());
    let doc = remote_config::read_config(&transport, &draft, RemoteConfigId::ClaudeHome).unwrap();
    assert_eq!(doc.path, file.display().to_string());
    assert_eq!(doc.port, port);
    assert_eq!(doc.user, user);
    assert!(doc.host_key.starts_with("SHA256:"));
    let before = fs::metadata(&file).unwrap();
    let saved = remote_config::save_config(
        &transport,
        &draft,
        &doc,
        r#"{"projects":{"x":1},"theme":"light"}"#,
        &root.join("backups"),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&saved.backup_path).unwrap(), doc.content);
    assert_eq!(
        fs::metadata(&saved.backup_path).unwrap().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(root.join("backups")).unwrap().mode() & 0o777,
        0o700
    );
    let after = fs::metadata(&file).unwrap();
    assert_eq!(before.mode() & 0o7777, after.mode() & 0o7777);
    assert_eq!((before.uid(), before.gid()), (after.uid(), after.gid()));
    let xattr = Command::new("/usr/bin/python3")
        .args([
            "-c",
            "import os,sys; sys.stdout.buffer.write(os.getxattr(sys.argv[1], 'user.fixture'))",
        ])
        .arg(&file)
        .output()
        .unwrap();
    assert!(xattr.status.success());
    assert_eq!(xattr.stdout, b"preserved");
    let mut wrong_identity =
        remote_config::read_config(&transport, &draft, RemoteConfigId::ClaudeHome).unwrap();
    wrong_identity.host_key = "SHA256:not-the-server".into();
    assert!(matches!(
        remote_config::save_config(
            &transport,
            &draft,
            &wrong_identity,
            r#"{"projects":{"x":1},"theme":"wrong"}"#,
            &root.join("backups")
        ),
        Err(CoreError::RemoteDestinationChanged)
    ));
    let verified_before_drop =
        remote_config::read_config(&transport, &draft, RemoteConfigId::ClaudeHome).unwrap();
    fs::write(&drop_flag, "drop").unwrap();
    let uncertain_content = r#"{"projects":{"x":1},"theme":"after-drop"}"#;
    assert!(matches!(
        remote_config::save_config(
            &transport,
            &draft,
            &verified_before_drop,
            uncertain_content,
            &root.join("backups")
        ),
        Err(CoreError::RemoteUnconfirmed)
    ));
    fs::remove_file(&drop_flag).unwrap();
    let verified_after_drop =
        remote_config::read_config(&transport, &draft, RemoteConfigId::ClaudeHome).unwrap();
    assert_eq!(verified_after_drop.content, uncertain_content);
    assert_eq!(fs::read_to_string(&attempts).unwrap(), "x");
    assert!(
        fs::read_dir(root.join("backups"))
            .unwrap()
            .any(|entry| fs::read(entry.unwrap().path()).unwrap()
                == verified_before_drop.content.as_bytes())
    );
    let old = remote_config::read_config(&transport, &draft, RemoteConfigId::ClaudeHome).unwrap();
    fs::write(&file, "{}\n").unwrap();
    assert!(matches!(
        remote_config::save_config(
            &transport,
            &draft,
            &old,
            r#"{"projects":{"x":1},"theme":"different"}"#,
            &root.join("backups")
        ),
        Err(CoreError::ExternalConflict)
    ));
    let current =
        remote_config::read_config(&transport, &draft, RemoteConfigId::ClaudeHome).unwrap();
    let blocker = root.join("blocker");
    fs::write(&blocker, "file").unwrap();
    assert!(
        remote_config::save_config(&transport, &draft, &current, "{\"x\":1}", &blocker).is_err()
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), "{}\n");
    let toml_path = home.join(".codex/config.toml");
    fs::write(&toml_path, "key = 1\n").unwrap();
    let toml_doc =
        remote_config::read_config(&transport, &draft, RemoteConfigId::CodexConfig).unwrap();
    fs::set_permissions(home.join(".codex"), fs::Permissions::from_mode(0o500)).unwrap();
    let failed = remote_config::save_config(
        &transport,
        &draft,
        &toml_doc,
        "key = 2\n",
        &root.join("backups"),
    );
    fs::set_permissions(home.join(".codex"), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(failed.is_err());
    assert_eq!(fs::read_to_string(&toml_path).unwrap(), "key = 1\n");
    fs::write(home.join(".codex/config.toml"), [0xff]).unwrap();
    assert!(remote_config::read_config(&transport, &draft, RemoteConfigId::CodexConfig).is_err());
    fs::remove_file(home.join(".codex/config.toml")).unwrap();
    symlink(&file, home.join(".codex/config.toml")).unwrap();
    assert!(matches!(
        remote_config::read_config(&transport, &draft, RemoteConfigId::CodexConfig),
        Err(CoreError::PathNotAllowed { .. })
    ));
}

#[cfg(unix)]
#[test]
fn lost_confirmation_does_not_retry_or_report_success() {
    use agents_manager_core::domain::{SshConnection, SshEnvironmentDraft};
    use agents_manager_remote::ssh::SshTransport;
    use sha2::{Digest, Sha256};
    use std::{fs, time::Duration};
    use tempfile::tempdir;
    let tmp = tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir(&home).unwrap();
    let target = home.join(".claude.json");
    let original = "{\"projects\":{},\"theme\":\"dark\"}";
    fs::write(&target, original).unwrap();
    let hash = format!("{:x}", Sha256::digest(original.as_bytes()));
    let counter = tmp.path().join("attempts");
    let program = tmp.path().join("ssh-fixture");
    let code = format!(
        r#"#!/usr/bin/python3
import json,sys
if '-G' in sys.argv: print('port 22'); sys.exit(0)
print('debug1: Connecting to fixture port 22.',file=sys.stderr,flush=True)
print('debug1: Server host key: ssh-ed25519 SHA256:fixture',file=sys.stderr,flush=True)
print(json.dumps({{'host':'fixture','user':'fixture','home':{home:?}}}),flush=True)
for line in sys.stdin:
 req=json.loads(line)
 if req['op']=='read': print(json.dumps({{'path':{target:?},'content':{original:?},'sha256':{hash:?},'device':1,'inode':2,'mode':33152,'uid':1000,'gid':1000,'metadata_hash':'meta'}}),flush=True)
 else:
  open({counter:?},'a').write('x')
  open({target:?},'w').write(req['content'])
  sys.exit(1)
"#,
        home = home.to_str().unwrap(),
        target = target.to_str().unwrap(),
        original = original,
        hash = hash,
        counter = counter.to_str().unwrap()
    );
    write_executable(&program, &code);
    let transport = SshTransport::new(program, None, Duration::from_secs(2));
    let draft = SshEnvironmentDraft {
        name: "fixture".into(),
        connection: SshConnection::Alias {
            alias: "fixture".into(),
        },
    };
    let mut doc = document();
    doc.entry.label = "Claude configuration".into();
    doc.host = "fixture".into();
    doc.user = "fixture".into();
    doc.host_key = "SHA256:fixture".into();
    doc.path = target.display().to_string();
    doc.content = original.into();
    doc.sha256 = hash;
    doc.mode = 33152;
    doc.metadata_hash = "meta".into();
    let requested = "{\"projects\":{},\"theme\":\"light\"}";
    assert!(matches!(
        remote_config::save_config(
            &transport,
            &draft,
            &doc,
            requested,
            &tmp.path().join("backups")
        ),
        Err(CoreError::RemoteUnconfirmed)
    ));
    assert_eq!(fs::read_to_string(&target).unwrap(), requested);
    assert_eq!(fs::read_to_string(counter).unwrap(), "x");
    assert_eq!(fs::read_dir(tmp.path().join("backups")).unwrap().count(), 1);
}
