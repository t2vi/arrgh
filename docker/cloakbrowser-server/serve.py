"""Start CloakBrowser's stealth Chromium with CDP remote debugging.

Chromium 112+ ignores --remote-debugging-address and always binds to 127.0.0.1.
socat proxies 0.0.0.0:PORT -> 127.0.0.1:INTERNAL_PORT so other containers can reach it.
"""
import os
import subprocess
import sys
import time

from cloakbrowser.download import ensure_binary
from cloakbrowser.config import get_default_stealth_args, IGNORE_DEFAULT_ARGS

PORT = int(os.environ.get("CDP_PORT", "3000"))
INTERNAL_PORT = 9222

binary = ensure_binary()
ignore = set(IGNORE_DEFAULT_ARGS)
stealth = [
    a for a in get_default_stealth_args()
    if a not in ignore
    and not a.startswith("--remote-debugging-address")
    and not a.startswith("--remote-debugging-port")
]

print(f"[cloakserve] stealth Chromium on CDP port {PORT} (internal {INTERNAL_PORT})", flush=True)
print(f"[cloakserve] binary: {binary}", flush=True)

chromium = subprocess.Popen([
    binary,
    f"--remote-debugging-port={INTERNAL_PORT}",
    "--remote-allow-origins=*",
    "--headless=new",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-gpu",
    "--no-sandbox",
    "--disable-setuid-sandbox",
    "--disable-dev-shm-usage",
] + stealth)

# Wait for Chromium to bind before nginx starts.
time.sleep(1)

nginx = subprocess.Popen(["nginx", "-g", "daemon off;"])
print(f"[cloakserve] nginx forwarding 0.0.0.0:{PORT} -> 127.0.0.1:{INTERNAL_PORT}", flush=True)

chromium.wait()
nginx.terminate()
sys.exit(chromium.returncode)
