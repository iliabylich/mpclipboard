import Cocoa

@MainActor
final class Clipboard {
    private let pasteboard: NSPasteboard = NSPasteboard.general
    private var lastChangeCount: Int

    init() {
        lastChangeCount = pasteboard.changeCount
    }

    private func isChanged() -> Bool {
        let newChangeCount = pasteboard.changeCount
        if newChangeCount == lastChangeCount {
            return false
        }
        lastChangeCount = newChangeCount
        return true
    }

    private func pollOnce() -> String? {
        isChanged() ? pasteboard.string(forType: .string) : nil
    }

    func startPolling(onCopy: @escaping @MainActor (String) -> Void) -> Timer {
        let timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [self] _ in
            MainActor.assumeIsolated {
                if let copiedText = pollOnce() {
                    onCopy(copiedText)
                }
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
