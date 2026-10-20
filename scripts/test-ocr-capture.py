"""Controlled-window WGC + UDP integration. Does not operate the actual game.

The fixture window stays at the bottom of the desktop without taking focus.
Requires development-only Pillow and tkinter, not part of the OCR distribution.
"""
import ctypes
from ctypes import wintypes
import json
from pathlib import Path
import queue
import runpy
import socket
import subprocess
import threading
import time
import tkinter as tk

from PIL import Image, ImageTk

fixture = runpy.run_path(str(Path(__file__).with_name("test-native-ocr.py")))
EXE, EXPECTED, ROOT = (fixture[key] for key in ("EXE", "EXPECTED", "ROOT"))


def main():
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    user32.SetProcessDpiAwarenessContext.argtypes = [wintypes.HANDLE]
    user32.SetProcessDpiAwarenessContext(wintypes.HANDLE(-4))
    user32.GetAncestor.argtypes = [wintypes.HWND, wintypes.UINT]
    user32.GetAncestor.restype = wintypes.HWND
    user32.SetWindowPos.argtypes = [wintypes.HWND, wintypes.HWND, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, wintypes.UINT]
    root = tk.Tk()
    root.title("Echo OCR controlled capture fixture")
    root.geometry("960x540+80+80")
    label = tk.Label(root, borderwidth=0, highlightthickness=0)
    label.pack(fill="both", expand=True)
    photo = None
    proc = None
    udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    events = queue.Queue()
    seen = []

    def display(name):
        nonlocal photo
        source = Image.open(ROOT / "captures/ocr" / f"{name}.png").convert("RGB").crop((2, 45, 1922, 1125))
        photo = ImageTk.PhotoImage(source.resize((960, 540), Image.Resampling.LANCZOS))
        label.configure(image=photo)
        root.update()

    def background():
        root.update()
        hwnd = user32.GetAncestor(root.winfo_id(), 2)
        assert user32.SetWindowPos(hwnd, wintypes.HWND(1), 0, 0, 0, 0, 0x13)  # HWND_BOTTOM, no move/size/activate
        return hwnd

    def pump():
        root.update()
        while True:
            try:
                item = events.get_nowait()
                seen.append(item)
                assert item.get("status") not in ("error", "capture_error"), item
            except queue.Empty:
                break
        if proc and proc.poll() is not None:
            raise AssertionError(("sidecar exited", proc.returncode, seen))
        time.sleep(0.02)

    def packet(expected, timeout=10):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            pump()
            try:
                result = json.loads(udp.recv(16384))
                actual = [(e["buffName"], e["buffValue"]) for e in result["buffEntries"]]
                assert actual == expected, (actual, expected)
                return
            except BlockingIOError:
                pass
        raise AssertionError(("UDP timeout", expected, seen))

    try:
        udp.bind(("127.0.0.1", 9999))
        udp.setblocking(False)
        display("1-2")
        hwnd = background()
        proc = subprocess.Popen([str(EXE), "--hwnd", str(hwnd)], stdin=subprocess.PIPE,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                                encoding="utf-8", creationflags=subprocess.CREATE_NO_WINDOW)

        def read():
            for line in proc.stdout:
                events.put(json.loads(line))

        threading.Thread(target=read, daemon=True).start()
        packet(EXPECTED["1-2"])
        display("2-3")
        background()
        packet(EXPECTED["2-3"])
        root.iconify()
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            pump()
        assert any(e.get("status") == "paused" for e in seen), seen
        try:
            unexpected = udp.recv(16384)
            raise AssertionError(("minimized window emitted a packet", unexpected))
        except BlockingIOError:
            pass
        root.deiconify()
        background()
        packet(EXPECTED["2-3"])
        proc.stdin.write("q\n")
        proc.stdin.flush()
        proc.wait(timeout=5)
        assert proc.returncode == 0, proc.stderr.read()
        print(json.dumps({"controlled_wgc": "passed", "checks": ["background client crop", "stable UDP result", "scene change", "minimize pause", "restore rescan", "graceful exit"], "events": seen}, ensure_ascii=False, indent=2))
    finally:
        if proc and proc.poll() is None:
            proc.kill()
            proc.wait(timeout=5)
        udp.close()
        root.destroy()


if __name__ == "__main__":
    main()
