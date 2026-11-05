#!/usr/bin/env python3
"""Works contacts through the running window with a virtual keyboard, by Enter or by paddle.
The program writes its state to the trace file, this script acts on it like an operator.

Usage: e2e.py <trace file> [qsos] [--paddle] [--shot FILE]
"""
import fcntl, os, struct, subprocess, sys, time

UI_SET_EVBIT, UI_SET_KEYBIT = 0x40045564, 0x40045565
UI_DEV_CREATE, UI_DEV_DESTROY = 0x5501, 0x5502
EV_SYN, EV_KEY = 0, 1
F1, F10, F11, ENTER, TAB, SPACE, LCTRL, RCTRL, LALT, LSHIFT = 59, 68, 87, 28, 15, 57, 29, 97, 56, 42
# QWERTZ: the driver thinks in positions, so Y and Z change places
CODES = {c: k for k, c in zip(
    [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 30, 31, 32, 33, 34, 35, 36, 37, 38, 44, 45, 46, 47, 48, 49, 50],
    "QWERTZUIOPASDFGHJKLYXCVBNM")}
CODES.update({str((i + 1) % 10): 2 + i for i in range(10)})
MORSE = dict(zip("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", (
    ".- -... -.-. -.. . ..-. --. .... .. .--- -.- .-.. -- -. --- .--. --.- .-. ... - ..- ...- .-- -..- -.-- --.. "
    "----- .---- ..--- ...-- ....- ..... -.... --... ---.. ----.").split()))


class Vk:
    def __init__(self):
        self.fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_KEY)
        for k in [F1, F10, F11, ENTER, TAB, SPACE, LCTRL, RCTRL, LALT, LSHIFT, *CODES.values()]:
            fcntl.ioctl(self.fd, UI_SET_KEYBIT, k)
        os.write(self.fd, struct.pack("80sHHHHi", b"Morse Runner test keys", 3, 0x1234, 0x567a, 1, 0) + bytes(64 * 4 * 4))
        fcntl.ioctl(self.fd, UI_DEV_CREATE)
        time.sleep(1.0)

    def edge(self, code, down):
        os.write(self.fd, struct.pack("llHHi", 0, 0, EV_KEY, code, down))
        os.write(self.fd, struct.pack("llHHi", 0, 0, EV_SYN, 0, 0))

    def tap(self, code):
        self.edge(code, 1)
        time.sleep(0.03)
        self.edge(code, 0)
        time.sleep(0.03)

    def text(self, s):
        for c in s:
            self.tap(CODES[c])

    def key_morse(self, text, wpm):
        unit = 1.2 / wpm
        for word in text.split():
            for ch in word:
                for e in MORSE[ch]:
                    code = LCTRL if e == "." else RCTRL
                    self.edge(code, 1)
                    time.sleep(unit * 0.5 if e == "." else unit * 2)
                    self.edge(code, 0)
                    time.sleep(unit * 1.6 if e == "." else unit * 2.1)
                time.sleep(unit * 2)
            time.sleep(unit * 4)

    def close(self):
        fcntl.ioctl(self.fd, UI_DEV_DESTROY)


def focused():
    return b'"title": "MORSEHUNTER"' in subprocess.check_output(["hyprctl", "activewindow", "-j"])


def state(path):
    dx, log, over = [], [], ""
    try:
        for line in open(path).read().splitlines():
            p = line.split(" ")
            if p[0] == "DX":
                dx.append({"call": p[1], "op": p[2], "st": p[3], "nr": p[4], "rst": p[5]})
            elif p[0] == "LOG":
                log.append((p[1], p[2]))
            elif p[0] == "OVER":
                over = line[5:]
    except OSError:
        pass
    return dx, log, over


def wait_quiet(path, pause=0.5, limit=20):
    end = time.time() + limit
    last = time.time()
    while time.time() < end:
        dx, _, _ = state(path)
        if any(d["st"] == "Sending" for d in dx):
            last = time.time()
        if time.time() - last > pause:
            return True
        time.sleep(0.05)
    return False


