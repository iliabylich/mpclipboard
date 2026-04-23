import Foundation
import Dispatch

final class FDWatcher {
    typealias Callback = () -> Void

    private let fd: Int32
    private let source: DispatchSourceRead
    private let onReadable: Callback

    init(
        _ fd: Int32,
        queue: DispatchQueue = .global(),
        onReadable: @escaping Callback
    ) {
        self.fd = fd
        self.onReadable = onReadable
        self.source = DispatchSource.makeReadSource(fileDescriptor: fd, queue: queue)

        source.setEventHandler { [weak self] in
            guard let self else { return }
            self.onReadable()
        }

        source.resume()
    }
}
