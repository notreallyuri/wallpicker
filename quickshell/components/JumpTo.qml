// "Jump to [page] of N": digits only (no spinner arrows), Enter goes, kept to
// the pages there are.
import QtQuick
import QtQuick.Layouts
import qs.services

RowLayout {
    id: jumpTo

    property int last: 1
    signal jump(int page)

    spacing: 8

    Text {
        text: "Jump to"
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
    TextField {
        id: field
        implicitWidth: 64
        implicitHeight: 32
        placeholder: "page"
        align: TextInput.AlignHCenter
        validator: RegularExpressionValidator {
            regularExpression: /[0-9]{0,6}/
        }
        onAccepted: {
            const n = parseInt(text);
            text = "";
            if (n > 0)
                jumpTo.jump(Math.min(n, jumpTo.last));
        }
    }
    Text {
        text: "of " + jumpTo.last
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
}
