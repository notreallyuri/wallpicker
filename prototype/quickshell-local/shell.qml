// wallpicker's Local tab in Quickshell (QML), as 1:1 with the app as it
// goes: to judge looks, feel, memory and open time before deciding on a
// port. The Rust core does the work through the CLI (Backend.qml).
//
//   qs -p prototype/quickshell-local
//   qs -p prototype/quickshell-local ipc call wallpicker toggle
import QtQuick
import QtQuick.Controls
import QtQuick.Effects
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Widgets

ShellRoot {
    id: root

    /// Local tiles per page, as in the app.
    readonly property int pageSize: 60

    property string query: ""
    property bool favoritesOnly: false
    property string sort: "newest"
    property var tagFilter: []
    property bool tagsExpanded: false
    property int page: 1
    /// Monitors a click applies to (the focused one at first).
    property var targets: []
    /// Thumbnails loaded on this page (for timing: logged once all are in).
    property int thumbsReady: 0
    property bool timed: false
    property real openedAt: Date.now()
    onThumbsReadyChanged: {
        if (!timed && visible.length > 0 && thumbsReady >= Math.min(visible.length, 24)) {
            timed = true;
            console.info("open-timing: first screen of thumbnails at", Date.now(), "(" + (Date.now() - openedAt) + " ms after the open)");
        }
    }

    readonly property var shown: {
        const q = query.trim().toLowerCase();
        let list = Backend.images.filter(i => (!favoritesOnly || i.favorite) && (tagFilter.length === 0 || tagFilter.every(t => (i.tags || []).some(x => x.toLowerCase() === t.toLowerCase()))) && (q === "" || i.name.toLowerCase().includes(q) || (i.tags || []).some(t => t.toLowerCase().includes(q))));
        const order = {
            newest: (a, b) => b.modified - a.modified,
            oldest: (a, b) => a.modified - b.modified,
            name: (a, b) => a.name.localeCompare(b.name),
            random: (a, b) => (hash(a.path) - hash(b.path))
        }[sort];
        // Favorites first, then the chosen order (as the app does).
        return list.slice().sort((a, b) => (b.favorite - a.favorite) || order(a, b));
    }
    readonly property int lastPage: Math.max(1, Math.ceil(shown.length / pageSize))
    readonly property int currentPage: Math.min(Math.max(1, page), lastPage)
    readonly property var visible: shown.slice((currentPage - 1) * pageSize, currentPage * pageSize)
    // Where each wallpaper is shown, by path.
    readonly property var shownOn: {
        const out = {};
        for (const m of Backend.monitors)
            if (m.current)
                (out[m.current] = out[m.current] || []).push(m.name);
        return out;
    }
    readonly property var backdrop: {
        const m = Backend.monitors.find(m => targets.includes(m.name)) || Backend.monitors[0];
        const img = m && Backend.images.find(i => i.path === m.current);
        return img && img.thumb ? "file://" + img.thumb : "";
    }

    function hash(s) {
        let h = 0;
        for (let i = 0; i < s.length; i++)
            h = (h * 31 + s.charCodeAt(i)) | 0;
        return h;
    }
    function goTo(p) {
        page = Math.min(Math.max(1, p), lastPage);
        grid.positionViewAtBeginning();
    }
    // A new filter starts from the first page.
    onQueryChanged: page = 1
    onFavoritesOnlyChanged: page = 1
    onSortChanged: page = 1
    onTagFilterChanged: page = 1

    Connections {
        target: Backend
        function onMonitorsChanged() {
            if (root.targets.length === 0) {
                const f = Backend.monitors.find(m => m.focused) || Backend.monitors[0];
                if (f)
                    root.targets = [f.name];
            }
        }
    }

    /// Whether the window exists. Closed, it's destroyed (its memory with
    /// it) and Quickshell idles; opened, it's built again from the loaded
    /// QML (no new process).
    property bool open: true

    IpcHandler {
        target: "wallpicker"
        function toggle(): void {
            if (!root.open) {
                root.openedAt = Date.now();
                root.thumbsReady = 0;
                root.timed = false;
            }
            root.open = !root.open;
        }
    }

    LazyLoader {
        active: root.open

    FloatingWindow {
        id: win
        title: "wallpicker"
        implicitWidth: 1400
        implicitHeight: 900
        color: Theme.background

        // The wallpaper, big and blurred, behind everything (Backdrop in
        // palette.rs: blur-2xl, saturate-150, 70%).
        Image {
            id: backdropImage
            anchors.fill: parent
            source: root.backdrop
            fillMode: Image.PreserveAspectCrop
            sourceSize.width: 360
            visible: false
        }
        MultiEffect {
            visible: !Theme.noEffects
            anchors.fill: parent
            source: backdropImage
            scale: 1.1
            opacity: 0.7
            blurEnabled: true
            blur: 1
            blurMax: 64
            saturation: 0.5
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // The top bar.
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 56
                color: Theme.a(Theme.background, 0.55)
                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: 1
                    color: Theme.a(Theme.border, 0.6)
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 20
                    anchors.rightMargin: 20
                    spacing: 16

                    // "wallpicker", primary into foreground (bg-clip-text).
                    Item {
                        implicitWidth: title.implicitWidth
                        implicitHeight: title.implicitHeight
                        Text {
                            id: title
                            text: "wallpicker"
                            font.family: Theme.font
                            font.pixelSize: 20
                            font.weight: Font.Black
                            font.letterSpacing: -0.5
                            color: "white"
                            visible: false
                            layer.enabled: true
                        }
                        Rectangle {
                            id: titleFill
                            anchors.fill: title
                            visible: false
                            layer.enabled: true
                            gradient: Gradient {
                                orientation: Gradient.Horizontal
                                GradientStop {
                                    position: 0
                                    color: Theme.primary
                                }
                                GradientStop {
                                    position: 1
                                    color: Theme.a(Theme.foreground, 0.8)
                                }
                            }
                        }
                        MultiEffect {
                            anchors.fill: title
                            source: titleFill
                            maskEnabled: true
                            maskSource: title
                        }
                    }

                    // Tabs.
                    Rectangle {
                        implicitWidth: tabs.implicitWidth + 8
                        implicitHeight: 40
                        radius: 20
                        color: Theme.muted
                        Row {
                            id: tabs
                            anchors.centerIn: parent
                            spacing: 2
                            Repeater {
                                model: ["Local", "Online", "Collections"]
                                Rectangle {
                                    required property string modelData
                                    required property int index
                                    width: tabLabel.implicitWidth + 32
                                    height: 32
                                    radius: 16
                                    color: index === 0 ? Theme.primary : "transparent"
                                    Text {
                                        id: tabLabel
                                        anchors.centerIn: parent
                                        text: parent.modelData
                                        color: parent.index === 0 ? Theme.primaryForeground : Theme.mutedForeground
                                        font.family: Theme.font
                                        font.pixelSize: 14
                                        font.weight: Font.Bold
                                    }
                                }
                            }
                        }
                    }

                    Item {
                        Layout.fillWidth: true
                    }

                    Text {
                        text: "APPLY TO"
                        color: Theme.mutedForeground
                        font.family: Theme.font
                        font.pixelSize: 11
                        font.weight: Font.Bold
                        font.letterSpacing: 1.5
                    }
                    // A pill per monitor: a little screen with its wallpaper,
                    // the name, a check when a click applies there.
                    Repeater {
                        model: Backend.monitors
                        Rectangle {
                            id: pill
                            required property var modelData
                            readonly property bool on: root.targets.includes(modelData.name)
                            readonly property var shownImg: Backend.images.find(i => i.path === modelData.current)
                            implicitWidth: pillRow.implicitWidth + 20
                            implicitHeight: 36
                            radius: 18
                            color: pillMouse.containsMouse ? Theme.muted : Theme.a(Theme.card, 0.6)
                            border.width: on ? 2 : 1
                            border.color: on ? Theme.primary : Theme.a(Theme.foreground, 0.08)
                            RowLayout {
                                id: pillRow
                                anchors.centerIn: parent
                                spacing: 8
                                Rectangle {
                                    implicitWidth: 30
                                    implicitHeight: 20
                                    radius: 4
                                    color: "#18181b"
                                    ClippingRectangle {
                                        anchors.fill: parent
                                        anchors.margins: 2
                                        radius: 2
                                        Image {
                                            anchors.fill: parent
                                            source: pill.shownImg && pill.shownImg.thumb ? "file://" + pill.shownImg.thumb : ""
                                            fillMode: Image.PreserveAspectCrop
                                            sourceSize.width: 64
                                        }
                                    }
                                }
                                Text {
                                    text: pill.modelData.name
                                    color: Theme.foreground
                                    font.family: Theme.font
                                    font.pixelSize: 14
                                    font.weight: Font.Black
                                }
                                Rectangle {
                                    visible: pill.on
                                    implicitWidth: 18
                                    implicitHeight: 18
                                    radius: 9
                                    color: Theme.primary
                                    Icon {
                                        anchors.centerIn: parent
                                        name: "check"
                                        size: 12
                                        color: Theme.primaryForeground
                                    }
                                }
                            }
                            MouseArea {
                                id: pillMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    const t = root.targets.slice();
                                    const i = t.indexOf(pill.modelData.name);
                                    if (i >= 0) {
                                        if (t.length > 1)
                                            t.splice(i, 1);
                                    } else
                                        t.push(pill.modelData.name);
                                    root.targets = t;
                                }
                            }
                        }
                    }
                    // All: a desk with every monitor on it.
                    Rectangle {
                        id: allPill
                        readonly property bool on: root.targets.length === Backend.monitors.length && Backend.monitors.length > 1
                        visible: Backend.monitors.length > 1
                        implicitWidth: allRow.implicitWidth + 20
                        implicitHeight: 36
                        radius: 18
                        color: allMouse.containsMouse ? Theme.muted : Theme.a(Theme.card, 0.6)
                        border.width: on ? 2 : 1
                        border.color: on ? Theme.primary : Theme.a(Theme.foreground, 0.08)
                        RowLayout {
                            id: allRow
                            anchors.centerIn: parent
                            spacing: 8
                            Row {
                                spacing: 1
                                Repeater {
                                    model: Backend.monitors
                                    Rectangle {
                                        required property var modelData
                                        readonly property var shownImg: Backend.images.find(i => i.path === modelData.current)
                                        width: 18
                                        height: 13
                                        radius: 2
                                        color: "#18181b"
                                        ClippingRectangle {
                                            anchors.fill: parent
                                            anchors.margins: 1.5
                                            radius: 1
                                            Image {
                                                anchors.fill: parent
                                                source: parent.parent.shownImg && parent.parent.shownImg.thumb ? "file://" + parent.parent.shownImg.thumb : ""
                                                fillMode: Image.PreserveAspectCrop
                                                sourceSize.width: 48
                                            }
                                        }
                                    }
                                }
                            }
                            Text {
                                text: "All"
                                color: Theme.foreground
                                font.family: Theme.font
                                font.pixelSize: 14
                                font.weight: Font.Black
                            }
                        }
                        MouseArea {
                            id: allMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.targets = Backend.monitors.map(m => m.name)
                        }
                    }
                    Btn {
                        variant: "ghost"
                        icon: "chevron-down"
                    }
                    Btn {
                        variant: "ghost"
                        icon: "history"
                        onClicked: Backend.previous(root.targets)
                    }
                    Btn {
                        variant: "ghost"
                        icon: "settings"
                    }
                }
            }

            // The grid, with the search card floating over its top.
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                GridView {
                    id: grid
                    // grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4,
                    // px-5: a cell is a tile and its 16 px gap.
                    readonly property int columns: Math.max(1, Math.floor(width / (220 + 16)))
                    anchors.fill: parent
                    anchors.leftMargin: 20
                    anchors.rightMargin: 4
                    topMargin: searchCard.height + 12 + 28
                    bottomMargin: 20
                    cellWidth: Math.floor(width / columns)
                    // The tallest card (a row of tags) and the gap.
                    cellHeight: (cellWidth - 16 - 12) * 10 / 16 + 12 + 54 + 16
                    model: root.visible
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: ScrollBar {}
                    delegate: Item {
                        required property var modelData
                        width: grid.cellWidth
                        height: grid.cellHeight
                        Tile {
                            x: 0
                            y: 4
                            width: parent.width - 16
                            img: parent.modelData
                            shownOn: root.shownOn[parent.modelData.path] || []
                            busy: !!Backend.applying[parent.modelData.path]
                            onApply: Backend.apply(img.path, root.targets)
                            onToggleFavorite: Backend.setFavorite(img.path, !img.favorite)
                            onThumbReady: root.thumbsReady++
                        }
                    }
                    footer: Item {
                        width: grid.width - 16
                        height: root.lastPage > 1 ? 80 : 0
                        Pager {
                            visible: root.lastPage > 1
                            anchors.centerIn: parent
                            current: root.currentPage
                            last: root.lastPage
                            onGoTo: p => root.goTo(p)
                        }
                    }
                }

                // The floating search card (rounded-2xl, bg-background/70,
                // backdrop blur of what's under it).
                Item {
                    id: searchCard
                    x: 12
                    y: 12
                    width: parent.width - 24
                    height: cardContent.implicitHeight + 18

                    RectangularShadow {
                        anchors.fill: parent
                        radius: 22
                        offset.y: 8
                        blur: 20
                        color: Qt.rgba(0, 0, 0, 0.15)
                    }
                    ClippingRectangle {
                        anchors.fill: parent
                        radius: 22
                        color: "transparent"
                        ShaderEffectSource {
                            id: under
                            live: !Theme.noEffects
                            anchors.fill: parent
                            sourceItem: grid
                            sourceRect: Qt.rect(searchCard.x - grid.x, searchCard.y - grid.y, searchCard.width, searchCard.height)
                            visible: false
                        }
                        MultiEffect {
                            visible: !Theme.noEffects
                            anchors.fill: parent
                            source: under
                            blurEnabled: true
                            blur: 1
                            blurMax: 48
                        }
                        Rectangle {
                            anchors.fill: parent
                            color: Theme.a(Theme.background, 0.7)
                        }
                    }
                    Rectangle {
                        anchors.fill: parent
                        radius: 22
                        color: "transparent"
                        border.width: 1
                        border.color: Theme.a(Theme.border, 0.6)
                    }

                    ColumnLayout {
                        id: cardContent
                        x: 16
                        y: 10
                        width: parent.width - 32
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            // The search box.
                            Rectangle {
                                Layout.preferredWidth: Math.min(320, Math.max(160, cardContent.width - 760))
                                implicitHeight: 36
                                radius: 18
                                color: Theme.a(Theme.input, 0.3)
                                border.width: search.activeFocus ? 2 : 1
                                border.color: search.activeFocus ? Theme.a(Theme.primary, 0.7) : Theme.input
                                Icon {
                                    x: 10
                                    anchors.verticalCenter: parent.verticalCenter
                                    name: "search"
                                    color: Theme.mutedForeground
                                }
                                TextInput {
                                    id: search
                                    x: 34
                                    width: parent.width - 44
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: Theme.foreground
                                    selectionColor: Theme.a(Theme.primary, 0.4)
                                    font.family: Theme.font
                                    font.pixelSize: 14
                                    clip: true
                                    onTextChanged: root.query = text
                                    Text {
                                        visible: !search.text
                                        text: "Filter by name or tag"
                                        color: Theme.mutedForeground
                                        font: search.font
                                    }
                                }
                            }
                            Btn {
                                icon: "star"
                                text: "Favorites"
                                active: root.favoritesOnly
                                onClicked: root.favoritesOnly = !root.favoritesOnly
                            }
                            Btn {
                                id: sortBtn
                                minWidth: 144
                                text: ({
                                        newest: "Newest",
                                        oldest: "Oldest",
                                        name: "Name",
                                        random: "Random order"
                                    })[root.sort]
                                Icon {
                                    name: "chevron-down"
                                    color: Theme.mutedForeground
                                }
                                onClicked: sortMenu.open()
                                Menu {
                                    id: sortMenu
                                    y: sortBtn.height + 6
                                    Repeater {
                                        model: [["newest", "Newest"], ["oldest", "Oldest"], ["name", "Name"], ["random", "Random order"]]
                                        MenuItem {
                                            required property var modelData
                                            text: modelData[1]
                                            onTriggered: root.sort = modelData[0]
                                        }
                                    }
                                }
                            }
                            Btn {
                                minWidth: 112
                                text: "Color"
                                Row {
                                    spacing: -5
                                    Repeater {
                                        model: ["#ef4444", "#eab308", "#3b82f6"]
                                        Rectangle {
                                            required property string modelData
                                            width: 12
                                            height: 12
                                            radius: 6
                                            color: modelData
                                            border.width: 2
                                            border.color: Theme.card
                                        }
                                    }
                                }
                            }
                            Btn {
                                icon: "grid"
                                text: "Flat"
                            }
                            Btn {
                                icon: "shuffle"
                                text: "Random"
                                onClicked: {
                                    if (root.shown.length > 0)
                                        Backend.apply(root.shown[Math.floor(Math.random() * root.shown.length)].path, root.targets);
                                }
                            }
                            Text {
                                Layout.minimumWidth: 80
                                text: root.shown.length + " images"
                                color: Theme.mutedForeground
                                font.family: Theme.font
                                font.pixelSize: 14
                            }
                            Item {
                                Layout.fillWidth: true
                            }
                            Btn {
                                variant: "ghost"
                                icon: "ellipsis"
                            }
                        }

                        // Tags: the row, or every tag when expanded.
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Text {
                                Layout.alignment: Qt.AlignTop
                                Layout.topMargin: 6
                                text: "Tags"
                                color: Theme.mutedForeground
                                font.family: Theme.font
                                font.pixelSize: 14
                            }
                            Flow {
                                Layout.fillWidth: true
                                Layout.preferredHeight: root.tagsExpanded ? implicitHeight : 28
                                spacing: 8
                                clip: true
                                Repeater {
                                    model: Backend.tags
                                    Rectangle {
                                        id: tagChip
                                        required property var modelData
                                        readonly property bool on: root.tagFilter.includes(modelData.name)
                                        width: tagText.implicitWidth + 24
                                        height: 28
                                        radius: 14
                                        color: on ? Theme.primary : (tagMouse.containsMouse ? Theme.muted : Theme.a(Theme.card, 0.8))
                                        border.width: on ? 0 : 1
                                        border.color: Theme.input
                                        Text {
                                            id: tagText
                                            anchors.centerIn: parent
                                            text: tagChip.modelData.name + "  " + tagChip.modelData.count
                                            color: tagChip.on ? Theme.primaryForeground : Theme.foreground
                                            font.family: Theme.font
                                            font.pixelSize: 13
                                            font.weight: Font.Bold
                                        }
                                        MouseArea {
                                            id: tagMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                const t = root.tagFilter.slice();
                                                const i = t.indexOf(tagChip.modelData.name);
                                                if (i >= 0)
                                                    t.splice(i, 1);
                                                else
                                                    t.push(tagChip.modelData.name);
                                                root.tagFilter = t;
                                            }
                                        }
                                    }
                                }
                            }
                            Btn {
                                Layout.alignment: Qt.AlignTop
                                variant: "ghost"
                                text: root.tagsExpanded ? "Less" : "Expand"
                                implicitHeight: 28
                                onClicked: root.tagsExpanded = !root.tagsExpanded
                            }
                        }
                    }
                }

                Text {
                    visible: !Backend.loading && root.shown.length === 0
                    anchors.centerIn: parent
                    text: "No wallpapers match."
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 14
                }
            }
        }
    }
    }
}
