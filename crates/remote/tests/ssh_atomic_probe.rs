#![allow(clippy::unwrap_used)]
//! Isolated research probe for SPEC-0005, not an implementation of remote editing.
#[cfg(unix)]
#[test]
fn ssh_atomic_replace_revalidates_but_exposes_postcheck_race() {
    use std::{
        fs,
        io::{BufRead, BufReader, Write},
        net::{TcpListener, TcpStream},
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::Path,
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
    fn key(path: &Path) {
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
    fn shell_quote(text: &str) -> String {
        format!("'{}'", text.replace('\'', "'\\''"))
    }
    let temp = tempdir().unwrap();
    let root = temp.path();
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
    let sshd = std::env::var_os("AGENTS_MANAGER_SSHD")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/usr/sbin/sshd".into());
    assert!(
        sshd.is_file(),
        "Set AGENTS_MANAGER_SSHD or install openssh-server"
    );
    let mut helpers = String::new();
    if let Some(base) = sshd.parent().and_then(Path::parent) {
        let session = base.join("lib/openssh/sshd-session");
        let auth = base.join("lib/openssh/sshd-auth");
        if session.exists() && auth.exists() {
            helpers = format!(
                "SshdSessionPath {}\nSshdAuthPath {}\n",
                session.display(),
                auth.display()
            );
        }
    }
    let config = root.join("sshd_config");
    fs::write(&config,format!("Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nStrictModes no\nAllowUsers {user}\nSetEnv HOME={}\n{helpers}",host.display(),root.join("pid").display(),root.join("authorized_keys").display(),root.display())).unwrap();
    let child = Command::new(&sshd)
        .args(["-D", "-e", "-f"])
        .arg(config)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut server = Guard(child);
    let start = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(server.0.try_wait().unwrap().is_none(), "sshd exited");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "sshd failed to start"
        );
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
    let client_config = root.join("ssh_config");
    fs::write(&client_config,format!("Host probe\n HostName 127.0.0.1\n User {user}\n Port {port}\n IdentityFile {}\n IdentitiesOnly yes\n IdentityAgent none\n UserKnownHostsFile {}\n GlobalKnownHostsFile /dev/null\n StrictHostKeyChecking yes\n BatchMode yes\n ForwardAgent no\n",client.display(),known.display())).unwrap();
    let script = r#"
import hashlib, os, stat, sys
parent = sys.stdin.readline().rstrip('\n')
dirfd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
temp = '.probe-' + str(os.getpid())
def digest(fd):
    os.lseek(fd, 0, os.SEEK_SET)
    h = hashlib.sha256()
    while True:
        b = os.read(fd, 65536)
        if not b: return h.digest()
        h.update(b)
def attrs(fd):
    return {name: os.getxattr(fd, name) for name in os.listxattr(fd)}
def identity(st):
    return (st.st_dev, st.st_ino, st.st_mode, st.st_uid, st.st_gid)
try:
    original = os.open('config.json', os.O_RDONLY | os.O_NOFOLLOW, dir_fd=dirfd)
    st = os.fstat(original)
    if not stat.S_ISREG(st.st_mode): raise RuntimeError('not regular')
    expected, extended = digest(original), attrs(original)
    new = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=dirfd)
    try:
        os.fchown(new, st.st_uid, st.st_gid)
        os.fchmod(new, stat.S_IMODE(st.st_mode))
        for name, value in extended.items(): os.setxattr(new, name, value)
        replacement = b'{"draft":true}\n'
        while replacement: replacement = replacement[os.write(new, replacement):]
        os.fsync(new)
        proposed = os.fstat(new)
        if (stat.S_IMODE(proposed.st_mode), proposed.st_uid, proposed.st_gid) != (stat.S_IMODE(st.st_mode), st.st_uid, st.st_gid) or attrs(new) != extended:
            raise RuntimeError('metadata cannot be preserved')
    finally: os.close(new)
    print('BEFORE_CHECK', flush=True)
    sys.stdin.readline()
    try:
        current = os.open('config.json', os.O_RDONLY | os.O_NOFOLLOW, dir_fd=dirfd)
        try:
            same = identity(os.fstat(current)) == identity(st) and digest(current) == expected and attrs(current) == extended
        finally: os.close(current)
    except OSError: same = False
    if not same:
        print('CONFLICT', flush=True)
    else:
        print('AFTER_CHECK', flush=True)
        sys.stdin.readline()
        os.rename(temp, 'config.json', src_dir_fd=dirfd, dst_dir_fd=dirfd)
        os.fsync(dirfd)
        print('SAVED', flush=True)
finally:
    try: os.unlink(temp, dir_fd=dirfd)
    except FileNotFoundError: pass
    os.close(dirfd)
"#;
    let remote = format!("/usr/bin/python3 -u -c {}", shell_quote(script));
    let target_dir = root.join("synthetic");
    fs::create_dir(&target_dir).unwrap();
    let target = target_dir.join("config.json");
    let set_xattr = |path: &Path| {
        let output = Command::new("/usr/bin/python3")
            .args([
                "-c",
                "import os,sys; os.setxattr(sys.argv[1], 'user.fixture', b'private')",
            ])
            .arg(path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "fixture filesystem must support user extended attributes"
        );
    };
    let read_xattr = |path: &Path| {
        let out = Command::new("/usr/bin/python3")
            .args([
                "-c",
                "import os,sys; sys.stdout.buffer.write(os.getxattr(sys.argv[1], 'user.fixture'))",
            ])
            .arg(path)
            .output()
            .unwrap();
        assert!(out.status.success());
        out.stdout
    };
    for scenario in [
        "success",
        "conflict_before_check",
        "replacement_before_check",
        "write_after_check",
    ] {
        fs::write(&target, b"{\"original\":true}\n").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
        set_xattr(&target);
        let before = fs::metadata(&target).unwrap();
        let mut child = Command::new("ssh")
            .arg("-F")
            .arg(&client_config)
            .args(["-T", "probe"])
            .arg(&remote)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        writeln!(input, "{}", target_dir.display()).unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        assert_eq!(line, "BEFORE_CHECK\n");
        if scenario == "conflict_before_check" {
            fs::write(&target, b"{\"external\":1}\n").unwrap();
        }
        if scenario == "replacement_before_check" {
            fs::remove_file(&target).unwrap();
            fs::write(&target, b"{\"original\":true}\n").unwrap();
            fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
            set_xattr(&target);
        }
        writeln!(input, "continue").unwrap();
        input.flush().unwrap();
        line.clear();
        output.read_line(&mut line).unwrap();
        if scenario == "conflict_before_check" || scenario == "replacement_before_check" {
            assert_eq!(line, "CONFLICT\n");
            let expected = if scenario == "conflict_before_check" {
                b"{\"external\":1}\n".as_slice()
            } else {
                b"{\"original\":true}\n".as_slice()
            };
            assert_eq!(fs::read(&target).unwrap(), expected);
        } else {
            assert_eq!(line, "AFTER_CHECK\n");
            if scenario == "write_after_check" {
                fs::write(&target, b"{\"external\":2}\n").unwrap();
            }
            writeln!(input, "continue").unwrap();
            input.flush().unwrap();
            line.clear();
            output.read_line(&mut line).unwrap();
            assert_eq!(line, "SAVED\n");
            assert_eq!(fs::read(&target).unwrap(), b"{\"draft\":true}\n");
            let after = fs::metadata(&target).unwrap();
            assert_ne!(before.ino(), after.ino());
            assert_eq!(before.mode() & 0o7777, after.mode() & 0o7777);
            assert_eq!((before.uid(), before.gid()), (after.uid(), after.gid()));
            assert_eq!(read_xattr(&target), b"private");
        }
        drop(input);
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(fs::read_dir(&target_dir).unwrap().count(), 1);
    }
}
