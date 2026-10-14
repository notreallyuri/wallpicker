// One line of soft badges (a tile's artists, then its tags): as many as fit
// whole, then +N for the rest, instead of the last one cut in the middle.
import QtQuick

Item {
    id: line

    /// [{text, icon}] ("" icon: none)
    property var items: []

    implicitHeight: shown.implicitHeight

    // Every badge measured, unseen, to know how many fit.
    Row {
        id: measure
        visible: false
        spacing: 4
        Repeater {
            id: measured
            model: line.items
            Badge {
                required property var modelData
                soft: true
                icon: modelData.icon || ""
                text: modelData.text
            }
        }
    }
    Badge {
        id: plus
        visible: false
        soft: true
        text: "+" + line.items.length
    }

    /// How many fit, with room for +N when some don't.
    readonly property int fits: {
        let used = 0;
        for (let i = 0; i < measured.count; i++) {
            const badge = measured.itemAt(i);
            if (!badge)
                return i;
            const room = i < measured.count - 1 ? plus.implicitWidth + 4 : 0;
            if (used + badge.implicitWidth + room > line.width)
                return i;
            used += badge.implicitWidth + 4;
        }
        return measured.count;
    }

    Row {
        id: shown
        spacing: 4
        Repeater {
            model: line.items.slice(0, line.fits)
            Badge {
                required property var modelData
                soft: true
                icon: modelData.icon || ""
                text: modelData.text
            }
        }
        Badge {
            visible: line.fits < line.items.length
            soft: true
            text: "+" + (line.items.length - line.fits)
        }
    }
}
