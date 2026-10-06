import Cocoa
import ServiceManagement
import UserNotifications

let app = NSApplication.shared

let mpclipboard: MPClipboard = MPClipboard()
let clipboard: Clipboard = Clipboard()
let tray: Tray = Tray()

func registerLoginItem() {
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

@MainActor
func readMPClipboard() {
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

func showNotification(_ text: String) {
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

let mpclipboardSource = DispatchSource.makeReadSource(fileDescriptor: mpclipboard.fd(), queue: .main)
mpclipboardSource.setEventHandler {
    MainActor.assumeIsolated {
        readMPClipboard()
    }
}
mpclipboardSource.resume()

let clipboardTimer = clipboard.startPolling(onCopy: { text in
    if mpclipboard.pushText(text) == .pushed {
        tray.pushSent(text)
    }
})

app.run()
