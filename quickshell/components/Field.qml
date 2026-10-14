// A form field (the app's Field): a bold label, the controls, and an
// optional muted note. `side`: the label and note at the left, the
// controls at the right (a select, a number). In a Group, its rows have a
// hairline between them.
import QtQuick
import QtQuick.Layouts
import qs.services

ColumnLayout {
    id: field

    property string label
    property string description
    /// The controls beside the label instead of under it.
    property bool side: false
    default property alias content: body.data
    /// In a Group's card (its rows are divided).
    readonly property bool grouped: !!parent && parent.grouped === true
    readonly property bool first: grouped && parent.visibleChildren.length > 0 && parent.visibleChildren[0] === field

    spacing: 8

    Rectangle {
        visible: field.grouped && !field.first
        Layout.fillWidth: true
        Layout.bottomMargin: 4
        implicitHeight: 1
        color: Theme.a(Theme.foreground, 0.06)
    }
    Item {
        visible: field.grouped
        implicitHeight: field.first ? 8 : 2
    }
    GridLayout {
        Layout.fillWidth: true
        columns: field.side ? 2 : 1
        columnSpacing: 16
        rowSpacing: 8
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                Layout.fillWidth: true
                text: field.label
                wrapMode: Text.Wrap
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.Bold
            }
            Text {
                visible: field.side && field.description !== ""
                Layout.fillWidth: true
                text: field.description
                wrapMode: Text.Wrap
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
        ColumnLayout {
            id: body
            Layout.fillWidth: !field.side
            Layout.alignment: field.side ? Qt.AlignRight | Qt.AlignVCenter : Qt.AlignLeft | Qt.AlignTop
            spacing: 8
        }
    }
    Text {
        visible: !field.side && field.description !== ""
        Layout.fillWidth: true
        text: field.description
        wrapMode: Text.Wrap
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }
    Item {
        visible: field.grouped
        implicitHeight: 8
    }
}
