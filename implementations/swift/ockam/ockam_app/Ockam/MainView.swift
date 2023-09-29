import SwiftUI

struct MainView: View {
    @State private var isOn : Bool = true
    @Binding dynamic var state : ApplicationState
    
    var body: some View {
        NavigationStack {
            VStack(alignment: .leading) {
                HStack {
                    VStack(alignment: .leading) {
                        Text("Ockam").font(.headline)
                        switch state.orchestrator_status {
                            case .Disconnected:
                                Text("Disconnected from the Orchestrator").font(.subheadline)
                            case .Connected:
                                Text("Connected to Orchestrator").font(.subheadline)
                            case .Connecting:
                                Text("Connecting to Orchestrator").font(.subheadline)
                            case .WaitingForToken:
                                Text("Waiting for token").font(.subheadline)
                            case .RetrievingSpace:
                                Text("Retrieving space").font(.subheadline)
                            case .RetrievingProject:
                                Text("Retrieving project").font(.subheadline)
                        }
                    }
                    Spacer()
                    Toggle(isOn: $isOn) {
                    }.toggleStyle(SwitchToggleStyle()).disabled(/*@START_MENU_TOKEN@*/true/*@END_MENU_TOKEN@*/)
                }.padding(5)
                
                if state.enrolled {
                    Group {
                        Divider()
                        HStack {
                            Avatar(url: state.enrollmentImage)
                            VStack(alignment: .leading) {
                                if let name = state.enrollmentName {
                                    Text(verbatim: name).font(.title3)
                                }
                                HStack {
                                    VStack(alignment: .trailing) {
                                        Text("Email:").foregroundColor(.primary.opacity(0.7))
                                        if state.enrollmentGithubUser != nil {
                                            Text("GitHub:").foregroundColor(.primary.opacity(0.7))
                                        }
                                    }
                                    VStack(alignment: .leading) {
                                        Text(verbatim: state.enrollmentEmail.unsafelyUnwrapped)
                                        if let github = state.enrollmentGithubUser {
                                            Text(verbatim: github)
                                        }
                                    }
                                }
                            }
                            
                            Spacer()
                        }
                    }
                } else {
                    ClickableMenuEntry(text: "Enroll", icon: "arrow.right.square", action: {
                        enroll_user()
                    })
                }
                
                if state.enrolled {
                    Group {
                        Divider()
                        Text("Your services").font(.body).bold().foregroundColor(.primary.opacity(0.7))
                        ClickableMenuEntry(text: "Create Service", icon: "plus")
                        ForEach(state.localServices){ localService in
                            LocalServiceView(localService: localService)
                        }
                    }
                    
                    Divider()
                    Text("Services shared with you").font(.body).bold().foregroundColor(.primary.opacity(0.7))
                    
                    ForEach(state.groups){ group in
                        NavigationLink {
                            ServiceGroupView(group: group)
                        } label: {
                            ServiceGroupButton(group: group)
                        }.buttonStyle(.plain)
                    }
                    
                }
                
                Group {
                    Divider()
                    VStack(spacing: 0) {
                        ClickableMenuEntry(text: "Reset", icon: "arrow.counterclockwise", action: {
                            
                        })
                        ClickableMenuEntry(text: "Documentation", icon: "book", action: {
                          if let url = URL(string: "https://docs.ockam.io") {
                              NSWorkspace.shared.open(url)
                          }
                        })
                        ClickableMenuEntry(text: "Quit", icon: "power", action: {
                            shutdown_application();
                            NSApplication.shared.terminate(self);
                        })
                    }
                }
                
                Spacer()
            }
            .padding(.horizontal, 6)
            .padding(.top, 6)
            .frame(minWidth: 300)
            
        }
    }
}


struct MainView_Previews: PreviewProvider {
    @State static var state = swift_demo_application_state();
    
    static var previews: some View {
        MainView(state: $state)
            .frame(width: 300)
    }
}


struct Avatar: View {
    @State var url: String?;
    @State var placeholder = "person";
    @State var size: CGFloat = 64;
    
    var body: some View {
        if let url = url {
            AsyncImage(
                url: URL(string: url),
                content: { image in
                    image.resizable()
                        .aspectRatio(contentMode: .fit)
                        .clipShape(Circle())
                },
                placeholder: {
                    Image(systemName: placeholder)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(maxWidth: size, maxHeight: size)
                }
            ).frame(minWidth: size, maxWidth: size, minHeight: size, maxHeight: size)
        } else {
            Image(systemName: placeholder)
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(maxWidth: size, maxHeight: size)
        }
    }
}

