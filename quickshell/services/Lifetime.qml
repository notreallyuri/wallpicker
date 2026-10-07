// Whether the picker is showing, and how long it stays loaded after it's
// closed (Settings → Performance, `Prefs.keepOpen` seconds). Closing hides
// it and starts the clock; opening it again in that time shows it at once,
// where it was. When the time's up (or at once, with Off) Quickshell exits
// and the backend with it: nothing stays in memory.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: life

    property bool shown: true

    /// Closed: hidden for a while, or gone.
    function close() {
        if (!shown)
            return;
        // Menus and dialogs don't wait to be found open next time. Each is
        // told once: one leaves the open list only when its closing
        // animation ends, so looping until the list is empty never ended
        // (the picker froze).
        for (const popup of Dialogs.open.slice())
            popup.close();
        if (Prefs.keepOpen <= 0) {
            Qt.quit();
            return;
        }
        shown = false;
        clock.restart();
    }

    function show() {
        clock.stop();
        shown = true;
    }

    function toggle() {
        if (shown)
            close();
        else
            show();
    }

    Timer {
        id: clock
        interval: Math.max(1, Prefs.keepOpen) * 1000
        onTriggered: Qt.quit()
    }
}
