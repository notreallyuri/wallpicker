#!/usr/bin/env python3
"""Run animation lifecycle checks on private KWin virtual outputs, never the desktop."""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, help="optional local GIF/WebP instead of generated fixture")
    parser.add_argument("--fallback", action="store_true", help="qualify representative stills with software EGL rejected")
    parser.add_argument("--daemon", action="store_true",
                        help="drive the real daemon and CLI: playback policy, pause, restore (needs Pillow)")
    args = parser.parse_args()
    for binary in ("kwin_wayland", "dbus-run-session", "kscreen-doctor", "fusermount3", "timeout", "cargo"):
        if not shutil.which(binary):
            parser.error(f"missing required executable: {binary}")
    root = Path(__file__).resolve().parents[1]
    subprocess.run(["cargo", "build", "--offline", "-p", "wallpicker", "--bin", "wallpicker"], cwd=root, check=True)
    subprocess.run(["cargo", "test", "--offline", "-p", "wallpicker", "--lib", "--no-run"], cwd=root, check=True)
    with tempfile.TemporaryDirectory(prefix="wallpicker-wayland-") as directory:
        base = Path(directory)
        env = os.environ.copy()
        for key in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "SESSION_MANAGER",
                    "HYPRLAND_INSTANCE_SIGNATURE"):
            env.pop(key, None)
        for key, folder in (("XDG_RUNTIME_DIR", "runtime"), ("XDG_CONFIG_HOME", "config"),
                            ("XDG_CACHE_HOME", "cache"), ("XDG_DATA_HOME", "data"),
                            ("XDG_STATE_HOME", "state")):
            path = base / folder
            path.mkdir(mode=0o700)
            env[key] = str(path)
        env["QT_QPA_PLATFORM"] = "offscreen"
        env["QT_NO_XDG_DESKTOP_PORTAL"] = "1"
        env["WALLPICKER_PRIVATE_COMPOSITOR"] = "1"
        env["WALLPICKER_TEST_EXECUTABLE"] = str(root / "target/debug/wallpicker")
        if args.source:
            env["WALLPICKER_TEST_ANIMATION"] = str(args.source.resolve(strict=True))
        else:
            env.pop("WALLPICKER_TEST_ANIMATION", None)
        log = base / "compositor.log"
        with log.open("w") as output:
            process = subprocess.Popen([
                "dbus-run-session", "--", "python3", "-c",
                "import os,pathlib,sys; pathlib.Path(sys.argv[1]).write_text(os.environ['DBUS_SESSION_BUS_ADDRESS']); os.execvp(sys.argv[2],sys.argv[2:])",
                str(base / "private-bus"), "kwin_wayland", "--virtual", "--no-lockscreen",
                "--no-global-shortcuts", "--no-kactivities", "--socket", "wallpicker-test",
                "--width", "960", "--height", "540", "--output-count", "2",
            ], env=env, cwd=base, stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                socket = Path(env["XDG_RUNTIME_DIR"]) / "wallpicker-test"
                deadline = time.monotonic() + 15
                while not socket.exists():
                    if process.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError("private virtual compositor failed to start")
                    time.sleep(0.05)
                env["WAYLAND_DISPLAY"] = "wallpicker-test"
                env["DBUS_SESSION_BUS_ADDRESS"] = (base / "private-bus").read_text()
                if args.fallback:
                    # Only the test renderer gets software EGL; KWin was already
                    # started with the original environment on its private bus.
                    env["LIBGL_ALWAYS_SOFTWARE"] = "1"
                if args.daemon:
                    daemon_scenario(root, base, env, args.source)
                else:
                    subprocess.run([
                        "cargo", "test", "--offline", "-p", "wallpicker", "--lib",
                        "private_wayland_representative_fallback" if args.fallback else "private_wayland_animated_lifecycle",
                        "--", "--ignored", "--nocapture",
                    ], cwd=root, env=env, check=True, timeout=120)
            except BaseException:
                print(log.read_text()[-10000:])
                raise
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
                # A Qt helper can activate a private document portal. Its FUSE
                # mount may outlive the bus; clean up only this run's mount.
                document_mount = str(Path(env["XDG_RUNTIME_DIR"]) / "doc")
                mounted = lambda: any(line.split()[4] == document_mount
                                      for line in Path("/proc/self/mountinfo").read_text().splitlines())
                if mounted():
                    result = subprocess.run(["fusermount3", "-u", document_mount], capture_output=True, text=True)
                    if result.returncode and mounted():
                        raise RuntimeError(result.stderr)


def daemon_scenario(root, base, env, source=None):
    """The playback policy through the real daemon, CLI and settings file."""
    from PIL import Image

    env = dict(env)
    # The daemon reads saved state and settings from HOME too; Noctalia and
    # Hyprland must never be reached from here.
    env["HOME"] = str(base / "home")
    (base / "home").mkdir()
    stubs = base / "bin"
    stubs.mkdir()
    for name in ("noctalia", "hyprctl"):
        (stubs / name).write_text("#!/bin/sh\nexit 1\n")
        (stubs / name).chmod(0o755)
    env["PATH"] = f"{stubs}:{env['PATH']}"
    wallpicker = str(root / "target/debug/wallpicker")
    moving = source.resolve(strict=True) if source else base / "moving.webp"
    if not source:
        frames = [Image.new("RGB", (64, 36), color) for color in ((255, 0, 0), (0, 0, 255), (0, 255, 0))]
        frames[0].save(moving, save_all=True, append_images=frames[1:], duration=60, loop=0)
    still = base / "still.png"
    Image.new("RGB", (64, 36), (255, 255, 0)).save(still)

    def cli(*arguments):
        return subprocess.run([wallpicker, *arguments], env=env, check=True, timeout=60,
                              stdout=subprocess.PIPE, text=True).stdout

    def kscreen(*arguments):
        # Only ever the private KWin: WAYLAND_DISPLAY and the bus are its own.
        # (It hangs on Qt's offscreen platform.)
        subprocess.run(["kscreen-doctor", *arguments], env={**env, "QT_QPA_PLATFORM": "wayland"}, check=True, timeout=30,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def ask(request):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
            connection.settimeout(30)
            connection.connect(f"\0wallpicker-daemon:{env['XDG_RUNTIME_DIR']}:")
            connection.sendall((json.dumps(request) + "\n").encode())
            data = b""
            while not data.endswith(b"\n"):
                chunk = connection.recv(65536)
                if not chunk:
                    break
                data += chunk
        return json.loads(data)

    def wait(what, predicate, timeout=10):
        deadline = time.monotonic() + timeout
        while True:
            try:
                status = ask({"cmd": "status"})
                if status.get("screens") and predicate(status, status["screens"]):
                    return status
            except (OSError, ValueError):
                status = None
            if time.monotonic() >= deadline:
                raise AssertionError(f"timed out waiting for {what}: {status}")
            time.sleep(0.1)

    def states(screens):
        return {(s["playback"] or {}).get("state") for s in screens}

    def decoders(pid):
        found = []
        for stat in Path("/proc").glob("[0-9]*/stat"):
            try:
                if int(stat.read_text().rsplit(")", 1)[1].split()[1]) == pid:
                    if b"__decode" in (stat.parent / "cmdline").read_bytes():
                        found.append(stat.parent.name)
            except (OSError, IndexError, ValueError):
                pass
        return found

    def start():
        log = (base / "daemon.log").open("a")
        process = subprocess.Popen([wallpicker, "daemon"], env=env, stdout=log, stderr=log)
        wait("monitors", lambda st, sc: len(sc) == 2 and all(s["size"] for s in sc))
        return process

    def stop(process):
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

    cli("settings", "set", "animation.enabled=true")
    daemon = start()
    try:
        cli("apply", str(moving))
        playing = lambda st, sc: states(sc) == {"playing"} and all(s["playback"]["presentations"] >= 3 for s in sc)
        status = wait("shared playback", playing)
        screens = status["screens"]
        assert status["playback_workers"] == 1, status
        assert screens[0]["playback"]["generation"] == screens[1]["playback"]["generation"], status
        assert len(decoders(daemon.pid)) == 1, decoders(daemon.pid)
        generation = screens[0]["playback"]["generation"]
        print("playing:", [s["playback"]["presentations"] for s in screens])

        # A live adjustment keeps the session (and its clock).
        monitor = screens[0]["monitor"]
        cli("adjust", monitor, "--frame", "200,70,40")
        status = wait("adjusted playback", lambda st, sc: sc[0]["playback"] and sc[0]["playback"]["geometry"]["view"]["kind"] == "frame"
                      and states(sc) == {"playing"})
        assert status["screens"][0]["playback"]["generation"] == generation, status
        print("adjusted without a new decoder")

        cli("settings", "set", "animation.paused=true")
        status = wait("pause", lambda st, sc: states(sc) == {"paused"} and st["playback_paused"])
        held = [s["playback"]["sequence"] for s in status["screens"]]
        time.sleep(1)
        status = ask({"cmd": "status"})
        assert [s["playback"]["sequence"] for s in status["screens"]] == held, status
        assert not status["playback_timer"], status
        cli("settings", "set", "animation.paused=false")
        wait("resume", lambda st, sc: states(sc) == {"playing"} and [s["playback"]["sequence"] for s in sc] != held)
        print("paused and resumed")

        cli("settings", "set", "animation.enabled=false")
        wait("stills", lambda st, sc: states(sc) == {"still"} and st["playback_workers"] == 0)
        assert not ask({"cmd": "capabilities"})["animated_playback"]
        assert not decoders(daemon.pid), decoders(daemon.pid)
        print("off: first-frame stills, decoders gone")

        cli("settings", "set", "animation.enabled=true")
        wait("playing again", playing)
        print("on again: restored playback")

        cli("apply", str(still))
        wait("static", lambda st, sc: states(sc) == {None} and st["playback_workers"] == 0)
        assert not decoders(daemon.pid), decoders(daemon.pid)
        print("static wallpaper: no playback left")

        cli("apply", str(moving))
        wait("playing before restart", playing)
        stop(daemon)
        daemon = start()
        status = wait("playing after restart", playing)
        print("daemon restart: saved animation plays again")
        monitors = [s["monitor"] for s in status["screens"]]

        # A monitor connected later shows its saved animation, playing.
        kscreen(f"output.{monitors[1]}.disable")
        wait("one monitor", lambda st, sc: len(sc) == 1 and states(sc) == {"playing"})
        kscreen(f"output.{monitors[1]}.enable")
        wait("reconnected monitor playing",
             lambda st, sc: len(sc) == 2 and all(s["size"] for s in sc) and states(sc) == {"playing"})
        print("monitor reconnected: its animation plays")

        # Hidden outputs (locked, powered off) are covered by the runtime test
        # with withheld callbacks: KWin's virtual outputs ignore DPMS, and KWin
        # keeps answering surfaces covered by a window.

        cli("span", str(moving), "--monitors", ",".join(monitors))
        spanned = lambda st, sc: (states(sc) == {"playing"} and len({json.dumps(s["playback"]["geometry"]) for s in sc}) == 2
                                  and all(s["playback"]["presentations"] >= 2 for s in sc))
        wait("span playing", spanned)
        print("span: each monitor plays its part")

        preset = json.loads(cli("presets", "save", "moving span", "--json"))["id"]
        cli("apply", str(still))
        wait("static", lambda st, sc: states(sc) == {None} and st["playback_workers"] == 0)
        cli("presets", "apply", preset)
        wait("preset playing", spanned)
        print("preset: the spanned animation plays again")

        cli("apply", str(still))
        wait("static", lambda st, sc: states(sc) == {None} and st["playback_workers"] == 0)
        cli("previous")
        wait("previous playing", lambda st, sc: states(sc) == {"playing"})
        print("previous: back to the animation, playing")

        library = base / "library"
        library.mkdir()
        shutil.copy(moving, library / moving.name)
        cli("settings", "set", f"folders={library}")
        cli("apply", str(still))
        wait("static", lambda st, sc: states(sc) == {None} and st["playback_workers"] == 0)
        cli("random", "--folder", str(library))
        wait("random playing", playing)
        print("random: the animation plays")
    except BaseException:
        print((base / "daemon.log").read_text()[-10000:])
        raise
    finally:
        stop(daemon)


if __name__ == "__main__":
    main()
