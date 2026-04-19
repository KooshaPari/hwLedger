import Foundation
import AppKit
import ApplicationServices
import CoreGraphics

/// Real AppDriver using macOS Accessibility Framework (AXUIElement + CGEvent).
/// Drives SwiftUI apps via the public Accessibility API (available since macOS 10.2).
///
/// Accessibility Prerequisites:
/// 1. Grant Terminal/Xcode Accessibility permission:
///    - System Settings > Privacy & Security > Accessibility
///    - Add Terminal or Xcode to the allowed apps list
/// 2. Restart the test runner after granting permission
/// 3. The app must have `.accessibilityIdentifier()` on target elements
///
/// Implementation notes:
/// - Traversal is depth-limited (20 levels) to prevent infinite loops
/// - Screenshots are captured via CGWindowListCreateImage scoped to app window
/// - Sliders are set via AXValue attribute; fallback to keyboard simulation
/// - Text input uses CGEventCreateKeyboardEvent for unicode support
public final class AppDriver {
    private let app: NSRunningApplication
    private let axApp: AXUIElement

    /// Initialize AppDriver with the path to the HwLedger.app bundle.
    /// Launches the app and initializes the accessibility element.
    /// - Parameter appPath: Path to HwLedger.app (e.g., "/path/to/HwLedger.app")
    public init(appPath: String) throws {
        let workspace = NSWorkspace.shared
        let url = URL(fileURLWithPath: appPath)

        let config = NSWorkspaceOpenConfiguration()
        config.createsNewApplicationInstance = true

        var launchedApp: NSRunningApplication?
        let semaphore = DispatchSemaphore(value: 0)
        var launchError: Error?

        workspace.openURL(url, configuration: config) { app, error in
            launchedApp = app
            launchError = error
            semaphore.signal()
        }

        let result = semaphore.wait(timeout: .now() + 5.0)
        guard result == .timedOut ? false : true, let app = launchedApp else {
            throw launchError ?? AppDriverError.launchFailed
        }

        self.app = app

        // Wait for process ID to be available
        let startTime = Date()
        while app.processIdentifier == 0 && Date().timeIntervalSince(startTime) < 5 {
            Thread.sleep(forTimeInterval: 0.1)
        }

        guard app.processIdentifier != 0 else {
            throw AppDriverError.launchFailed
        }

        self.axApp = AXUIElementCreateApplication(app.processIdentifier)

        // Wait for app to become active and accessible (2 seconds debounce)
        try waitForIdle(timeout: 5.0)
    }

    /// Find the first element matching an accessibility identifier via depth-first traversal.
    /// Recursively searches the AX hierarchy up to 20 levels deep.
    /// - Parameter id: The accessibility identifier to match (matches AXIdentifier attribute)
    /// - Returns: The matching AXUIElement
    public func element(byId id: String) throws -> AXUIElement {
        guard let element = try findDescendant(axApp, matchingId: id, depthLimit: 20) else {
            throw AppDriverError.elementNotFound(id)
        }
        return element
    }

    /// Click a button or interactive element by accessibility identifier.
    /// First tries AXPress action; falls back to synthetic click via CGEvent.
    /// - Parameter identifier: The accessibility identifier of the element to click
    public func tapButton(identifier: String) throws {
        let el = try element(byId: identifier)

        // Try AXPress first (preferred)
        var actions: CFArray?
        let axErr = AXUIElementCopyActionNames(el, &actions)

        if axErr == .success, let actions = actions as? [String], actions.contains(kAXPressAction) {
            _ = AXUIElementPerformAction(el, kAXPressAction as CFString)
            try waitForIdle(timeout: 1.0)
            return
        }

        // Fallback: click via coordinates
        try clickElementViaCoordinates(el)
    }

    /// Set a slider to a normalized value (0.0 = min, 1.0 = max).
    /// Attempts direct AXValue attribute write; falls back to keyboard simulation.
    /// - Parameters:
    ///   - identifier: The accessibility identifier of the slider
    ///   - normalizedValue: Value from 0.0 to 1.0
    public func dragSlider(identifier: String, to normalizedValue: Double) throws {
        let el = try element(byId: identifier)
        let clampedValue = max(0.0, min(1.0, normalizedValue))

        // Try direct AXValue write
        let result = AXUIElementSetAttributeValue(
            el,
            kAXValueAttribute as CFString,
            clampedValue as CFNumber
        )

        if result == .success {
            try waitForIdle(timeout: 1.0)
            return
        }

        // Fallback: use keyboard to adjust slider
        try focusElement(el)

        // Home key to start, then right arrows to set value
        let steps = Int(clampedValue * 100)
        for _ in 0..<steps {
            try sendKeyboardEvent(keyCode: 124, down: true)  // Right arrow
            Thread.sleep(forTimeInterval: 0.01)
            try sendKeyboardEvent(keyCode: 124, down: false)
        }

        try waitForIdle(timeout: 1.0)
    }

    /// Type text into the currently focused element.
    /// Uses CGEventCreateKeyboardEvent for unicode support.
    /// - Parameter text: The text to type
    public func typeText(_ text: String) throws {
        // Use the pasteboard for reliable unicode input
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)

