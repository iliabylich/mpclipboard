import Cocoa
import ServiceManagement
import UserNotifications

@main
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let mpclipboard: MPClipboard = MPClipboard()
    private var mpclipboardSource: DispatchSourceRead?

    private let clipboard: Clipboard = Clipboard()
    private var clipboardTimer: Timer?

    private let tray: Tray = Tray()

    static func main() {
        let app = NSApplication.shared
        let delegate = AppDelegate()
        app.delegate = delegate
        app.run()
    }

    func applicationDidFinishLaunching(_ aNotification: Notification) {
        ProcessInfo.processInfo.disableAutomaticTermination("MPClipboard runs continuously as a menu bar clipboard sync agent")
        ProcessInfo.processInfo.disableSuddenTermination()

        #if !DEBUG
        registerLoginItem()
        #endif

        Task {
            do {
                if try await UNUserNotificationCenter.current().requestAuthorization(options: [.alert]) {
                    log.notice("Got permission to send notifications")
                } else {
                    log.error("Failed to get permission to send notifications")
                }
            } catch {
                log.error("Error requesting notification permission: \(error, privacy: .public)")
            }
        }

        let source = DispatchSource.makeReadSource(fileDescriptor: mpclipboard.fd(), queue: .main)
        source.setEventHandler { [weak self] in
            MainActor.assumeIsolated {
                self?.readMPClipboard()
            }
        }
        source.resume()
        mpclipboardSource = source

        clipboardTimer = clipboard.startPolling(onCopy: { text in
            if self.mpclipboard.pushText(text) == .pushed {
                self.tray.pushSent(text)
            }
        })
    }

    @objc
    func quit() {
        log.notice("Quitting...")
        self.clipboardTimer?.invalidate()
        NSApp.terminate(self)
    }

    private func registerLoginItem() {
        let service = SMAppService.mainApp
        guard service.status == .notRegistered else {
            return
        }
        do {
            try service.register()
        } catch {
            log.error("Error registering login item: \(error, privacy: .public)")
        }
    }

    private func readMPClipboard() {
        guard let output = mpclipboard.read() else {
            return
        }

        if let connectivity = output.connectivity {
            tray.setConnectivity(connectivity)
        }

        if let text = output.text {
            clipboard.writeText(text)
            tray.pushReceived(text)
            showNotification(text)
        }
    }

    private func showNotification(_ text: String) {
        let content = UNMutableNotificationContent()
        content.title = "MPClipboard"
        content.body = text

        let request = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
        Task {
            do {
                try await UNUserNotificationCenter.current().add(request)
            } catch {
                log.error("Error showing notification: \(error, privacy: .public)")
            }
        }
    }
}
