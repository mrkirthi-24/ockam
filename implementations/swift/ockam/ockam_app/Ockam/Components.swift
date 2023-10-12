import SwiftUI

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
            ).frame(width: size, height: size)
        } else {
            Image(systemName: placeholder)
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(width: size, height: size)
        }
    }
}

struct ServiceGroupView: View {
    @ObservedObject var group: ServiceGroup

    var body: some View {
        VStack {
            HStack {
                Spacer()
                Avatar(url: group.imageUrl, size: 32)
                VStack(alignment: .leading) {
                    if let name = group.name {
                        Text(verbatim: name)
                    }
                    Text(verbatim: group.email)
                }
                Spacer()
            }
            ForEach(group.invites) { invite in
                IncomingInvite(invite: invite)
            }
            ForEach(group.incomingServices) { service in
                RemoteServiceView(service: service)
            }
        }
    }
}

struct ServiceGroupButton: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var group: ServiceGroup
    
    var body: some View {
        if isOpen {
            ClickableMenuEntry(text: "", icon: "arrowshape.turn.up.backward", action: {
                isOpen = !isOpen
            })
            ServiceGroupView(group: group)
        } else {
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
            .onTapGesture {
                isOpen = !isOpen
            }
            .padding(3)
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .contentShape(Rectangle())
            .cornerRadius(4)
        }
    }
}

struct RemoteServiceView: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var service: Service
    
    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: "circle")
                    .foregroundColor(service.available ? ( service.enabled ? .green : .orange) : .red)
                    .frame(maxWidth: 16, maxHeight: 16)
                
                VStack(alignment: .leading) {
                    Text(service.sourceName).font(.title3)
                    if service.available {
                        let address = if let scheme = service.scheme {
                            scheme + "://" + service.address.unsafelyUnwrapped + ":" + String(service.port.unsafelyUnwrapped)
                        } else {
                            service.address.unsafelyUnwrapped + ":" + String(service.port.unsafelyUnwrapped)
                        }
                        Text(verbatim: address).font(.caption)
                    } else {
                        Text(verbatim: "Connecting...").font(.caption)
                    }
                }
                Spacer()
                if service.available {
                    Image(systemName: "chevron.right")
                        .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
                }
            }
            .padding(3)
            .contentShape(Rectangle())
            .onTapGesture {
                if service.available {
                    withAnimation {
                        isOpen = !isOpen
                    }
                }
            }
            .onHover { hover in
                isHovered = hover
            }
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .cornerRadius(4)
            
            if isOpen {
                VStack(spacing: 0) {
                    if service.available {
                        if service.enabled {
                            let address = service.address.unsafelyUnwrapped + ":" + String(service.port.unsafelyUnwrapped);
                            if let scheme = service.scheme {
                                let url = scheme + "://" + service.address.unsafelyUnwrapped + ":" + String(service.port.unsafelyUnwrapped)
                                ClickableMenuEntry(text: "Open "+url, action: {
                                    if let url = URL(string: url) {
                                        NSWorkspace.shared.open(url)
                                    }
                                })
                            }
                            ClickableMenuEntry(text: "Copy " + address, action: {
                                copyToClipboard(address)
                            })
                            ClickableMenuEntry(text: "Disconnect", action: {
                                disable_accepted_service(service.id)
                            })
                        } else {
                            ClickableMenuEntry(text: "Connect", action: {
                                enable_accepted_service(service.id)
                            })
                        }
                    }
                }
            }
        }
    }
}

struct LocalServiceView: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @Environment(\.openWindow) private var openWindow

    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var localService: LocalService
    
    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: "circle")
                    .foregroundColor(localService.available ? .green : .red)
                    .frame(maxWidth: 16, maxHeight: 16)
                VStack(alignment: .leading) {
                    Text(verbatim: localService.name).font(.title3)
                    let address = if let scheme = localService.scheme {
                        scheme + "://" + localService.address + ":" + String(localService.port)
                    } else {
                        localService.address + ":" + String(localService.port)
                    }
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
                    let address = localService.address + ":" + String(localService.port);
                    if let scheme = localService.scheme {
                        let url = scheme + "://" + address
                        ClickableMenuEntry(text: "Open "+url, action: {
                            if let url = URL(string: url) {
                                NSWorkspace.shared.open(url)
                            }
                        })
                    }
                    ClickableMenuEntry(text: "Copy " + address, action: {
                        copyToClipboard(address)
                    })
                    ClickableMenuEntry(text: "Share", action: {
                        openWindow(id:"share-service", value: localService.id)
                        bringInFront()
                        self.closeWindow()
                    })
                    ClickableMenuEntry(text: "Delete", action: {
                        delete_local_service(self.localService.name)
                    })
                }
            }
        }
    }
    
    func closeWindow() {
        self.presentationMode.wrappedValue.dismiss()
    }
}

struct IncomingInvite: View {
    @State private var isHovered = false
    @State private var isOpen = false
    @ObservedObject var invite: Invite
    
    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(systemName: invite.accepting ? "envelope.open" : "envelope")
                    .frame(maxWidth: 16, maxHeight: 16)
                VStack(alignment: .leading) {
                    Text(verbatim: invite.serviceName).font(.title3)
                    if invite.accepting {
                        Text(verbatim: "Accepting...").font(.caption)
                    } else {
                        if let scheme = invite.serviceScheme {
                            Text(verbatim: scheme).font(.caption)
                        }
                    }
                }
                Spacer()
                if !invite.accepting {
                    Image(systemName: "chevron.right")
                        .rotationEffect(isOpen ? Angle.degrees(90.0) : Angle.degrees(0), anchor: .center)
                }
            }
            .padding(3)
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation {
                    if !invite.accepting {
                        isOpen = !isOpen
                    }
                }
            }
            .onHover { hover in
                isHovered = hover
            }
            .background(isHovered ? Color.gray.opacity(0.25) : Color.clear)
            .cornerRadius(4)
            
            if isOpen {
                VStack(spacing: 0) {
                    ClickableMenuEntry(text: "Accept", action: {
                        accept_invitation(invite.id)
                        isOpen = false
                    })
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
