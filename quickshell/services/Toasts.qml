// Short messages in the corner: errors (with what went wrong) and
// confirmations ("Path copied"). Each goes away on its own.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: toasts

    /// [{id, title, detail, error}], oldest first.
    property var list: []
    property int nextId: 1

    function show(title, detail, error) {
        const id = nextId++;
        list = list.concat([{
                id: id,
                title: title,
                detail: detail || "",
                error: !!error
            }]).slice(-4);
        Qt.callLater(() => timer.createObject(toasts, {
                toastId: id,
                interval: error ? 8000 : 3000
            }));
    }

    function info(title, detail) {
        show(title, detail, false);
    }
    function error(title, detail) {
        show(title, detail, true);
    }
    function dismiss(id) {
        list = list.filter(t => t.id !== id);
    }

    property Component timer: Component {
        Timer {
            property int toastId
            running: true
            onTriggered: {
                toasts.dismiss(toastId);
                destroy();
            }
        }
    }
}
