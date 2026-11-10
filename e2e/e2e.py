"""End-to-end checks of the Prono web app in a real browser (Selenium Chromium
in podman), against a real backend and a dev MariaDB.

Run through `e2e/run.sh`, which starts the browser, the static web server and
exports the environment this script needs. Needs Pillow for pixel checks.

The app is an egui canvas without accessibility nodes on the web, so the test
clicks at fixed viewport coordinates, types with the WebDriver actions API,
and asserts on (a) the database, (b) the REST traffic the browser saw (a
`fetch` recorder injected into every page) and (c) pixels of screenshots
(error banner = reddish pixels, timeline points = green discs).
"""
import io
import json
import os
import subprocess
import sys
import tempfile
import time
import traceback
import urllib.error
import urllib.request

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wd import KEY_ENTER, Browser  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.environ.get("E2E_SRC", ROOT)  # the workspace that was built (run.sh may build a clean `git archive HEAD`)
OUT = os.environ.get("E2E_OUT", os.path.join(ROOT, "target", "e2e-shots"))
WEB = os.environ.get("E2E_WEB_URL", "http://localhost:8080/index.html#dev")
BACKEND = os.environ.get("E2E_BACKEND_URL", "http://localhost:8081")
BACKEND_BIN = os.environ.get("E2E_BACKEND_BIN", os.path.join(SRC, "target", "release", "prono-backend"))
WEBDRIVER = os.environ.get("E2E_WEBDRIVER", "http://127.0.0.1:4444")
DB_CONTAINER = os.environ.get("E2E_DB_CONTAINER", "prono-dev-db")
DB_ROOT_PASS = os.environ.get("E2E_DB_ROOT_PASS", "devroot")
DB_ENV = {
    "PRONO_DB_HOST": os.environ.get("PRONO_DB_HOST", "127.0.0.1"),
    "PRONO_DB_PORT": os.environ.get("PRONO_DB_PORT", "13306"),
    "PRONO_DB_USER": os.environ.get("PRONO_DB_USER", "prono-app"),
    "PRONO_DB_PASS": os.environ.get("PRONO_DB_PASS", "devpass"),
}
API_KEY = None  # read from the app source on demand, see api_key()

# ---- layout of the egui UI (CSS px, scale 1) ---------------------------------
NAME_FIELD = (220, 64)
SURVEY_BUTTON = (120, 112)  # the only survey of the catalog
SURVEY_BUTTON_BELOW_WARNING = (120, 130)  # the "name already filled in" line pushes it down
ROW_Y = [110, 131, 152, 173]  # the four date questions
MONTH_X, YEAR_X = 215, 300
RESET, SUBMIT = (28, 194), (81, 194)
CHANGE_NAME_BELOW_BANNER = (157, 227)  # the error banner pushes everything down by 33 px
CHOOSE_ANOTHER = (78, 128)
VIEW = (892, 957)

os.makedirs(OUT, exist_ok=True)


# ---- tiny test framework --------------------------------------------------------
RESULTS = []  # (feature, status, detail)


class Feature:
    """`with Feature("name"):` records PASS, FAIL (assertion/exception) or the
    status set through `f.note(...)`/`f.not_verifiable(...)`."""

    def __init__(self, name):
        self.name, self.status, self.details = name, "PASS", []

    def note(self, text):
        self.details.append(text)

    def not_verifiable(self, text):
        self.status = "N/A"
        self.details.append(text)

    def fail(self, text):
        self.status = "FAIL"
        self.details.append(text)

    def check(self, cond, text):
        if cond:
            self.details.append("ok: " + text)
        else:
            self.fail("CHECK FAILED: " + text)
        return cond

    def __enter__(self):
        print(f"\n== {self.name}", flush=True)
        return self

    def __exit__(self, exc_type, exc, tb):
        if exc is not None:
            self.status = "FAIL"
            self.details.append("EXCEPTION: " + "".join(traceback.format_exception_only(exc_type, exc)).strip())
            traceback.print_exc()
        RESULTS.append((self.name, self.status, self.details))
        print(f"-- {self.status}: {self.name}")
        for d in self.details:
            print("   " + d)
        return True


