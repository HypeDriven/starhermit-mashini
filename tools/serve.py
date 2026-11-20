"""Serve the packaged game locally with the MIME types used by StarHermit."""
import json
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
class GameFiles(SimpleHTTPRequestHandler):
    extensions_map = {**SimpleHTTPRequestHandler.extensions_map,
                      ".wasm": "application/wasm", ".opus": "audio/ogg"}

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=ROOT, **kwargs)

    def send_head(self):
        name = urlsplit(self.path).path.lstrip("/") or "index.html"
        files = json.loads((ROOT / "starhermit-files.json").read_text())["files"]
        if name not in {"index.html", "starhermit-files.json"} and name not in files:
            self.send_error(404)
            return None
        self.path = "/" + name
        return super().send_head()

if __name__ == "__main__":
    address = ("127.0.0.1", 8000)
    print("Mashini is available at http://localhost:8000", flush=True)
    ThreadingHTTPServer(address, GameFiles).serve_forever()
