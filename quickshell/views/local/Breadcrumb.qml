// The way to the current folder (folders.rs's Breadcrumb): Up, then each
// step back (clickable; the current one marked), and "Searching in … and
// below" while a filter is on.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services
import "Folders.js" as Folders

RowLayout {
    id: crumbs

    required property LocalModel local

    readonly property var steps: Folders.trail(local.place, local.rootNames, "All folders")

    spacing: 4

    Btn {
        variant: "ghost"
        icon: "folder-up"
        tip: "Up a folder (Alt+Up)"
        opacity: crumbs.local.canGoUp ? 1 : 0.4
        onClicked: crumbs.local.up()
    }
    Repeater {
        model: crumbs.steps
        RowLayout {
            id: step
            required property var modelData
            required property int index
            readonly property bool current: index === crumbs.steps.length - 1
            spacing: 4
            Text {
                visible: step.index > 0
                text: "›"
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 14
            }
            Btn {
                variant: step.current ? "outline" : "ghost"
                implicitHeight: 30
                text: step.modelData.label
                onClicked: if (!step.current)
                    crumbs.local.openFolder(step.modelData.place)
            }
        }
    }
    Text {
        visible: crumbs.local.filtering
        Layout.leftMargin: 8
        text: "Searching in " + (crumbs.steps.length ? crumbs.steps[crumbs.steps.length - 1].label : "") + " and below"
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
}
