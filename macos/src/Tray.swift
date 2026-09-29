import Cocoa

@MainActor
final class Tray {
    private let redImage: NSImage? = NSImage(named: "red")
    private let greenImage: NSImage? = NSImage(named: "green")
    private let yellowImage: NSImage? = NSImage(named: "yellow")

    private let statusItem: NSStatusItem

    private static let maxItemsCount: Int = 5 // 4 clips + Quit

    init() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.button?.image = redImage

        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "Quit", action: #selector(AppDelegate.quit), keyEquivalent: "q"))
        statusItem.menu = menu
    }

    func setConnectivity(_ connectivity: Connectivity) {
        statusItem.button?.image =
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

    private func push(_ text: String) {
        guard let menu = statusItem.menu else {
            return
        }

        while menu.items.count >= Tray.maxItemsCount {
            menu.items.remove(at: menu.items.count - 2)
        }

        let item = NSMenuItem(title: text, action: nil, keyEquivalent: "")
        item.isEnabled = false
        menu.insertItem(item, at: 0)
    }
}