        // Simulate Cmd+V paste
        try sendKeyboardEvent(keyCode: 9, modifiers: .maskCommand, down: true)   // V with Cmd
        Thread.sleep(forTimeInterval: 0.05)
        try sendKeyboardEvent(keyCode: 9, modifiers: .maskCommand, down: false)

        try waitForIdle(timeout: 1.0)
    }

    /// Wait for the app to become idle (no UI changes for 250ms).
    /// Polls the app's accessible elements; useful after interactions.
    /// - Parameter timeout: Maximum time to wait before giving up
    public func waitForIdle(timeout: TimeInterval = 5.0) throws {
        let startTime = Date()
        var lastHash: Int = 0

        while Date().timeIntervalSince(startTime) < timeout {
            var windows: AnyObject?
            let err = AXUIElementCopyAttributeValue(
                axApp,
                kAXWindowsAttribute as CFString,
                &windows
            )

            if err == .success, let windowArray = windows as? [AXUIElement], !windowArray.isEmpty {
                let currentHash = windowArray.hashValue
                if currentHash == lastHash {
                    // Hash unchanged for 250ms
                    Thread.sleep(forTimeInterval: 0.25)
                    // Verify one more time
                    var windows2: AnyObject?
                    AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows2)
                    if (windows2 as? [AXUIElement])?.hashValue == currentHash {
                        return
                    }
                }
                lastHash = currentHash
            }

            Thread.sleep(forTimeInterval: 0.05)
        }
    }

    /// Capture a screenshot of the app's window (not full screen).
    /// Uses CGWindowListCreateImage scoped to the app's main window.
    /// Falls back to full-screen capture if window ID cannot be determined.
    /// - Returns: PNG image data
    public func screenshot() throws -> Data {
        guard let windowID = try getMainWindowID() else {
            // Fallback: full-screen capture
            return try screenshotFullScreen()
        }

        let bounds = try getWindowBounds(windowID)
        guard let cgImage = CGWindowListCreateImage(
            bounds,
            .optionIncludingWindow,
            windowID,
            .boundsIgnoreFraming
        ) else {
            return try screenshotFullScreen()
        }

        let nsImage = NSImage(cgImage: cgImage, size: .zero)
        guard let tiffData = nsImage.tiffRepresentation,
              let bitmapRep = NSBitmapImageRep(data: tiffData),
              let pngData = bitmapRep.representation(using: .png, properties: [:]) else {
            throw AppDriverError.screenshotFailed
        }

        return pngData
    }

    /// Wait for an element to appear on screen (with timeout).
    /// - Parameters:
    ///   - id: The accessibility identifier to wait for
    ///   - timeout: Maximum time to wait (default 5 seconds)
    /// - Returns: The element once found
    public func waitForElement(id: String, timeout: TimeInterval = 5.0) throws -> AXUIElement {
        let startTime = Date()

        while Date().timeIntervalSince(startTime) < timeout {
            if let element = try? element(byId: id) {
                return element
            }
            Thread.sleep(forTimeInterval: 0.1)
        }

        throw AppDriverError.elementNotFound(id)
    }

    /// Get the string value of an element (for assertions).
    /// - Parameter identifier: The accessibility identifier
    /// - Returns: The element's AXValue as a String
    public func getValue(identifier: String) throws -> String {
        let el = try element(byId: identifier)
        var value: AnyObject?
        let err = AXUIElementCopyAttributeValue(el, kAXValueAttribute as CFString, &value)

        guard err == .success else {
            throw AppDriverError.actionFailed("getValue: could not read AXValue")
        }

        if let str = value as? String {
            return str
        } else if let num = value as? NSNumber {
            return num.stringValue
        }

        if let value = value {
            return String(describing: value)
        }
        return ""
    }

    // MARK: - Private Helpers

    /// Recursively find a descendant element matching an accessibility identifier.
    private func findDescendant(
        _ element: AXUIElement,
        matchingId targetId: String,
        depthLimit: Int
    ) throws -> AXUIElement? {
        guard depthLimit > 0 else { return nil }

        // Check current element
        var id: AnyObject?
        AXUIElementCopyAttributeValue(element, kAXIdentifierAttribute as CFString, &id)
        if let id = id as? String, id == targetId {
            return element
        }

        // Traverse children
        var childrenRef: AnyObject?
        AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef)

        guard let children = childrenRef as? [AXUIElement] else {
            return nil
        }

        for child in children {
            if let found = try findDescendant(child, matchingId: targetId, depthLimit: depthLimit - 1) {
                return found
            }
        }

        return nil
    }

    /// Click an element by calculating its on-screen position and using CGEvent.
    private func clickElementViaCoordinates(_ element: AXUIElement) throws {
        var position: AnyObject?
        var size: AnyObject?

        AXUIElementCopyAttributeValue(element, kAXPositionAttribute as CFString, &position)
        AXUIElementCopyAttributeValue(element, kAXSizeAttribute as CFString, &size)

        guard position != nil, size != nil else {
            throw AppDriverError.actionFailed("tapButton: could not determine element position")
        }

        var cgPoint = CGPoint.zero
        var cgSize = CGSize.zero

        if let posValue = position {
            AXValueGetValue(posValue as! AXValue, .cgPoint, &cgPoint)
        }
        if let sizeValue = size {
            AXValueGetValue(sizeValue as! AXValue, .cgSize, &cgSize)
        }

        let clickPoint = CGPoint(
            x: cgPoint.x + cgSize.width / 2,
            y: cgPoint.y + cgSize.height / 2
        )

        // Simulate click: down, then up
        guard let downEvent = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: clickPoint, timestamp: 0),
              let upEvent = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: clickPoint, timestamp: 0) else {
            throw AppDriverError.actionFailed("tapButton: could not create CGEvent")
        }

        downEvent.post(tap: .cghidEventTap)
        Thread.sleep(forTimeInterval: 0.05)
        upEvent.post(tap: .cghidEventTap)
    }

    /// Focus an element (make it the focused UI element).
    private func focusElement(_ element: AXUIElement) throws {
        _ = AXUIElementSetAttributeValue(element, kAXFocusedAttribute as CFString, true as CFBoolean)
        Thread.sleep(forTimeInterval: 0.1)
    }

    /// Send a keyboard event (key press or release).
    private func sendKeyboardEvent(
        keyCode: CGKeyCode,
        modifiers: CGEventFlags = [],
        down: Bool
    ) throws {
        let keyDown = down ? CGEventType.keyDown : CGEventType.keyUp
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: keyCode, keyDown: down) else {
            throw AppDriverError.actionFailed("sendKeyboardEvent: failed to create event")
        }

        event.flags = modifiers
        event.post(tap: .cghidEventTap)
    }

    /// Get the main window ID of the app.
    private func getMainWindowID() throws -> CGWindowID? {
        var windows: CFArray?
        let err = AXUIElementCopyAttributeValue(
            axApp,
            kAXWindowsAttribute as CFString,
            &windows
        )

        guard err == .success, let windows = windows as? [AXUIElement], !windows.isEmpty else {
            return nil
        }

        // Try to get the native window ID via CFTypeRef bridge
        for window in windows {
            if let pid = try getWindowPID(window) {
                // Get all windows for this PID and filter by app
                let opts = CGWindowListOption.optionOnScreenOnly
                guard let allWindows = CGWindowListCopyWindowInfo(opts, kCGNullWindowID) as? [[String: Any]] else {
                    continue
                }

                for winInfo in allWindows {
                    if let winPID = winInfo[kCGWindowOwnerPID as String] as? pid_t, winPID == pid {
                        if let winID = winInfo[kCGWindowNumber as String] as? CGWindowID {
                            return winID
                        }
                    }
                }
            }
        }

        return nil
    }

    /// Get the PID associated with a window element (fallback: return app's PID).
    private func getWindowPID(_ window: AXUIElement) throws -> pid_t? {
        return app.processIdentifier
    }

    /// Get the bounds of a window by its CGWindowID.
    private func getWindowBounds(_ windowID: CGWindowID) throws -> CGRect {
        guard let allWindows = CGWindowListCopyWindowInfo(
            .optionOnScreenOnly,
            kCGNullWindowID
        ) as? [[String: Any]] else {
            return CGRect(x: 0, y: 0, width: 1440, height: 900)
        }

        for winInfo in allWindows {
            if let winID = winInfo[kCGWindowNumber as String] as? CGWindowID, winID == windowID {
                if let bounds = winInfo[kCGWindowBounds as String] as? [String: CGFloat] {
                    return CGRect(
                        x: bounds["X"] ?? 0,
                        y: bounds["Y"] ?? 0,
                        width: bounds["Width"] ?? 1440,
                        height: bounds["Height"] ?? 900
                    )
                }
            }
        }

        return CGRect(x: 0, y: 0, width: 1440, height: 900)
    }

    /// Fallback full-screen screenshot.
    private func screenshotFullScreen() throws -> Data {
        guard let cgImage = CGWindowListCreateImage(
            .infinite,
            .optionOnScreenBelowWindow,
            kCGNullWindowID,
            []
        ) else {
            throw AppDriverError.screenshotFailed
        }

        let nsImage = NSImage(cgImage: cgImage, size: .zero)
        guard let tiffData = nsImage.tiffRepresentation,
              let bitmapRep = NSBitmapImageRep(data: tiffData),
              let pngData = bitmapRep.representation(using: .png, properties: [:]) else {
            throw AppDriverError.screenshotFailed
        }

        return pngData
    }
}


// MARK: - Error Types

public enum AppDriverError: LocalizedError {
    case launchFailed
    case windowNotFound
    case screenshotFailed
    case elementNotFound(String)
    case appNotActive
    case actionFailed(String)

    public var errorDescription: String? {
        switch self {
        case .launchFailed:
            return "Failed to launch app"
        case .windowNotFound:
            return "App window not found"
        case .screenshotFailed:
            return "Failed to capture screenshot"
        case .elementNotFound(let id):
            return "Element not found: \(id)"
        case .appNotActive:
            return "App did not become active"
        case .actionFailed(let action):
            return "Action failed: \(action)"
        }
    }
}
