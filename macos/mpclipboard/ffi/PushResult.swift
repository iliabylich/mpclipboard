enum PushResult {
    case sent
    case droppedAsStale
    case error

    static func from(_ push_result: mpclipboard_PushResult) -> Self {
        switch (push_result) {
        case MPCLIPBOARD_PUSH_RESULT_SENT:
            return .sent
        case MPCLIPBOARD_PUSH_RESULT_DROPPED_AS_STALE:
            return .droppedAsStale
        case MPCLIPBOARD_PUSH_RESULT_ERROR:
            return .error
        default:
            fatalError("unnsupported PushResult")
        }
    }
}
