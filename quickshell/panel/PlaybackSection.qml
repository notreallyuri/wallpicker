// The control panel's Playback, for an animated wallpaper: what it's doing
// (the renderer's status, read every 2 s while it's shown) and Pause/Play
// for this wallpaper: held on every monitor showing it (they share one
// clock), apart from Settings' Pause all. Paused here, it stays paused
// until played or replaced, a restart too. A still says why.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

PanelSection {
    id: section

    required property string monitor
    required property string path
    /// Its frame count (the library's), for the line under Playing.
    property int frames: 0

    /// {state, fps, reason, held}; null while unknown.
    readonly property var status: Playback.of(monitor)
    readonly property string state: status ? status.state : ""
    readonly property bool held: !!status && !!status.held
    /// The other monitors showing it (one clock with this one).
    readonly property var others: Monitors.showing(path).filter(n => n !== monitor)

    function pause() {
        Monitors.setAdjust(monitor, Object.assign({}, Monitors.adjustOf(monitor), {
            paused: true
        }));
    }
    /// Off everywhere it's held: a monitor sharing it would keep it paused.
    function play() {
        for (const name of [monitor].concat(others)) {
            const a = Monitors.adjustOf(name);
            if (a.paused)
                Monitors.setAdjust(name, Object.assign({}, a, {
                    paused: false
                }));
        }
    }

    // [title, line, tone ("primary", "muted", "warning", "destructive"), icon]
    readonly property var look: {
        switch (state) {
        case "playing":
            return ["Playing" + (status.fps ? " · " + Math.round(status.fps) + " fps" : ""), frames > 1 ? frames + " frames, looping" : "Looping", "primary", "play"];
        case "paused":
            return held ? ["Paused", "Held on this frame until you play it, after a restart too.", "muted", "pause"] : ["Paused", "Pause all is on (Settings → Animated wallpapers).", "muted", "pause"];
        case "hidden":
            return ["Resting", "Not on screen (locked or powered off): nothing is decoded. It goes on from the same frame.", "muted", "pause"];
        case "still":
            return ["First frame", Playback.enabled ? "It couldn't start playing." : "Playback is off (experimental).", "warning", "film"];
        case "unavailable":
            return ["First frame", status.reason || "No hardware GPU playback on this system.", "warning", "info"];
        case "frozen":
            return ["Stopped", (status.reason || "A frame couldn't be decoded") + "; the last good frame stays.", "destructive", "x"];
        default:
            return ["Starting…", "Getting it ready to play.", "muted", "film"];
        }
    }
    readonly property color tone: ({
            primary: Theme.primary,
            warning: Theme.warning,
            destructive: Theme.destructive
        })[look[2]] || Theme.mutedForeground

    title: "Playback"
    hint: state === "playing" && status.fps ? Math.round(status.fps) + " fps" : state === "paused" ? "Paused" : ""

    Component.onCompleted: Playback.watchers++
    Component.onDestruction: Playback.watchers--

    RowLayout {
        Layout.fillWidth: true
        spacing: 10
        Rectangle {
            Layout.alignment: Qt.AlignTop
            implicitWidth: 28
            implicitHeight: 28
            radius: 14
            color: Theme.a(section.tone, 0.15)
            Icon {
                anchors.centerIn: parent
                name: section.look[3]
                size: 14
                color: section.tone
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                text: section.look[0]
                color: section.look[2] === "muted" ? Theme.foreground : section.tone
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.Bold
            }
            Text {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: section.look[1]
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
    }

    Btn {
        visible: section.state === "playing"
        small: true
        icon: "pause"
        text: "Pause"
        tip: section.others.length > 0 ? "Holds it on " + [section.monitor].concat(section.others).join(" and ") : "Holds it on its frame"
        onClicked: section.pause()
    }
    Btn {
        visible: section.state === "paused" && section.held
        small: true
        variant: "primary"
        icon: "play"
        text: "Play"
        onClicked: section.play()
    }
    Btn {
        visible: section.state === "paused" && !section.held
        small: true
        icon: "play"
        text: "Resume all"
        onClicked: Playback.setPaused(false)
    }
    Btn {
        visible: section.state === "still" && !Playback.enabled
        small: true
        icon: "settings"
        text: "Turn it on…"
        onClicked: Dialogs.openSettings("animation")
    }
    Btn {
        visible: section.state === "frozen"
        small: true
        icon: "refresh"
        text: "Try again"
        tip: "Put it on " + section.monitor + " again"
        onClicked: Monitors.applyTo(section.path, [section.monitor])
    }

    Text {
        visible: section.others.length > 0
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        text: "Also on " + section.others.join(", ") + ": one clock for all of them, so pausing here holds it there too."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
}
