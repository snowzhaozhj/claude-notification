import Cocoa
import UserNotifications

// MARK: - CLI Argument Parsing

struct NotifierArgs {
    var title: String = "Claude Code"
    var subtitle: String = ""
    var message: String = ""
    var activate: String = ""  // bundle ID to activate on click
    var group: String = ""     // thread/group ID
    var sound: Bool = false

    static func parse() -> NotifierArgs {
        var args = NotifierArgs()
        let argv = CommandLine.arguments
        var i = 1
        while i < argv.count {
            switch argv[i] {
            case "-title" where i + 1 < argv.count:
                i += 1; args.title = argv[i]
            case "-subtitle" where i + 1 < argv.count:
                i += 1; args.subtitle = argv[i]
            case "-message" where i + 1 < argv.count:
                i += 1; args.message = argv[i]
            case "-activate" where i + 1 < argv.count:
                i += 1; args.activate = argv[i]
            case "-group" where i + 1 < argv.count:
                i += 1; args.group = argv[i]
            case "-sound":
                args.sound = true
            default:
                break
            }
            i += 1
        }
        return args
    }
}

// MARK: - Notification Delegate

class NotificationDelegate: NSObject, UNUserNotificationCenterDelegate {
    let activateBundleId: String

    init(activateBundleId: String) {
        self.activateBundleId = activateBundleId
        super.init()
    }

    // Show notification even when app is in foreground
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound])
    }

    // Handle click on notification
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        if !activateBundleId.isEmpty {
            if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: activateBundleId) {
                NSWorkspace.shared.openApplication(at: url, configuration: NSWorkspace.OpenConfiguration())
            }
        }
        completionHandler()
        // Exit after handling click
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            NSApplication.shared.terminate(nil)
        }
    }
}

// MARK: - Main

let args = NotifierArgs.parse()

// Detect terminal bundle ID if not specified
let activateId: String = {
    if !args.activate.isEmpty { return args.activate }
    // Auto-detect from TERM_PROGRAM
    let termProgram = ProcessInfo.processInfo.environment["TERM_PROGRAM"] ?? ""
    switch termProgram.lowercased() {
    case "iterm.app", "iterm2": return "com.googlecode.iterm2"
    case "apple_terminal": return "com.apple.Terminal"
    case "ghostty": return "com.mitchellh.ghostty"
    case "warp": return "dev.warp.Warp-Stable"
    case "alacritty": return "org.alacritty"
    case "kitty": return "net.kovidgoyal.kitty"
    case "wezterm": return "com.github.wez.wezterm"
    case "hyper": return "co.zeit.hyper"
    default: return ""
    }
}()

let app = NSApplication.shared
let delegate = NotificationDelegate(activateBundleId: activateId)
let center = UNUserNotificationCenter.current()
center.delegate = delegate

// Request permission
let semaphore = DispatchSemaphore(value: 0)
center.requestAuthorization(options: [.alert, .sound, .badge]) { granted, error in
    if let error = error {
        fputs("Permission error: \(error.localizedDescription)\n", stderr)
    }
    semaphore.signal()
}
semaphore.wait()

// Build notification content
let content = UNMutableNotificationContent()
content.title = args.title
content.body = args.message
if !args.subtitle.isEmpty {
    content.subtitle = args.subtitle
}
if args.sound {
    content.sound = .default
}
if !args.group.isEmpty {
    content.threadIdentifier = args.group
}

// Send notification
let request = UNNotificationRequest(
    identifier: args.group.isEmpty ? UUID().uuidString : args.group,
    content: content,
    trigger: nil
)

let sendSemaphore = DispatchSemaphore(value: 0)
center.add(request) { error in
    if let error = error {
        fputs("Notification error: \(error.localizedDescription)\n", stderr)
    }
    sendSemaphore.signal()
}
sendSemaphore.wait()

// Keep alive briefly for delegate callbacks, then exit
DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
    NSApplication.shared.terminate(nil)
}
app.run()
