# Fixed read-only operation. It receives known references and returns availability metadata only.
import json
import os
import pwd
import stat
import sys
import time

LIMIT = 1024 * 1024
account = pwd.getpwuid(os.getuid())
home = account.pw_dir

def reply(value):
    print(json.dumps(value, ensure_ascii=True, separators=(",", ":")), flush=True)

def open_dir(path):
    if not path.startswith("/") or "\x00" in path:
        raise ValueError("invalid path")
    fd = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for part in path.split("/"):
            if not part:
                continue
            if part in (".", ".."):
                raise ValueError("invalid path")
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
        result = fd
        fd = -1
        return result
    finally:
        if fd >= 0:
            os.close(fd)

def valid_id(value):
    if not isinstance(value, str) or len(value) != 36 or value[8] != "-" or value[13] != "-" or value[18] != "-" or value[23] != "-":
        return False
    return all(ch in "0123456789abcdefABCDEF-" for ch in value)

def running_claude():
    try:
        for name in os.listdir("/proc"):
            if not name.isdigit():
                continue
            try:
                with open("/proc/" + name + "/comm", "rb") as f:
                    comm = f.read(128).decode("ascii", "ignore").strip().lower()
                if comm in ("claude", "claude-code"):
                    return True
            except (FileNotFoundError, PermissionError, ProcessLookupError):
                continue
    except OSError:
        # If process state cannot be checked, fail closed for import safety.
        return True
    return False

def probe(ref, any_cli_running):
    sid = ref.get("sessionId")
    cwd = ref.get("cwd")
    transcript = ref.get("transcriptPath")
    base = {"sessionId": sid, "cwd": cwd, "transcriptPath": transcript,
            "available": False, "active": False, "reason": "invalidReference"}
    if not valid_id(sid) or not isinstance(cwd, str) or not isinstance(transcript, str):
        return base
    if not cwd.startswith("/") or "\x00" in cwd or any(p in (".", "..") for p in cwd.split("/")):
        return base
    project = cwd.replace("/", "-")
    expected = os.path.join(home, ".claude", "projects", project, sid + ".jsonl")
    if transcript != expected:
        return base
    try:
        cwd_fd = open_dir(cwd)
        os.close(cwd_fd)
        projects_fd = open_dir(os.path.join(home, ".claude", "projects"))
        try:
            project_fd = os.open(project, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=projects_fd)
            try:
                transcript_fd = os.open(sid + ".jsonl", os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW, dir_fd=project_fd)
                try:
                    info = os.fstat(transcript_fd)
                    if not stat.S_ISREG(info.st_mode):
                        return {**base, "reason": "missing"}
                    recent = time.time() - info.st_mtime < 15
                finally:
                    os.close(transcript_fd)
            finally:
                os.close(project_fd)
        finally:
            os.close(projects_fd)
    except FileNotFoundError:
        return {**base, "reason": "missing"}
    except (NotADirectoryError, PermissionError, OSError, ValueError):
        return {**base, "reason": "inaccessibleWorkingDirectory"}
    active = any_cli_running or recent
    return {"sessionId": sid, "cwd": cwd, "transcriptPath": transcript,
            "available": True, "active": active, "reason": "active" if active else None}

reply({"host": __import__("socket").gethostname(), "user": account.pw_name, "home": home})
for line in sys.stdin:
    try:
        if len(line) > LIMIT:
            raise ValueError("limit")
        request = json.loads(line)
        refs = request["references"]
        if request.get("op") != "probe_sessions" or not isinstance(refs, list) or len(refs) > 2048:
            raise ValueError("request")
        busy = running_claude()
        reply({"sessions": [probe(ref, busy) for ref in refs]})
    except (ValueError, KeyError, TypeError):
        reply({"error": "invalidRequest"})
        break
