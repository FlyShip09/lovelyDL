"""A slow local media server for live tests: every path serves the same file at a capped
speed, with Range support so paused downloads can resume. Each request is logged to stdout.

    python scripts/test-server.py <file> [port] [bytes-per-second]
"""

import http.server
import socketserver
import sys
import time

DATA = open(sys.argv[1], "rb").read()
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 18766
RATE = int(sys.argv[3]) if len(sys.argv) > 3 else 1_500_000


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def do_HEAD(self):
        self.serve(False)

    def do_GET(self):
        self.serve(True)

    def serve(self, body):
        start, end = 0, len(DATA) - 1
        rng = self.headers.get("Range")
        if rng and rng.startswith("bytes="):
            a, _, b = rng[6:].partition("-")
            start, end = int(a or 0), int(b) if b else end
            self.send_response(206)
            self.send_header("Content-Range", f"bytes {start}-{end}/{len(DATA)}")
        else:
            self.send_response(200)
        print(f"{self.command} {self.path} range={rng}", flush=True)
        self.send_header("Content-Type", "video/mp4")
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Length", str(end - start + 1))
        self.end_headers()
        if not body:
            return
        chunk = 64 * 1024
        try:
            for i in range(start, end + 1, chunk):
                self.wfile.write(DATA[i : min(i + chunk, end + 1)])
                time.sleep(chunk / RATE)
        except OSError:
            pass


class Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True


Server(("127.0.0.1", PORT), Handler).serve_forever()
