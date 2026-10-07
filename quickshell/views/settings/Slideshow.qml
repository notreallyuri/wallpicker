// Settings → Slideshow: on, how often, the day's set, which monitors (one
// wallpaper each or the same), the night's set and its hours; when it
// changes next, and Change now (at once).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    readonly property var s: form ? (form.draft.slideshow || {}) : ({})
    readonly property var night: s.night || {}
    property string status: ""
    property bool busy: false

    function refresh() {
        Backend.request("slideshow.status", {}, st => {
            if (!st.enabled) {
                section.status = "Off.";
                return;
            }
            const when = st.period === "night" ? "Night now. " : st.period ? "Day now. " : "";
            section.status = st.next_in !== null && st.next_in !== undefined ? when + "Next change in about " + Math.max(1, Math.ceil(st.next_in / 60)) + " min." : "";
        }, () => section.status = "");
    }
    Component.onCompleted: refresh()

    spacing: 20

    ToggleField {
        label: "Change the wallpaper by itself"
        checked: !!section.s.enabled
        onToggled: section.form.set("slideshow.enabled", !section.s.enabled)
    }
    Field {
        label: "Every"
        Select {
            minWidth: 180
            options: [["5", "5 minutes"], ["10", "10 minutes"], ["15", "15 minutes"], ["30", "30 minutes"], ["60", "1 hour"], ["120", "2 hours"], ["240", "4 hours"], ["480", "8 hours"], ["1440", "24 hours"]]
            value: String(section.s.minutes || 30)
            onPicked: v => section.form.set("slideshow.minutes", parseInt(v))
        }
    }
    PickFields {
        Layout.fillWidth: true
        form: section.form
        path: "slideshow"
    }
    Field {
        visible: Monitors.list.length > 1
        label: "On these monitors"
        description: "None picked: all of them."
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: Monitors.list.map(m => m.name)
                Chip {
                    required property string modelData
                    readonly property var picked: section.s.monitors || []
                    text: modelData
                    on: picked.includes(modelData)
                    onClicked: section.form.set("slideshow.monitors", on ? picked.filter(m => m !== modelData) : picked.concat([modelData]))
                }
            }
        }
    }
    ToggleField {
        visible: Monitors.list.length > 1
        label: "A different wallpaper on each monitor"
        checked: !!section.s.each
        onToggled: section.form.set("slideshow.each", !section.s.each)
    }

    // The night: its own set, in its own hours.
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: nightBox.implicitHeight + 32
        radius: 18
        color: "transparent"
        border.width: 2
        border.color: Theme.a(Theme.border, 0.6)
        ColumnLayout {
            id: nightBox
            x: 16
            y: 16
            width: parent.width - 32
            spacing: 16
            ToggleField {
                label: "Something else at night"
                checked: !!section.night.enabled
                onToggled: section.form.set("slideshow.night.enabled", !section.night.enabled)
            }
            RowLayout {
                visible: !!section.night.enabled
                spacing: 8
                Text {
                    text: "From"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                }
                TextField {
                    implicitWidth: 90
                    align: TextInput.AlignHCenter
                    text: section.night.from || ""
                    onTextChanged: if (text !== section.night.from)
                        section.form.set("slideshow.night.from", text.trim())
                }
                Text {
                    text: "to"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                }
                TextField {
                    implicitWidth: 90
                    align: TextInput.AlignHCenter
                    text: section.night.to || ""
                    onTextChanged: if (text !== section.night.to)
                        section.form.set("slideshow.night.to", text.trim())
                }
            }
            PickFields {
                visible: !!section.night.enabled
                Layout.fillWidth: true
                form: section.form
                path: "slideshow.night"
            }
            Note {
                visible: !!section.night.enabled
                text: "It changes right away when night starts and ends, then every interval as usual."
            }
        }
    }

    RowLayout {
        spacing: 12
        Btn {
            text: section.busy ? "Changing…" : "Change now"
            opacity: section.busy ? 0.5 : 1
            onClicked: {
                if (section.busy)
                    return;
                section.busy = true;
                Monitors.expectAll();
                Backend.request("slideshow.now", {}, () => {
                    Toasts.info("New wallpaper");
                    section.busy = false;
                    section.refresh();
                }, e => {
                    Toasts.error("The slideshow could not change it", e);
                    section.busy = false;
                    section.refresh();
                });
            }
        }
        Text {
            text: section.status
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 13
        }
    }
    Note {
        text: "Kept on time by the Noctalia plugin while Noctalia runs, or by `wallpicker slideshow run` (for a Hyprland autostart). Save to apply changes here; Change now uses what's saved."
    }
}
