// Which tab the picker shows: Local, Online or Collections. Anything can
// switch it (a tag in Collections' details searches on Online).
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    readonly property int local: 0
    readonly property int online: 1
    readonly property int collections: 2

    property int current: local
}
