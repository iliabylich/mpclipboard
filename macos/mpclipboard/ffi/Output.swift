import Foundation

enum Output {
    case connectivityChanged(Connectivity)
    case newText(String)
    case newBinary(Data)

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
        case MPCLIPBOARD_OUTPUT_NEW_BINARY:
            let ptr = output.NEW_BINARY.ptr!
            let len = output.NEW_BINARY.len
            let data = Data(bytes: ptr, count: len)
            free(ptr)
            return .newBinary(data)
        case MPCLIPBOARD_OUTPUT_INTERNAL:
            return nil
        default:
            fatalError("unnsupported Output")
        }
    }
}
