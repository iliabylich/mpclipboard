import Foundation

enum Connectivity {
    case connecting
    case connected
    case disconnected

    static func from(_ connectivity: mpclipboard_Connectivity) -> Self {
        switch connectivity {
        case MPCLIPBOARD_CONNECTIVITY_CONNECTING:
            .connecting
        case MPCLIPBOARD_CONNECTIVITY_CONNECTED:
            .connected
        case MPCLIPBOARD_CONNECTIVITY_DISCONNECTED:
            .disconnected
        default:
            fatalError("unsupported Connectivity")
        }
    }
}

struct Output {
    let connectivity: Connectivity?
    let text: String?

    static func from(_ output: mpclipboard_Output) -> Self? {
        if output.error {
            fatalError("MPClipboard return error from .read()")
        }

        let connectivity = output.has_connectivity ? Connectivity.from(output.connectivity) : nil
        let text = output.text.ptr != nil ? string(output.text) : nil

        if connectivity == nil && text == nil {
            return nil
        }
        return Output(connectivity: connectivity, text: text)
    }

    private static func string(_ text: mpclipboard_OwnedString) -> String {
        let data = Data(bytes: text.ptr!, count: text.len)
        mpclipboard_drop_str(text)

        guard let text = String(data: data, encoding: .utf8) else {
            fatalError("non-utf8 new text in output")
        }

        return text
    }
}

enum PushResult {
    case pushed
    case dropped
}

final class MPClipboard {
    private let handle: OpaquePointer

    init() {
#if DEBUG
        log.notice("Debug build, using local config")
        guard let handle = mpclipboard_new_with_local_config() else {
            fatalError("NULL mpclipboard")
        }
#else
        log.notice("Release build, using config from XDG dir")
        guard let handle = mpclipboard_new_with_xdg_config() else {
            fatalError("NULL mpclipboard")
        }
#endif

        self.handle = handle
    }

    deinit {
        mpclipboard_drop(handle)
    }

    func fd() -> Int32 {
        mpclipboard_get_fd(handle)
    }

    func pushText(_ text: String) -> PushResult {
        text.utf8CString.withUnsafeBufferPointer { bytes in
            let text = mpclipboard_BorrowedString(ptr: bytes.baseAddress, len: bytes.count - 1)
            let pushResult = mpclipboard_push_text(handle, text)
            switch pushResult {
            case MPCLIPBOARD_PUSH_RESULT_PUSHED:
                return .pushed
            case MPCLIPBOARD_PUSH_RESULT_DROPPED:
                return .dropped
            case MPCLIPBOARD_PUSH_RESULT_ERROR:
                fatalError("failed to push text to MPClipboard")
            default:
                fatalError("unsupported push result")
            }
        }
    }

    func read() -> Output? {
        Output.from(mpclipboard_read(handle))
    }
}