struct ServiceGroupView: View {
    var group: ServiceGroup

    var body: some View {
        VStack {
            HStack {
                Avatar(url: group.imageUrl, size: 32)
                VStack(alignment: .leading) {
                    if let name = group.name {
                        Text(verbatim: name)
                    }
                    Text(verbatim: group.email)
                }
            }
            ForEach(group.invites) { invite in
                IncomingInvite(invite: invite)
            }
            ForEach(group.incomingServices) { service in
                RemoteServiceView(service: service)
            }
            Spacer()
        }
        .padding(6)
        .frame(minWidth: 320)
    }
}

struct ServiceGroupButton: View {
    @State private var isHovered = false
    @State private var isOpen = false
    var group: ServiceGroup
    
    var body: some View {
        HStack {
            Avatar(url: group.imageUrl, size: 32)
            VStack(alignment: .leading) {
                if let name = group.name {
                    Text(verbatim: name)
                }
                Text(verbatim: group.email)
            }
            Spacer()
            Image(systemName: "chevron.right")
                .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
        }.onHover { hover in
            isHovered = hover
        }
        .padding(3)
        .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
        .cornerRadius(4)
    }
}

struct RemoteServiceView: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @State var service: Service
    
    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: "circle")
                    .foregroundColor(service.available ? .green : .red)
                    .frame(maxWidth: 16, maxHeight: 16)
                
                VStack(alignment: .leading) {
                    Text(service.sourceName).font(.title3)
                    if let scheme = service.scheme {
                        Text(verbatim: scheme+service.address+":"+String(service.port)).font(.caption)
                    }
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
            }
            .padding(3)
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation {
                    isOpen = !isOpen
                }
            }
            .onHover { hover in
                isHovered = hover
            }
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .cornerRadius(4)
            
            if isOpen {
                VStack(spacing: 0) {
                    ClickableMenuEntry(text: "Open http://localhost:1234")
                    ClickableMenuEntry(text: "Copy localhost:1234")
                    ClickableMenuEntry(text: "Delete")
                }
            }
        }
    }
}

struct LocalServiceView: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @State var localService: LocalService
    
    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: "circle")
                    .foregroundColor(localService.available ? .green : .red)
                    .frame(maxWidth: 16, maxHeight: 16)
                VStack(alignment: .leading) {
                    Text(verbatim: localService.name).font(.title3)
                    let address = localService.address + ":" + String(localService.port);
                    Text(verbatim: address).font(.caption)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
            }
            .padding(3)
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation {
                    isOpen = !isOpen
                }            }
            .onHover { hover in
                isHovered = hover
            }
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .cornerRadius(4)
            
            if isOpen {
                VStack(spacing: 0) {
                    if let scheme = localService.scheme {
                        let url = "Open " + scheme + localService.address + String(localService.port)
                        ClickableMenuEntry(text: url)
                    }
                    ClickableMenuEntry(text: "Modify")
                    ClickableMenuEntry(text: "Invite")
                    ClickableMenuEntry(text: "Delete")
                }
            }
        }
    }
}

struct IncomingInvite: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @State var invite: Invite
    
    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: "envelope")
                    .frame(maxWidth: 16, maxHeight: 16)
                VStack(alignment: .leading) {
                    Text(verbatim: invite.serviceName).font(.title3)
                    if let scheme = invite.serviceScheme {
                        Text(verbatim: scheme).font(.caption)
                    }
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
            }
            .padding(3)
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation {
                    isOpen = !isOpen
                }
            }
            .onHover { hover in
                isHovered = hover
            }
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .cornerRadius(4)
            
            if isOpen {
                VStack(spacing: 0) {
                    ClickableMenuEntry(text: "Accept")
                    ClickableMenuEntry(text: "Reject")
                }
            }
        }
    }
}

struct ClickableMenuEntry: View {
    @State private var isHovered = false
    
    @State var text: String
    @State var icon: String = ""
    @State var action: (() -> Void)? = nil
    

    var body: some View {
        HStack {
            if icon != "" {
                Image(systemName: icon)
                    .frame(minWidth: 16, maxWidth: 16)
            }
            Text(verbatim: text)
            Spacer()
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
        .buttonStyle(PlainButtonStyle())
        .cornerRadius(4)
        .contentShape(Rectangle())
        .onHover { hover in
            isHovered = hover
        }
        .onTapGesture {
            if let action = action {
                action()
            }
        }
    }
}
