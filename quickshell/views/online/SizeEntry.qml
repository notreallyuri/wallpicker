// A size typed in (sizes.rs SizeEntry): a field and a button; Enter or the
// button hands a valid WIDTHxHEIGHT (× and spaces allowed) to `added`, then
// clears. Anything else turns the field red.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: entry

    property string placeholder
    property string button
    signal added(string size)

    spacing: 8

    function submit() {
        const v = field.text.trim().toLowerCase().replace(/[× ]/g, "x");
        if (/^\d{1,5}x\d{1,5}$/.test(v)) {
            added(v);
            field.text = "";
            field.invalid = false;
        } else {
            field.invalid = v !== "";
        }
    }

    TextField {
        id: field
        implicitWidth: 176
        implicitHeight: 32
        placeholder: entry.placeholder
        onTextChanged: invalid = false
        onAccepted: entry.submit()
    }
    Btn {
        implicitHeight: 32
        text: entry.button
        onClicked: entry.submit()
    }
}
