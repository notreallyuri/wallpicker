// A line of the details' text: plain, or muted ("Uploaded by").
import QtQuick
import qs.services

Text {
    property bool muted: false

    color: muted ? Theme.mutedForeground : Theme.foreground
    font.family: Theme.font
    font.pixelSize: 14
    font.weight: muted ? Font.Normal : Font.Bold
}
