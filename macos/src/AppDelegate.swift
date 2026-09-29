import Cocoa
import UserNotifications

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let mpclipboard: MPClipboard = MPClipboard()
    private var mpclipboardSource: DispatchSourceRead?

    private let clipboard: Clipboard = Clipboard()
    private var clipboardTimer: Timer?

    private let tray: Tray = Tray()

    func applicationDidFinishLaunching(_ aNotification: Notification) {
        ProcessInfo.processInfo.disableAutomaticTermination("MPClipboard runs continuously as a menu bar clipboard sync agent")
        ProcessInfo.processInfo.disableSuddenTermination()

        UNUserNotificationCenter.current().requestAuthorization(options: [.alert]) { granted, error in
            if granted {
                log.notice("Got permission to send notifications")
            } else {
                log.error("Failed to get permission to send notifications")
                if let error = error {
                    log.error("Error requesting notification permission: \(error, privacy: .public)")
                }
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
        UNUserNotificationCenter.current().add(request) { error in
            if let error = error {
                log.error("Error showing notification: \(error, privacy: .public)")
            }
        }
    }
}
