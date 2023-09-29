import SwiftUI

class StateContainer: ObservableObject {
    static var shared = StateContainer()

    @Published var state: ApplicationState = ApplicationState(
        enrolled: false,
        orchestrator_status: OrchestratorStatus.Disconnected,
        enrollmentName: nil,
        enrollmentEmail: nil,
        enrollmentImage: nil,
        enrollmentGithubUser: nil,
        localServices: [],
        groups: []
    )
    
    func update(state: ApplicationState) {
        print("new state! \(state)")
        self.state = state
        if let callback = self.callback {
            callback(state)
        }
    }

    var callback: ((ApplicationState) -> Void)?
    func callback(callback: @escaping (ApplicationState) -> Void) {
        self.callback = callback
        callback(state)
    }
}

@main
struct OckamApp: App {
    @State var state: ApplicationState = StateContainer.shared.state;

    var body: some Scene {
        @ObservedObject var stateContainer = StateContainer.shared
        MenuBarExtra(
            "App Menu Bar Extra", systemImage: "star")
        {
            // This scroll view serves as a workaround for broken window resize
            ScrollView(.vertical) {
                MainView(state: $state)
            }
            .scrollIndicators(.automatic)
            .fixedSize(horizontal: true, vertical: false)
            .onAppear(perform: {
                StateContainer.shared.callback(callback: { state in
                    self.state = state
                })
            })

        }
        .menuBarExtraStyle(.window)
    }
    
    init() {
        swift_initialize_application()
    }


}
