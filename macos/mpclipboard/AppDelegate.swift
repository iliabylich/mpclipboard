import Cocoa
import Carbon
import SwiftUI
import UserNotifications

class AppDelegate: NSObject, NSApplicationDelegate {
    var mpclipboard: MPClipboard?
    var fdwatcher: FDWatcher?
    var mpclipboardTimer: Timer?

    var clipboard: Clipboard = Clipboard()
    var clipboardTimer: Timer?

    var tray: Tray = Tray()

    func applicationDidFinishLaunching(_ aNotification: Notification) {
        // Hide Dock icon
        NSApp.setActivationPolicy(.accessory)

        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge]) { granted, error in
            if granted {
                print("Got permission to send notifications")
            } else {
                fputs("Failed to get permission to send notifications", stderr)
                if let error = error {
                    fputs("Error showing notification: \(error)", stderr)
                }
            }
        }

        mpclipboard = MPClipboard(self)
        guard let fd = mpclipboard?.fd() else {
            fatalError("can't get FD")
        }

        fdwatcher = FDWatcher(fd, onReadable: {
            guard let output = self.mpclipboard?.read() else {
                return
            }

            DispatchQueue.main.async {
                switch output {
                case .connectivityChanged(let connectivity):
                    self.tray.setConnectivity(connectivity)
                case .newText(let text):
                    self.clipboard.writeText(text)
                    self.tray.pushReceived(text)
                    self.showNotification(text)
                case .newBinary(let data):
                    print("TODO: received data: \(data)")
                }
            }
        })

        clipboardTimer = clipboard.startPolling(onCopy: { text in
            if self.mpclipboard?.pushText(text) ?? false {
                self.tray.pushSent(text)
            }
        })
    }

    @objc
    func quit() {
        print("Quitting...")
        self.clipboardTimer?.invalidate()
        self.mpclipboardTimer?.invalidate()
        NSApp.terminate(self)
    }

    func showNotification(_ text: String) {
        let content = UNMutableNotificationContent()
        content.title = "MPClipboard"
        content.body = text

        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: 1, repeats: false)

        let request = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: trigger)

        UNUserNotificationCenter.current().add(request) { error in
            if let error = error {
                fputs("Error showing notification: \(error)", stderr)
            }
        }
    }
}
