enum Connectivity {
case connecting
case connected
case disconnected

    init(_ connectivity: mpclipboard_Connectivity) {
        switch connectivity {
        case MPCLIPBOARD_CONNECTIVITY_CONNECTING:
            self = .connecting
        case MPCLIPBOARD_CONNECTIVITY_CONNECTED:
            self = .connected
        case MPCLIPBOARD_CONNECTIVITY_DISCONNECTED:
            self = .disconnected
        default:
            fatalError("unnsupported Connectivity")
        }
    }
}
