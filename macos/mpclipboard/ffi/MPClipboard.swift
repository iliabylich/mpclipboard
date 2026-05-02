import Cocoa

class MPClipboard {
    var mpclipboard: OpaquePointer?
    var app: NSApplicationDelegate

    init(_ app: NSApplicationDelegate) {
        mpclipboard_init()

#if DEBUG
        puts("Debug build, using local config")
        var option = MPCLIPBOARD_CONFIG_READ_OPTION_FROM_LOCAL_FILE
#else
        puts("Release build, using config from XDG dir")
        var option = MPCLIPBOARD_CONFIG_READ_OPTION_FROM_XDG_CONFIG_DIR
#endif

        self.app = app

        guard let config = mpclipboard_config_read(option) else {
            fatalError("NULL config")
        }

        guard let context = mpclipboard_context_new(config) else {
            fatalError("NULL context")
        }

        guard let mpclipboard = mpclipboard_new(context) else {
            fatalError("NULL mpclipboard")
        }

        self.mpclipboard = mpclipboard
    }

    func fd() -> Int32 {
        return mpclipboard_get_fd(mpclipboard)
    }

    func pushText(_ text: String) -> PushResult {
        text.withCString { ptr in
            return PushResult.from(mpclipboard_push_text1(mpclipboard, ptr))
        }
    }

    func read() -> Output? {
        return Output.from(mpclipboard_read(mpclipboard))
    }
}
