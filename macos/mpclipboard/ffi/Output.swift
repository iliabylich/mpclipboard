import Foundation

enum Output {
    case connectivityChanged(Connectivity)
    case newText(String)
    case error

    static func from(_ output: mpclipboard_Output) -> Self? {
        switch (output.tag) {
        case MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED:
            return .connectivityChanged(Connectivity(output.CONNECTIVITY_CHANGED.connectivity))
        case MPCLIPBOARD_OUTPUT_NEW_TEXT:
            let ptr = output.NEW_TEXT.ptr!
            let len = output.NEW_TEXT.len
            let data = Data(bytes: ptr, count: Int(len))
            if let text = String(data: data, encoding: .utf8) {
                free(ptr)
                return .newText(text)
            } else {
                fatalError("non-utf8 new text in output")
            }
        case MPCLIPBOARD_OUTPUT_IGNORE:
            return nil
        case MPCLIPBOARD_OUTPUT_ERROR:
            return .error
        default:
            fatalError("unnsupported Output")
        }
    }
}
