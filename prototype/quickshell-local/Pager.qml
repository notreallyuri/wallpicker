// The pager under the grid (ui/src/views/pager.rs): Previous, the page
// numbers around this one (core's page_window), Next, and Jump to (digits
// only, Enter, kept to the pages there are).
import QtQuick
import QtQuick.Layouts

RowLayout {
    id: pager

    property int current: 1
    property int last: 1
    signal goTo(int page)

    spacing: 4

    // wallhaven::page_window: 1, current ±1 and last; a gap of one page is
    // filled in, longer gaps are "…" (0 here).
    function windowOf(c, l) {
        const pages = [...new Set([1, c - 1, c, c + 1, l].filter(p => p >= 1 && p <= l))].sort((a, b) => a - b);
        const out = [];
        let prev = 0;
        for (const p of pages) {
            if (prev && p - prev === 2)
                out.push(prev + 1);
            else if (prev && p - prev > 2)
                out.push(0);
            out.push(p);
            prev = p;
        }
        return out;
    }

    Btn {
        variant: "ghost"
        icon: "chevron-left"
        text: "Previous"
        opacity: pager.current > 1 ? 1 : 0.5
        onClicked: if (pager.current > 1)
            pager.goTo(pager.current - 1)
    }
    Repeater {
        model: pager.windowOf(pager.current, pager.last)
        Btn {
            required property int modelData
            variant: modelData === pager.current ? "outline" : "ghost"
            minWidth: 36
            text: modelData === 0 ? "…" : String(modelData)
            onClicked: if (modelData > 0)
                pager.goTo(modelData)
        }
    }
    Btn {
        variant: "ghost"
        text: "Next"
        Icon {
            name: "chevron-right"
        }
        opacity: pager.current < pager.last ? 1 : 0.5
        onClicked: if (pager.current < pager.last)
            pager.goTo(pager.current + 1)
    }
    Item {
        implicitWidth: 12
    }
    Text {
        text: "Jump to"
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
    Rectangle {
        implicitWidth: 64
        implicitHeight: 32
        radius: 16
        color: Theme.a(Theme.input, 0.3)
        border.width: jump.activeFocus ? 2 : 1
        border.color: jump.activeFocus ? Theme.a(Theme.primary, 0.7) : Theme.input
        TextInput {
            id: jump
            anchors.fill: parent
            horizontalAlignment: TextInput.AlignHCenter
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            // Digits only, no spinner.
            validator: RegularExpressionValidator {
                regularExpression: /[0-9]{0,6}/
            }
            onAccepted: {
                const n = parseInt(text);
                text = "";
                if (n > 0)
                    pager.goTo(Math.min(n, pager.last));
            }
            Text {
                visible: !jump.text && !jump.activeFocus
                anchors.centerIn: parent
                text: "page"
                color: Theme.mutedForeground
                font: jump.font
            }
        }
    }
    Text {
        text: "of " + pager.last
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
}
