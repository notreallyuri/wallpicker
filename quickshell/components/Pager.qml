// The pager under a grid (ui/src/views/pager.rs): Previous, the page
// numbers around this one, Next, and Jump to.
import QtQuick
import QtQuick.Layouts
import qs.services
import "PageWindow.js" as PageWindow

RowLayout {
    id: pager

    property int current: 1
    property int last: 1
    signal goTo(int page)

    spacing: 4

    Btn {
        variant: "ghost"
        icon: "chevron-left"
        text: "Previous"
        opacity: pager.current > 1 ? 1 : 0.5
        onClicked: if (pager.current > 1)
            pager.goTo(pager.current - 1)
    }
    Repeater {
        model: PageWindow.pages(pager.current, pager.last)
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
        opacity: pager.current < pager.last ? 1 : 0.5
        onClicked: if (pager.current < pager.last)
            pager.goTo(pager.current + 1)
        Icon {
            name: "chevron-right"
        }
    }
    JumpTo {
        Layout.leftMargin: 12
        last: pager.last
        onJump: page => pager.goTo(page)
    }
}