def shoot(path):
    import json
    w = json.loads(subprocess.check_output(["hyprctl", "activewindow", "-j"]))
    (x, y), (a, b) = w["at"], w["size"]
    subprocess.run(["grim", "-g", f"{x},{y} {a}x{b}", path], check=False)


def main():
    begin = time.time()
    args = [a for a in sys.argv[1:]]
    paddle = "--paddle" in args
    if paddle:
        args.remove("--paddle")
    shot = None
    if "--shot" in args:
        i = args.index("--shot")
        shot = args[i + 1]
        del args[i:i + 2]
    trace, want = args[0], int(args[1]) if len(args) > 1 else 3
    vk = Vk()
    wpm, deadline = 25, time.time() + 60 + 40 * want
    # not F9: the desktop here keeps that one for dictation
    vk.edge(LALT, 1)
    vk.tap(CODES["R"])
    vk.edge(LALT, 0)
    time.sleep(0.5)
    def caller():
        """A station waiting to be worked, as soon as it has stopped sending."""
        for _ in range(100):
            dx, _, _ = state(trace)
            for d in dx:
                if d["op"] == "NeedQso" and d["st"] == "Listening":
                    return d
            time.sleep(0.1)
        return None

    def of(call):
        # the trace is rewritten all the time: a read in the middle finds nothing, try again
        for _ in range(5):
            d = next((d for d in state(trace)[0] if d["call"] == call), None)
            if d is not None:
                return d
            time.sleep(0.02)
        return None

    while time.time() < deadline and len(state(trace)[1]) < want:
        if not focused():
            title = subprocess.check_output(["hyprctl", "activewindow", "-j"]).decode()
            print("focus lost, stopping:", [l.strip() for l in title.splitlines() if '"title"' in l or '"class"' in l], flush=True)
            break
        if not any(d["op"] == "NeedQso" for d in state(trace)[0]):
            if paddle:
                vk.key_morse("CQ TEST DL8UG", wpm)
            else:
                vk.tap(F1)
        c = caller()
        print("caller", c, flush=True)
        if c is None:
            print(open(trace).read(), flush=True)
            continue
        vk.text(c["call"])
        # a station that sends does not hear us: wait for the gap
        for _ in range(100):
            me = of(c["call"])
            if me is None or me["st"] == "Listening":
                break
            time.sleep(0.05)
        if paddle:
            # his call, the report and your own serial number, as on the air
            mine = len(state(trace)[1]) + 1
            vk.key_morse(f"{c['call']} 5NN {mine:03d}", wpm)
        else:
            vk.tap(ENTER)
        # he gives his exchange
        for _ in range(150):
            me = of(c["call"])
            # TU only to a station that listens: one about to send does not hear it
            if me is None or (me["op"] == "NeedEnd" and me["st"] == "Listening"):
                break
            time.sleep(0.1)
        print("state", me, flush=True)
        if shot and (me is None or me["op"] != "NeedEnd"):
            shoot(shot.replace(".png", "-fail.png"))
        if me is None or me["op"] != "NeedEnd":
            vk.tap(F11)
            continue
        if shot:
            shoot(shot)
        if paddle:
            vk.tap(SPACE)
        if me["rst"] != "599":
            # a LID gives 579 or so: Shift-Tab selects the middle digit of the report
            vk.edge(LSHIFT, 1)
            vk.tap(TAB)
            vk.edge(LSHIFT, 0)
            vk.text(me["rst"][1])
            vk.tap(TAB)
        vk.text(me["nr"])
        if paddle:
            vk.key_morse("TU", wpm)
        else:
            vk.tap(ENTER)
        time.sleep(1.5)
        print("log", state(trace)[1], "over", state(trace)[2], flush=True)
    print(f"after {time.time() - begin:.1f} s")
    dx, log, over = state(trace)
    print("log", log)
    print("last over", over)
    if shot:
        shoot(shot.replace(".png", "-end.png"))
    vk.tap(F10)
    time.sleep(0.5)
    vk.close()
    ok = [x for x in log if x[1] == "Ok"]
    print(f"{len(ok)} of {len(log)} clean")
    sys.exit(0 if len(log) >= want and len(ok) * 2 >= len(log) else 1)


main()
