"""Minimal W3C WebDriver client (stdlib only) for driving the egui canvas.

egui/eframe on the web draws into a <canvas> and (eframe 0.36) does NOT
implement AccessKit for wasm, so there are no DOM nodes to query. Everything
here is therefore pointer/keyboard actions at viewport coordinates plus
screenshots.
"""
import base64
import json
import time
import urllib.request

KEY_ENTER = ""
KEY_CTRL = ""
KEY_TAB = ""
KEY_BACKSPACE = ""

CHROME_ARGS = [
    # No GPU in the container: software WebGL is required, otherwise eframe
    # panics with "WebGL isn't supported".
    "--use-gl=angle",
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
    "--ignore-gpu-blocklist",
    "--enable-webgl",
]


class Browser:
    def __init__(self, endpoint="http://127.0.0.1:4444", width=900, height=1000):
        self.endpoint = endpoint
        caps = {
            "browserName": "chrome",
            "goog:loggingPrefs": {"browser": "ALL"},
            "goog:chromeOptions": {"args": CHROME_ARGS},
        }
        value = self._call("POST", "/session", {"capabilities": {"alwaysMatch": caps}})
        self.sid = value["sessionId"]
        self._call("POST", f"/session/{self.sid}/window/rect", {"width": width, "height": height})

    def _call(self, method, path, body=None):
        req = urllib.request.Request(
            self.endpoint + path,
            method=method,
            data=None if body is None else json.dumps(body).encode(),
            headers={"Content-Type": "application/json"},
        )
        return json.load(urllib.request.urlopen(req, timeout=120))["value"]

    def s(self, method, path, body=None):
        return self._call(method, f"/session/{self.sid}{path}", body)

    def close(self):
        try:
            self._call("DELETE", f"/session/{self.sid}")
        except Exception:
            pass

    # -- page -------------------------------------------------------------
    def open(self, url, wait=1.0):
        self.s("POST", "/url", {"url": url})
        self.js("return 1")
        time.sleep(wait)

    def js(self, script, *args):
        return self.s("POST", "/execute/sync", {"script": script, "args": list(args)})

    def clear_storage(self):
        self.js("localStorage.clear(); sessionStorage.clear();")

    def wait_for_app(self, timeout=60):
        """The wasm app removes #loading_text once eframe started."""
        end = time.time() + timeout
        while time.time() < end:
            if self.js("return !!document.querySelector('canvas') && !document.getElementById('loading_text')"):
                time.sleep(0.5)
                return
            time.sleep(0.5)
        raise TimeoutError("eframe did not start; console:\n" + "\n".join(self.console()))

    def console(self, level_min="WARNING"):
        order = ["DEBUG", "INFO", "WARNING", "SEVERE"]
        out = []
        try:
            for entry in self.s("POST", "/se/log", {"type": "browser"}):
                if order.index(entry["level"]) >= order.index(level_min):
                    out.append(f"{entry['level']} {entry['message'][:300]}")
        except Exception as err:  # log endpoint is Grid specific
            out.append(f"(no console: {err})")
        return out

    def screenshot_png(self):
        return base64.b64decode(self.s("GET", "/screenshot"))

    def screenshot(self, path):
        with open(path, "wb") as f:
            f.write(self.screenshot_png())
        return path

    def run_before_page_scripts(self, source):
        """Chrome DevTools: run `source` at the start of every new document."""
        self.s("POST", "/goog/cdp/execute", {"cmd": "Page.addScriptToEvaluateOnNewDocument", "params": {"source": source}})

    def fit_viewport(self, width, height):
        """Sets the window so that the *page* viewport is width x height."""
        inner_w, inner_h = self.js("return [innerWidth, innerHeight]")
        outer_w, outer_h = self.js("return [outerWidth, outerHeight]")
        self.s("POST", "/window/rect", {"width": width + outer_w - inner_w, "height": height + outer_h - inner_h})
        return self.js("return [innerWidth, innerHeight]")

    # -- input ------------------------------------------------------------
    def _actions(self, pointer=None, keys=None):
        acts = []
        if pointer is not None:
            acts.append({"type": "pointer", "id": "mouse", "parameters": {"pointerType": "mouse"}, "actions": pointer})
        if keys is not None:
            acts.append({"type": "key", "id": "kb", "actions": keys})
        # No DELETE /actions: releasing resets the pointer to (0, 0).
        self.s("POST", "/actions", {"actions": acts})

    def move(self, x, y):
        self._actions(pointer=[{"type": "pointerMove", "x": int(x), "y": int(y), "origin": "viewport", "duration": 0}])
        time.sleep(0.15)

    def click(self, x, y, settle=0.4):
        self.move(x, y)
        self._actions(
            pointer=[
                {"type": "pointerMove", "x": int(x), "y": int(y), "origin": "viewport", "duration": 0},
                {"type": "pointerDown", "button": 0},
                {"type": "pause", "duration": 80},
                {"type": "pointerUp", "button": 0},
            ]
        )
        time.sleep(settle)

    def type(self, text, settle=0.3):
        keys = []
        for ch in text:
            keys += [{"type": "keyDown", "value": ch}, {"type": "keyUp", "value": ch}]
        self._actions(keys=keys)
        time.sleep(settle)

    def press(self, key, settle=0.3):
        self._actions(keys=[{"type": "keyDown", "value": key}, {"type": "keyUp", "value": key}])
        time.sleep(settle)

    def select_all(self):
        self._actions(
            keys=[
                {"type": "keyDown", "value": KEY_CTRL},
                {"type": "keyDown", "value": "a"},
                {"type": "keyUp", "value": "a"},
                {"type": "keyUp", "value": KEY_CTRL},
            ]
        )
        time.sleep(0.2)
