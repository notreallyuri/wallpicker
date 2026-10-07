// A whole number typed in (digits only), kept within `from`..`to` by the
// backend when saved.
import QtQuick
import qs.components

TextField {
    id: field

    property int value: 0
    property int from: 0
    property int to: 99999
    signal edited(int value)

    implicitWidth: 120
    text: String(value)
    validator: IntValidator {
        bottom: 0
        top: field.to
    }
    onTextChanged: {
        const n = parseInt(text);
        if (!isNaN(n) && n !== value)
            edited(n);
    }
}
