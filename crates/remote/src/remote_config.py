# Fixed operation used over a single authenticated SSH session. Inputs arrive only as JSON data.
import errno
import hashlib
import json
import os
import pwd
import socket
import stat
import sys
import tempfile

LIMIT = 1024 * 1024
FILES = {
    "claude-settings": (".claude", "settings.json"),
    "claude-home": (".claude.json",),
    "claude-nested": (".claude", ".claude.json"),
    "codex-config": (".codex", "config.toml"),
}
account = pwd.getpwuid(os.getuid())
home = account.pw_dir

def reply(value):
    print(json.dumps(value, ensure_ascii=True, separators=(",", ":")), flush=True)

def identity(st):
    return [st.st_dev, st.st_ino, st.st_mode, st.st_uid, st.st_gid]

def xattrs(fd):
    return {name: os.getxattr(fd, name) for name in os.listxattr(fd)}

def metadata(st, attrs):
    h = hashlib.sha256(repr(identity(st)[2:]).encode())
    for key in sorted(attrs):
        val = attrs[key]
        h.update(len(key.encode()).to_bytes(8, 'big')); h.update(key.encode())
        h.update(len(val).to_bytes(8, 'big')); h.update(val)
    return h.hexdigest()

def parent_for(parts):
    fd = os.open(home, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for component in parts[:-1]:
            child = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd); fd = child
        return fd
    except BaseException:
        os.close(fd)
        raise

def load(parts):
    parent = parent_for(parts)
    try:
        fd = os.open(parts[-1], os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW, dir_fd=parent)
        try:
            st = os.fstat(fd)
            if not stat.S_ISREG(st.st_mode) or st.st_size > LIMIT: raise ValueError('unsupported')
            data = bytearray()
            while len(data) <= LIMIT:
                block = os.read(fd, min(65536, LIMIT + 1 - len(data)))
                if not block: break
                data.extend(block)
            if len(data) > LIMIT: raise ValueError('unsupported')
            text = data.decode('utf-8')
            attrs = xattrs(fd)
            return {"path": os.path.join(home,*parts), "content": text,
                    "sha256": hashlib.sha256(data).hexdigest(), "device": st.st_dev,
                    "inode": st.st_ino, "mode": st.st_mode, "uid": st.st_uid, "gid": st.st_gid,
                    "metadata_hash": metadata(st, attrs)}
        finally: os.close(fd)
    finally: os.close(parent)

def save(parts, expected, content):
    data = content.encode('utf-8')
    if len(data) > LIMIT: raise ValueError('unsupported')
    parent = parent_for(parts)
    tmpname = None
    try:
        source = os.open(parts[-1], os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW, dir_fd=parent)
        try:
            st = os.fstat(source)
            if not stat.S_ISREG(st.st_mode): raise ValueError('unsupported')
            current = load(parts)
            if any(current[k] != expected[k] for k in ('path','sha256','device','inode','mode','uid','gid','metadata_hash')): return {"error":"conflict"}
            if identity(st) != [current['device'], current['inode'], current['mode'], current['uid'], current['gid']]: return {"error":"conflict"}
            attrs = xattrs(source)
            # Random name in same directory; O_EXCL and O_NOFOLLOW prevent following links.
            for _ in range(32):
                name = '.agents-manager-' + os.urandom(16).hex()
                try: new = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=parent); tmpname = name; break
                except FileExistsError: continue
            else: raise OSError('temporary file unavailable')
            try:
                os.fchown(new, st.st_uid, st.st_gid)
                os.fchmod(new, stat.S_IMODE(st.st_mode))
                for name, value in attrs.items(): os.setxattr(new, name, value)
                remaining = data
                while remaining: remaining = remaining[os.write(new, remaining):]
                os.fsync(new)
                fresh = os.fstat(new)
                if (stat.S_IMODE(fresh.st_mode),fresh.st_uid,fresh.st_gid)!=(stat.S_IMODE(st.st_mode),st.st_uid,st.st_gid) or xattrs(new)!=attrs: raise OSError('metadata cannot be preserved')
            finally: os.close(new)
            # Reopen ancestors and target immediately before rename. Noncooperating writers can
            # still race after this check; rename is atomic but is not compare-and-swap.
            checkparent = parent_for(parts)
            try:
                if identity(os.fstat(checkparent))[:2] != identity(os.fstat(parent))[:2]: return {"error":"conflict"}
                check = load(parts)
                if any(check[k] != expected[k] for k in ('path','sha256','device','inode','mode','uid','gid','metadata_hash')): return {"error":"conflict"}
            finally: os.close(checkparent)
            os.rename(tmpname,parts[-1],src_dir_fd=parent,dst_dir_fd=parent); tmpname=None
            try: os.fsync(parent)
            except OSError: return {"error":"unconfirmed"}
            return {"sha256":hashlib.sha256(data).hexdigest()}
        finally: os.close(source)
    finally:
        if tmpname is not None:
            try: os.unlink(tmpname,dir_fd=parent)
            except FileNotFoundError: pass
        os.close(parent)

reply({"host":socket.gethostname(),"user":account.pw_name,"home":home})
for line in sys.stdin:
    try:
        if len(line) > 8 * LIMIT: raise ValueError('unsupported')
        request=json.loads(line)
        parts=FILES[request['id']]
        if request['op']=='read': reply(load(parts))
        elif request['op']=='save': reply(save(parts,request['expected'],request['content']))
        else: raise ValueError('unsupported')
    except FileNotFoundError: reply({"error":"missing"})
    except PermissionError: reply({"error":"permission"})
    except UnicodeError: reply({"error":"encoding"})
    except OSError as e:
        if e.errno in (errno.ELOOP,errno.ENOTDIR): reply({"error":"disallowed"})
        else: reply({"error":"io"})
    except (ValueError,KeyError,TypeError): reply({"error":"unsupported"})
