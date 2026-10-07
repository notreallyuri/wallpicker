// Settings (ui/src/views/settings.rs): a sidebar of sections, the chosen
// one's pane, and Save at the bottom for what's kept in config.json. A
// section is its own file here, built when it's shown; its edits go to the
// form's draft, so switching sections keeps them. Opened by
// services/Dialogs.openSettings (the top bar's cog).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.components
import qs.services

DialogFrame {
    id: dialog

    readonly property var sections: [
        {
            slug: "general",
            label: "General",
            group: "App",
            file: "General.qml",
            blurb: "Where your wallpapers are, and how they're shown and applied."
        },
        {
            slug: "appearance",
            label: "Appearance",
            group: "App",
            file: "Appearance.qml",
            blurb: "How wallpicker looks."
        },
        {
            slug: "downloads",
            label: "Downloads",
            group: "App",
            file: "Downloads.qml",
            blurb: "Where wallpapers from the online providers are saved."
        },
        {
            slug: "slideshow",
            label: "Slideshow",
            group: "App",
            file: "Slideshow.qml",
            blurb: "A new wallpaper every few minutes, picked at random."
        },
        {
            slug: "workspaces",
            label: "Workspaces",
            group: "App",
            file: "Workspaces.qml",
            blurb: "A wallpaper for each Hyprland workspace, put on as you switch to it."
        },
        {
            slug: "performance",
            label: "Performance",
            group: "App",
            file: "Performance.qml",
            blurb: "What wallpicker uses right now, and the effects that cost the most."
        },
        {
            slug: "backup",
            label: "Backup",
            group: "App",
            file: "Backup.qml",
            blurb: "Your favorites, tags and fits: copied every day on their own, and to a file when you want."
        },
        {
            slug: "wallhaven",
            label: "Wallhaven",
            group: "Providers",
            file: "Wallhaven.qml",
            blurb: "wallhaven.cc"
        },
        {
            slug: "konachan",
            label: "Konachan",
            group: "Providers",
            file: "Konachan.qml",
            blurb: "Anime wallpapers, in a safe-for-work and a full version."
        },
        {
            slug: "yandere",
            label: "yande.re",
            group: "Providers",
            file: "Yandere.qml",
            blurb: "High-resolution anime scans."
        }
    ]
    /// Sections built so far (the rest show Pending until their step).
    readonly property var built: ["general", "downloads", "wallhaven", "konachan", "yandere"]
    /// The section shown.
    property string current: "general"
    readonly property var section: sections.find(s => s.slug === current) || sections[0]

    preferredWidth: 880
    height: Math.min(640, parent ? parent.height - 48 : 640)
    padding: 0

    readonly property string wanted: Dialogs.settings
    onWantedChanged: if (wanted !== "") {
        current = wanted;
        form.load();
        open();
    }
    onClosed: Dialogs.settings = ""

    SettingsForm {
        id: form
    }

    contentItem: RowLayout {
        spacing: 0
        SettingsSidebar {
            Layout.preferredWidth: 192
            Layout.fillHeight: true
            topLeftRadius: 24
            bottomLeftRadius: 24
            sections: dialog.sections
            current: dialog.current
            onPicked: slug => dialog.current = slug
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // The section's name; the corner is left to nothing (Esc closes).
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 52
                Text {
                    x: 20
                    anchors.verticalCenter: parent.verticalCenter
                    text: dialog.section.label
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 15
                    font.weight: Font.ExtraBold
                }
                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: 1
                    color: Theme.a(Theme.border, 0.6)
                }
            }

            Flickable {
                id: pane
                Layout.fillWidth: true
                Layout.fillHeight: true
                contentHeight: body.implicitHeight + 40
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}
                SmoothScroll {
                    flickable: pane
                }
                ColumnLayout {
                    id: body
                    x: 20
                    y: 20
                    width: pane.width - 40
                    spacing: 20
                    Text {
                        Layout.fillWidth: true
                        text: dialog.section.blurb
                        wrapMode: Text.Wrap
                        color: Theme.mutedForeground
                        font.family: Theme.font
                        font.pixelSize: 13
                    }
                    Loader {
                        id: sectionLoader
                        Layout.fillWidth: true
                        // Built when shown, with the form to edit.
                        source: dialog.opened ? Qt.resolvedUrl(dialog.built.includes(dialog.section.slug) ? dialog.section.file : "Pending.qml") : ""
                        onLoaded: {
                            item.form = form;
                            pane.contentY = 0;
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.margins: 12
                Layout.leftMargin: 20
                spacing: 8
                Text {
                    Layout.fillWidth: true
                    text: "Saved to ~/.config/wallpicker/config.json"
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                }
                Btn {
                    variant: "primary"
                    text: form.saving ? "Saving…" : "Save"
                    opacity: form.saving ? 0.5 : 1
                    onClicked: if (!form.saving)
                        form.save(() => dialog.close())
                }
            }
        }
    }
}
