// A muted paragraph in a Settings section (what a choice means).
import QtQuick
import QtQuick.Layouts
import qs.services

Text {
    Layout.fillWidth: true
    wrapMode: Text.Wrap
    color: Theme.mutedForeground
    font.family: Theme.font
    font.pixelSize: 13
    lineHeight: 1.15
}
