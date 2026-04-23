import Cocoa

class Tray {
    var redImage: NSImage? = NSImage(named: "red")
    var greenImage: NSImage? = NSImage(named: "green")
    var yellowImage: NSImage? = NSImage(named: "yellow")

    var statusItem: NSStatusItem
    var trayButton: NSStatusBarButton?

    static let MAX_ITEMS_COUNT: Int = 5 // 4 clips + Quit

    init() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = statusItem.button {
            button.image = redImage
            trayButton = button
        }

        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "Quit", action: #selector(AppDelegate.quit), keyEquivalent: "q"))
        statusItem.menu = menu
    }

    func setConnectivity(_ connectivity: Connectivity) {
        trayButton?.image =
            switch connectivity {
            case .connected:
                greenImage
            case .connecting:
                yellowImage
            case .disconnected:
                redImage
            }
    }

    func pushSent(_ text: String) {
        push("S \(text)")
    }

    func pushReceived(_ text: String) {
        push("R \(text)")
    }

    func push(_ text: String) {
        guard let menu = statusItem.menu else {
            return;
        }

        while menu.items.count >= Tray.MAX_ITEMS_COUNT {
            menu.items.remove(at: menu.items.count - 2)
        }

        let item = NSMenuItem(title: text, action: nil, keyEquivalent: "");
        item.isEnabled = false
        menu.insertItem(item, at: 0)
    }
}

func currentThreadID() -> UInt64 {
    var tid: UInt64 = 0
    pthread_threadid_np(nil, &tid)
    return tid
}