# ---- database -------------------------------------------------------------------
def sql(query):
    out = subprocess.run(
        ["podman", "exec", DB_CONTAINER, "mariadb", "-uroot", f"-p{DB_ROOT_PASS}", "db_prono", "-N", "-B", "-e", query],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return [line.split("\t") for line in out.splitlines()]


def reset_db():
    sql("DELETE FROM Users")
    assert sql("SELECT COUNT(*) FROM Users") == [["0"]]
    assert sql("SELECT COUNT(*) FROM AnswerResponse") == [["0"]]


def user_answers(name):
    """Answers of `name` in question order of the survey: ['00/03/2027', ...]"""
    # `question_id` is a BINARY(16): format its hex as the hyphenated UUID the catalog uses.
    uuid = "LOWER(INSERT(INSERT(INSERT(INSERT(HEX(a.question_id), 9, 0, '-'), 14, 0, '-'), 19, 0, '-'), 24, 0, '-'))"
    rows = sql(
        f"SELECT {uuid}, a.answer FROM AnswerResponse a JOIN Users u USING (user_id) "
        f"WHERE u.user_name = '{name}'"
    )
    by_id = dict(rows)
    return [by_id.get(q) for q in question_ids()]


# ---- backend process --------------------------------------------------------------
_backend = None
_empty_cfg = tempfile.mkdtemp(prefix="prono-e2e-cfg-")


def start_backend():
    """Starts the backend with env-only DB config. XDG_CONFIG_HOME points to an
    empty dir so the real `secure_config.toml` is never read."""
    global _backend
    env = {"PATH": os.environ["PATH"], "XDG_CONFIG_HOME": _empty_cfg, "RUST_LOG": "info", **DB_ENV}
    log = open(os.path.join(OUT, "backend.log"), "ab")
    _backend = subprocess.Popen([BACKEND_BIN], env=env, stdout=log, stderr=log, start_new_session=True)
    for _ in range(100):
        try:
            rest("GET", "/api/surveys/1/answers")
            return
        except OSError:
            time.sleep(0.2)
    raise RuntimeError("backend did not start, see " + os.path.join(OUT, "backend.log"))


def stop_backend():
    global _backend
    if _backend and _backend.poll() is None:
        _backend.terminate()
        _backend.wait(10)
    _backend = None


# ---- REST ------------------------------------------------------------------------------
def rest(method, path, body=None, headers=None):
    req = urllib.request.Request(
        BACKEND + path,
        method=method,
        data=None if body is None else json.dumps(body).encode(),
        headers={"Content-Type": "application/json", **(headers or {})},
    )
    try:
        resp = urllib.request.urlopen(req, timeout=10)
    except urllib.error.HTTPError as err:
        resp = err
    return resp.status, resp.read().decode(), dict(resp.headers)


def api_key():
    global API_KEY
    if API_KEY is None:
        import re

        text = open(os.path.join(SRC, "api", "src", "lib.rs")).read()
        API_KEY = re.search(r'API_KEY: &str = "([^"]+)"', text).group(1)
    return API_KEY


def question_ids():
    """Question ids in survey order: UUIDv5(DNS, text + kind), see api/src/catalog.rs."""
    import uuid

    with open(os.path.join(SRC, "api", "surveys", "spacex_starship.json")) as f:
        survey = json.load(f)
    assert survey["survey_id"] == 1
    kinds = {"prediction_date": "PredictionDate", "text": "Text"}
    return [str(uuid.uuid5(uuid.NAMESPACE_DNS, q["question"] + kinds[q["answer_type"]])) for q in survey["questions"]]


# ---- browser helpers ----------------------------------------------------------------------
FETCH_RECORDER = """
(() => {
  window.__net = [];
  const orig = window.fetch;
  window.fetch = function (req, ...rest) {
    const method = (req && req.method) || 'GET';
    const url = (req && req.url) || String(req);
    return orig.call(this, req, ...rest).then(
      (r) => { r.clone().text().then((t) => window.__net.push({method, url, status: r.status, body: t.slice(0, 300)})); return r; },
      (e) => { window.__net.push({method, url, error: String(e)}); throw e; });
  };
})();
"""


class Page:
    """One participant = one fresh browser session (fresh localStorage, so a new device id)."""

    def __init__(self, label):
        self.label = label
        self.b = Browser(WEBDRIVER, 900, 1100)
        self.b.run_before_page_scripts(FETCH_RECORDER)
        for _ in range(5):  # a new window sometimes reports its size late
            if self.b.fit_viewport(*VIEW) == list(VIEW):
                break
            time.sleep(0.5)
        else:
            raise AssertionError("viewport not " + str(VIEW))
        self.shots = 0

    def open(self):
        self.b.open(WEB, wait=0.2)
        self.b.wait_for_app()
        return self

    def reload(self, settle=2.0):
        self.b.s("POST", "/refresh", {})
        self.b.wait_for_app()
        time.sleep(settle)
        return self

    def close(self):
        self.b.close()

    # -- input
    def set_name(self, name):
        self.b.click(*NAME_FIELD)
        self.b.select_all()
        self.b.type(name)

    def start_survey(self, name):
        self.set_name(name)
        # Give the name check time to answer: its warning moves the survey button down.
        time.sleep(1.0)
        data = self.b.screenshot_png()
        button = SURVEY_BUTTON_BELOW_WARNING if name_warning_shown(Image.open(io.BytesIO(data)).convert("RGB")) else SURVEY_BUTTON
        self.b.click(*button, settle=0.8)

    def set_dv(self, x, y, value):
        self.b.click(x, y)
        self.b.select_all()
        self.b.type(str(value))
        self.b.press(KEY_ENTER)

    def fill(self, values):
        for i, (month, year) in enumerate(values):
            self.set_dv(MONTH_X, ROW_Y[i], month)
            self.set_dv(YEAR_X, ROW_Y[i], year)

    def submit(self, settle=2.0):
        self.b.click(*SUBMIT, settle=settle)

    # -- observation
    def shot(self, name):
        data = self.b.screenshot_png()
        with open(os.path.join(OUT, f"{self.label}-{name}.png"), "wb") as f:
            f.write(data)
        return Image.open(io.BytesIO(data)).convert("RGB")

    def net(self):
        """REST calls to the backend seen by the browser (the wasm download is filtered out)."""
        return [e for e in self.b.js("return window.__net || []") if ":8081" in e["url"]]

    def wait_net(self, pred, timeout=10):
        end = time.time() + timeout
        while time.time() < end:
            hits = [e for e in self.net() if pred(e)]
            if hits:
                return hits[-1]
            time.sleep(0.3)
        return None

    def severe_console(self):
        return [m for m in self.b.console("SEVERE")]

    def stored_device_id(self):
        return self.b.js("return localStorage.getItem('prono_device_id')")


def banner_box(img):
    """Bounding box of the reddish error banner (fill and red text), or None."""
    px = img.load()
    xs, ys = [], []
    red_text = False  # the banner's text is pure red; the orange name warning is not a banner
    for y in range(35, 150):
        for x in range(0, img.width):
            r, g, b = px[x, y]
            if r > 200 and r - g >= 15 and r - b >= 15:
                xs.append(x)
                ys.append(y)
                red_text = red_text or (g < 60 and b < 60)
    return (min(xs), min(ys), max(xs), max(ys)) if red_text else None


def name_warning_shown(img):
    """Whether the orange "already filled in a survey" line is on the start screen."""
    px = img.load()
    return any(px[x, y][0] > 230 and 110 < px[x, y][1] < 180 and px[x, y][2] < 60
               for y in range(75, 95) for x in range(0, 500))


def timeline_points(img, y_min=0):
    """Number of separate green discs (timeline points)."""
    px = img.load()
    w, h = img.size
    seen, count = set(), 0
    for y in range(y_min, h):
        for x in range(w):
            if (x, y) in seen:
                continue
            r, g, b = px[x, y]
            if not (70 < r < 140 and g > 220 and b < 60):
                continue
            stack, size = [(x, y)], 0
            seen.add((x, y))
            while stack:
                cx, cy = stack.pop()
                size += 1
                for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
                    if 0 <= nx < w and 0 <= ny < h and (nx, ny) not in seen:
                        r2, g2, b2 = px[nx, ny]
                        if 70 < r2 < 140 and g2 > 220 and b2 < 60:
                            seen.add((nx, ny))
                            stack.append((nx, ny))
            if size >= 4:
                count += 1
    return count


def x_clusters(img):
    """Number of horizontally separated groups of timeline points: 1 when all
    four default dates are stacked on one spot, more when the dates differ."""
    return len(point_columns(img))


def point_columns(img):
    """Left edge (x) of each horizontally separated group of timeline points."""
    px = img.load()
    cols = sorted({x for x in range(img.width) for y in range(0, img.height, 1)
                   if 70 < px[x, y][0] < 140 and px[x, y][1] > 220 and px[x, y][2] < 60})
    groups = []
    last = -100
    for x in cols:
        if x - last > 10:
            groups.append(x)
        last = x
    return groups


def non_background(img):
    px = img.load()
    bg = px[600, 300]
    return sum(1 for y in range(0, img.height, 3) for x in range(0, img.width, 3) if px[x, y] != bg)


# ---- the scenario -------------------------------------------------------------------------------
ALICE = [(3, 2027), (7, 2030), (1, 2035), (12, 2041)]
ALICE_DB = ["00/03/2027", "00/07/2030", "00/01/2035", "00/12/2041"]
BOB = [(99, 3000), (0, 1999), (2, 2036), (6, 2045)]  # first two out of range: clamped
BOB_DB = ["00/12/2100", "00/01/2024", "00/02/2036", "00/06/2045"]


def main():
    pages = []

    def page(label):
        p = Page(label)
        pages.append(p)
        return p.open()

    try:
        reset_db()
        start_backend()

        # ---------------------------------------------------------------- start screen
        with Feature("1. start screen: name field + survey list from the catalog") as f:
            alice = page("alice")
            img = alice.shot("01-start")
            f.check(non_background(img) > 500, "canvas is rendered (not blank)")
            f.check(banner_box(img) is None, "no error banner at start")
            f.check(alice.severe_console() == [], "no SEVERE console messages")
            f.note("screenshot alice-01-start.png: heading, 'Your name:', 'SpaceX Starship achievements survey' button")

        # ---------------------------------------------------------------- name validation
        with Feature("2. empty name is refused with an error banner") as f:
            alice.b.click(*SURVEY_BUTTON, settle=0.8)
            img = alice.shot("02-empty-name")
            f.check(banner_box(img) is not None, "error banner shown")
            f.check(x_clusters(img) == 0, "still on the start screen (no timeline)")
            f.check(alice.net() == [], "no request sent")
            # dismiss with the x button at the right end of the banner
            x0, y0, x1, y1 = banner_box(img)
            alice.b.click(x1 - 10, (y0 + y1) // 2, settle=0.6)
            img2 = alice.shot("02b-dismissed")
            f.check(banner_box(img2) is None, "banner disappears after clicking the dismiss button")

        # ---------------------------------------------------------------- answering
        with Feature("3. start survey shows questions, date inputs and own timeline") as f:
            alice.start_survey("Alice")
            img = alice.shot("03-answering")
            f.check(banner_box(img) is None, "no error")
            f.check(x_clusters(img) == 1, "timeline shows the 4 default dates stacked on one spot")
            f.note("screenshot alice-03-answering.png: 4 questions with 'month N' / 'year N', Reset, Submit")

        with Feature("4. month/year drag values: typing, range clamping, timeline update") as f:
            alice.fill(ALICE)
            img = alice.shot("04-filled")
            alice_filled = point_columns(img)
            f.check(x_clusters(img) >= 3, f"timeline shows distinct dates ({x_clusters(img)} point groups)")

        with Feature("5. Reset restores the default answers") as f:
            alice.b.click(*RESET, settle=0.6)
            img = alice.shot("05-reset")
            f.check(x_clusters(img) == 1, "timeline back to the stacked defaults")
            alice.fill(ALICE)

        # ---------------------------------------------------------------- submit
        with Feature("6. submit stores one row per answer in the database") as f:
            alice.submit()
            post = alice.wait_net(lambda e: e["method"] == "POST")
            f.check(post and post.get("status") == 200, f"browser saw POST -> {post and post.get('status')}")
            device = alice.stored_device_id()
            f.check(sql("SELECT user_name, device_id FROM Users") == [["Alice", device]], "Users row with the browser's device id")
            f.check(user_answers("Alice") == ALICE_DB, f"AnswerResponse rows {user_answers('Alice')}")
            img = alice.shot("06-done")
            f.check(banner_box(img) is None, "no error after submit")

        with Feature("7. results view shows everyone's answers and the time graph per question") as f:
            f.check(timeline_points(img) == 4, f"Alice's Done screen: 4 points (one per question) = {timeline_points(img)}")
            get = alice.wait_net(lambda e: e["method"] == "GET" and e.get("status") == 200)
            f.check(get is not None, "browser fetched the results (GET 200)")
            status, body, _ = rest("GET", "/api/surveys/1/answers")
            res = json.loads(body)
            f.check(
                [res[q][0]["user"] for q in question_ids()] == ["Alice"] * 4,
                "REST GET returns Alice for every question",
            )
            # second participant on another device, with out-of-range input
            bob = page("bob")
            bob.start_survey("Bob")
            bob.fill(BOB)
            img_b = bob.shot("07a-bob-filled")
            bob.submit()
            f.check(user_answers("Bob") == BOB_DB, f"Bob's out-of-range input clamped in DB: {user_answers('Bob')}")
            img_b = bob.shot("07b-bob-done")
            f.check(timeline_points(img_b) == 8, f"Bob's Done screen: 8 points (2 users x 4 questions) = {timeline_points(img_b)}")
            alice.reload()
            img_a = alice.shot("07c-alice-after-reload")
            f.check(timeline_points(img_a) == 8, f"Alice's reloaded results show both users: {timeline_points(img_a)} points")

        with Feature("8. state survives a page reload (Answering and Done)") as f:
            f.check(timeline_points(img_a) == 8, "Done phase restored after reload (results shown again)")
            carol = page("carol")
            carol.start_survey("Carol")
            carol.fill([(4, 2029), (8, 2032), (10, 2037), (2, 2044)])
            before = x_clusters(carol.shot("08a-carol-before"))
            carol.reload(settle=1.5)
            after_img = carol.shot("08b-carol-after-reload")
            f.check(
                before >= 3 and x_clusters(after_img) == before,
                f"Answering phase and answers restored ({before} -> {x_clusters(after_img)} point groups)",
            )

        with Feature("9. same user logs in again: survey refilled with the earlier answers") as f:
            alice.b.click(*CHOOSE_ANOTHER, settle=0.6)
            alice.start_survey("Alice")
            img = alice.shot("09-alice-again")
            cols = point_columns(img)
            f.check(len(cols) >= 3, f"survey not restarted with the stacked defaults ({len(cols)} point groups)")
            f.check(
                len(cols) == len(alice_filled) and all(abs(a - b) <= 1 for a, b in zip(cols, alice_filled)),
                f"timeline points equal those of the stored answers: {cols} vs {alice_filled}",
            )
            f.check(banner_box(img) is None, "no error banner")
            # a name that never answered starts with the defaults
            fresh = Page("fresh").open()  # not in `pages`: closed at once, Selenium has few sessions
            fresh.start_survey("Fresh")
            f.check(x_clusters(fresh.shot("09b-new-name")) == 1, "a new name starts with the stacked defaults")
            fresh.close()

        # ---------------------------------------------------------------- duplicates
        with Feature("10. second submission of the same user is refused (409), DB unchanged") as f:
            alice.submit()
            conflict = alice.wait_net(lambda e: e["method"] == "POST" and e.get("status") == 409)
            f.check(conflict is not None, f"browser saw {conflict}")
            img = alice.shot("10-duplicate")
            f.check(banner_box(img) is not None, "error banner shown")
            f.check(user_answers("Alice") == ALICE_DB, "Alice's stored answers unchanged")
            # Alice's name from another device
            alice2 = page("alice-other-device")
            alice2.start_survey("Alice")
            alice2.submit()
            r = alice2.wait_net(lambda e: e["method"] == "POST")
            f.check(r and r.get("status") == 403, f"same name, other device -> {r and r.get('status')} {r and r.get('body')}")
            f.check(banner_box(alice2.shot("10b-name-other-device")) is not None, "banner shown")

        with Feature("11. one device cannot submit for a second name (403)") as f:
            # Bob's device (already submitted as Bob) tries as "Mallory"
            bob.b.click(*CHOOSE_ANOTHER, settle=0.6)
            bob.start_survey("Mallory")
            bob.submit()
            r = bob.wait_net(lambda e: e["method"] == "POST" and e.get("status") == 403)
            f.check(r is not None, f"browser saw {r}")
            f.check(banner_box(bob.shot("11-device-in-use")) is not None, "banner shown")
            f.check(sql("SELECT COUNT(*) FROM Users WHERE user_name='Mallory'") == [["0"]], "no Mallory in DB")

        # ---------------------------------------------------------------- REST contract
        with Feature("12. REST contract: API key, device header, CORS") as f:
            qids = question_ids()
            ans = [[q, {"PredictionDate": {"day": None, "month": 5, "year": 2030}}] for q in qids]
            body = {"user": "Eve", "answers": ans}
            good = {"Authorization": "Bearer " + api_key(), "X-Device-Id": "curl-dev"}
            f.check(rest("POST", "/api/surveys/1/answers", body, {"X-Device-Id": "curl-dev"})[0] == 401, "no key -> 401")
            f.check(
                rest("POST", "/api/surveys/1/answers", body, {"Authorization": "Bearer nope", "X-Device-Id": "curl-dev"})[0] == 401,
                "wrong key -> 401",
            )
            f.check(rest("POST", "/api/surveys/1/answers", body, {"Authorization": "Bearer " + api_key()})[0] == 400, "no device header -> 400")
            f.check(rest("POST", "/api/surveys/99/answers", body, good)[0] == 400, "unknown survey -> 400")
            bad = {"user": "Eve", "answers": ans[:2]}
            f.check(rest("POST", "/api/surveys/1/answers", bad, good)[0] == 400, "missing answers -> 400")
            bad = {"user": "Eve", "answers": [[q, {"Text": "x"}] for q in qids]}
            f.check(rest("POST", "/api/surveys/1/answers", bad, good)[0] == 400, "wrong answer kind -> 400")
            f.check(sql("SELECT COUNT(*) FROM Users WHERE user_name='Eve'") == [["0"]], "invalid submissions stored nothing")
            st, _, h = rest("OPTIONS", "/api/surveys/1/answers", None, {
                "Origin": "http://localhost:8080",
                "Access-Control-Request-Method": "POST",
                "Access-Control-Request-Headers": "authorization,content-type,x-device-id",
            })
            f.check(st == 200 and h.get("access-control-allow-origin") == "http://localhost:8080", f"CORS preflight {st}")
            f.check(rest("POST", "/api/surveys/1/answers", body, good)[0] == 200, "valid POST -> 200")
            f.check(rest("POST", "/api/surveys/1/answers", body, good)[0] == 409, "repeat POST -> 409")
            sql("DELETE FROM Users WHERE user_name='Eve'")

        # ---------------------------------------------------------------- text answers
        with Feature("13. text answers") as f:
            f.not_verifiable("the only survey in the catalog (api/surveys/spacex_starship.json) has date questions only; "
                             "no text input exists in the UI to drive (covered by Rust unit tests)")

        # ---------------------------------------------------------------- backend down
        with Feature("14. errors are shown when the backend is down, and the app recovers") as f:
            stop_backend()
            dave = page("dave")
            dave.start_survey("Dave")
            dave.fill([(1, 2026), (2, 2027), (3, 2028), (4, 2029)])
            dave.submit(settle=3.0)
            err = dave.wait_net(lambda e: e["method"] == "POST" and "error" in e)
            f.check(err is not None, f"browser saw the failed POST: {err and err['error']}")
            img = dave.shot("14a-submit-backend-down")
            box = banner_box(img)
            f.check(box is not None, "error banner shown on submit")
            f.check(x_clusters(img) >= 3, "still on the Answering screen (answers kept)")
            f.check(sql("SELECT COUNT(*) FROM Users WHERE user_name='Dave'") == [["0"]], "nothing stored")
            x0, y0, x1, y1 = box
            dave.b.click(x1 - 10, (y0 + y1) // 2, settle=0.6)
            f.check(banner_box(dave.shot("14b-dismissed")) is None, "banner dismissed")

            # results view while the backend is down: Alice reloads on her Done screen
            zed = page("zed")
            # Done phase is only reachable through a successful submit; do it with the backend up.
            start_backend()
            zed.start_survey("Zed")
            zed.fill([(5, 2030), (6, 2031), (7, 2032), (8, 2033)])
            zed.submit()
            f.check(sql("SELECT COUNT(*) FROM Users WHERE user_name='Zed'") == [["1"]], "Zed submitted")
            stop_backend()
            zed.reload(settle=3.0)
            img = zed.shot("14c-results-backend-down")
            f.check(banner_box(img) is not None, "error banner shown when results cannot be loaded")
            miss = zed.wait_net(lambda e: e["method"] == "GET" and "error" in e)
            f.check(miss is not None, "browser saw the failed GET")

            # recovery: backend returns, Dave submits again
            start_backend()
            dave.b.click(*SUBMIT, settle=2.5)
            f.check(sql("SELECT COUNT(*) FROM Users WHERE user_name='Dave'") == [["1"]], "Dave's retry stored after the backend came back")
            f.check(banner_box(dave.shot("14d-recovered")) is None, "no banner after recovery")

        with Feature("15. after a refused name the participant can go back to the name field and use another") as f:
            # `alice2` is still answering as "Alice" from another device (403 in check 10)
            before = len([e for e in alice2.net() if e["method"] == "POST"])
            alice2.b.click(*CHANGE_NAME_BELOW_BANNER, settle=0.8)
            img = alice2.shot("15a-back-to-name")
            f.check(banner_box(img) is None, "the error banner is gone")
            f.check(x_clusters(img) == 0, "start screen again (no answers timeline)")
            f.check(name_warning_shown(img), "the name field kept \"Alice\" (its 'already filled in' warning shows)")
            f.check(len([e for e in alice2.net() if e["method"] == "POST"]) == before, "going back sent nothing")
            alice2.start_survey("Alice B")
            alice2.submit()
            r = alice2.wait_net(lambda e: e["method"] == "POST" and e.get("status") == 200)
            f.check(r is not None, f"submit under the new name: browser saw {r and r.get('status')}")
            f.check(sql("SELECT COUNT(*) FROM Users WHERE user_name='Alice B'") == [["1"]], "'Alice B' stored")
            f.check(user_answers("Alice") == ALICE_DB, "Alice's own answers untouched")

        with Feature("16. browser console stays free of unexpected errors") as f:
            for p in pages:
                severe = [m for m in p.severe_console() if "Failed to fetch" not in m and "ERR_CONNECTION_REFUSED" not in m and "403" not in m and "409" not in m]
                f.check(not severe, f"{p.label}: {severe[:2]}")
    finally:
        for p in pages:
            p.close()
        stop_backend()

    print("\n================ SUMMARY ================")
    for name, status, _ in RESULTS:
        print(f"{status:5} {name}")
    return 0 if all(s != "FAIL" for _, s, _ in RESULTS) else 1


if __name__ == "__main__":
    sys.exit(main())
