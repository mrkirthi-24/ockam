import SwiftUI

class StateContainer {
    static var shared = StateContainer()

    var state = ApplicationState(
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
        print("update: \(state)")
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

func bringInFront() {
    NSApplication.shared.activate(ignoringOtherApps: true)
}

func copyToClipboard(_ text: String) {
    let pasteboard = NSPasteboard.general
    pasteboard.declareTypes([.string], owner: nil)
    pasteboard.setString(text, forType: .string)
}

@main
struct OckamApp: App {
    @State var state: ApplicationState = StateContainer.shared.state;
    
    var body: some Scene {
        MenuBarExtra
        {
            MainView(state: $state)
                .onAppear(perform: {
                    StateContainer.shared.callback(callback: { state in
                        self.state = state
                    })
                })
                .onOpenURL(perform: { url in
                    let urlComponents = URLComponents(url: url, resolvingAgainstBaseURL: false)
                    if let path = urlComponents?.path {
                        let segments = path.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
                        if segments.count >= 2 {
                            if segments[0] == "invitations" && segments[1] == "accept" {
                                accept_invitation(segments[2])
                                return
                            }
                        }
                        print("Ignoring URL \(url)")
                    }
                })
        } label: {
            Image("MenuBarIcon")
                .renderingMode(.template)
        }
        .menuBarExtraStyle(.window)
        .commandsRemoved()
        
        Window("Create a service", id: "create-service") {
            CreateServiceView()
        }
        .commandsRemoved()
        .windowResizability(.contentSize)
        
        WindowGroup("Share a service", id: "share-service", for: LocalService.ID.self) { $localServiceId in
            ShareServiceView(localService: StateContainer.shared.state.getLocalService(
                localServiceId.unsafelyUnwrapped
            ).unsafelyUnwrapped)
        }
        .windowResizability(.contentSize)
        .commandsRemoved()
    }
    
    init() {
        swift_initialize_application()
    }
}

