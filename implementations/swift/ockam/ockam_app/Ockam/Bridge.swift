import Foundation
import UserNotifications

struct Invitee: Identifiable {
    let name: Optional<String>
    let email: String
    
    var id: String { email }
}

struct Invite: Identifiable {
    let id: String
    let serviceName: String
    let serviceScheme: String?
}

struct LocalService: Identifiable {
    let name: String
    let address: String
    let port: UInt16
    let scheme: String?
    let sharedWith: [Invitee]
    let available: Bool
    
    var id: String { name }
}

struct Service: Identifiable {
    let sourceName: String
    let address: String
    let port: UInt16
    let scheme: String?
    let available: Bool
    
    var id: String { address+String(port) }
}

struct ServiceGroup: Identifiable {
    let name: String?
    let email: String
    let imageUrl: String?
    let invites: [Invite]
    let incomingServices: [Service]
    
    var id: String { email }
}

enum OrchestratorStatus: Int {
    case Disconnected = 0
    case Connecting
    case Connected
    case WaitingForToken
    case RetrievingSpace
    case RetrievingProject
}

struct ApplicationState {
    let enrolled: Bool
    let orchestrator_status: OrchestratorStatus
    let enrollmentName: String?
    let enrollmentEmail: String?
    let enrollmentImage: String?
    let enrollmentGithubUser: String?
    let localServices: [LocalService]
    let groups: [ServiceGroup]
}

enum NotificationKind: Int {
    case information = 0
    case warning = 1
    case error = 2
}

struct Notification {
    var kind: NotificationKind
    var title: String
    var message: String
}

func swift_demo_application_state() -> ApplicationState {
    return convertApplicationState(cState: mock_application_state())
}

func swift_application_snapshot() -> ApplicationState {
    return convertApplicationState(cState: application_state_snapshot())
}


func swift_initialize_application() {
    let applicationStateClosure: @convention(c) (C_ApplicationState) -> Void = { state in
        StateContainer.shared.update( state: convertApplicationState(cState: state ) )
    }

    let notificationClosure: @convention(c) (C_Notification) -> Void = { cNotification in
        let notification = convertNotification(cNotification: cNotification)
        let center = UNUserNotificationCenter.current()

        center.requestAuthorization(options: [.alert, .sound, .badge]) { (granted, error) in
           if granted {
               let content = UNMutableNotificationContent()
               content.title = notification.title
               content.body = notification.message

               let trigger = UNTimeIntervalNotificationTrigger(timeInterval: 5, repeats: false)
               let request = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: trigger)
               
               UNUserNotificationCenter.current().add(request)
           } else {
               print("Notification permission denied.")
           }
       }
    }

    initialize_application(applicationStateClosure, notificationClosure)

    UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { granted, error in
        if granted == true {
            print("Notification permission granted")
        } else {
            print("Notifications not allowed")
        }
    }
}

func optional_string(str: UnsafePointer<Int8>?) -> String? {
    guard let str = str else { return nil }
    return String(cString: str)
}

func convertNotification(cNotification: C_Notification) -> Notification {
    let kind = NotificationKind(rawValue: Int(cNotification.kind.rawValue))!
    let title = String(cString: cNotification.title)
    let message = String(cString: cNotification.message)

    return Notification(kind: kind, title: title, message: message)
}

func convertApplicationState(cState: C_ApplicationState) -> ApplicationState {
    let enrollmentName = optional_string(str: cState.enrollment_name)
    let enrollmentEmail = optional_string(str: cState.enrollment_email)
    let enrollmentImage = optional_string(str: cState.enrollment_image)
    let enrollmentGithubUser = optional_string(str: cState.enrollment_github_user)

    var localServices: [LocalService] = []
    var i = 0
    while let cLocalService = cState.local_services[i] {
        localServices.append(convertLocalService(cLocalService: cLocalService))
        i += 1
    }

    var groups: [ServiceGroup] = []
    i = 0
    while let cGroup = cState.groups[i] {
        groups.append(convertServiceGroup(cServiceGroup: cGroup))
        i += 1
    }

    return ApplicationState(
        enrolled: cState.enrolled != 0,
        orchestrator_status: OrchestratorStatus(rawValue: Int(cState.orchestrator_status.rawValue))!,
        enrollmentName: enrollmentName,
        enrollmentEmail: enrollmentEmail,
        enrollmentImage: enrollmentImage,
        enrollmentGithubUser: enrollmentGithubUser,
        localServices: localServices,
        groups: groups
    )
}

func convertLocalService(cLocalService: UnsafePointer<C_LocalService>) -> LocalService {
    let cService = cLocalService.pointee

    let name = String(cString: cService.name)
    let address = String(cString: cService.address)
    let scheme = optional_string(str: cService.scheme)

    var sharedWith: [Invitee] = []
    var i = 0
    while let cInvitee = cService.shared_with[i] {
        sharedWith.append(convertInvitee(cInvitee: cInvitee))
        i += 1
    }

    return LocalService(
        name: name,
        address: address,
        port: cService.port,
        scheme: scheme,
        sharedWith: sharedWith,
        available: cService.available != 0
    )
}


func convertInvitee(cInvitee: UnsafePointer<C_Invitee>) -> Invitee {
    let cRecord = cInvitee.pointee

    let name = String(cString: cRecord.name)
    let email = String(cString: cRecord.email)

    return Invitee(name: name, email: email)
}

func convertServiceGroup(cServiceGroup: UnsafePointer<C_ServiceGroup>) -> ServiceGroup {
    let cGroup = cServiceGroup.pointee

    let name = optional_string(str: cGroup.name)
    let email = String(cString: cGroup.email)
    let imageUrl = optional_string(str: cGroup.image_url)

    var invites: [Invite] = []
    var i = 0
    while let cInvite = cGroup.invites[i] {
        invites.append(convertInvite(cInvite: cInvite))
        i += 1
    }

    var incomingServices: [Service] = []
    i = 0
    while let cService = cGroup.incoming_services[i] {
        incomingServices.append(convertService(cService: cService))
        i += 1
    }

    return ServiceGroup(
        name: name,
        email: email,
        imageUrl: imageUrl,
        invites: invites,
        incomingServices: incomingServices
    )
}

func convertInvite(cInvite: UnsafePointer<C_Invite>) -> Invite {
    let cRecord = cInvite.pointee

    let id = String(cString: cRecord.id)
    let serviceName = String(cString: cRecord.service_name)
    let serviceScheme = optional_string(str: cRecord.service_scheme)

    return Invite(id: id, serviceName: serviceName, serviceScheme: serviceScheme)
}

func convertService(cService: UnsafePointer<C_Service>) -> Service {
    let cRecord = cService.pointee

    let sourceName = String(cString: cRecord.source_name)
    let address = String(cString: cRecord.address)
    let scheme = optional_string(str: cRecord.scheme)

    return Service(
        sourceName: sourceName,
        address: address,
        port: cRecord.port,
        scheme: scheme,
        available: cRecord.available != 0
    )
    
}
