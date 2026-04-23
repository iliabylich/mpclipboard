import Cocoa

class Clipboard {
    var pasteboard: NSPasteboard = NSPasteboard.general
    var lastChangeCount: Int

    init() {
        lastChangeCount = pasteboard.changeCount
    }

    func isChanged() -> Bool {
        let newChangeCount = pasteboard.changeCount
        if newChangeCount == lastChangeCount {
            return false
        }
        lastChangeCount = newChangeCount
        return true
    }

    func pollOnce() -> String? {
        if !isChanged() {
            return nil
        }

        return pasteboard.string(forType: .string)
    }

    func startPolling(onCopy: @escaping (String) -> Void) -> Timer {
        let timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [self] _ in
            if let copiedText = pollOnce() {
                onCopy(copiedText)
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        return timer
    }

    func writeText(_ text: String) {
        pasteboard.declareTypes([.string], owner: nil)
        pasteboard.setString(text, forType: .string)
    }
}
