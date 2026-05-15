#!/usr/bin/env swift

import AppKit
import ApplicationServices
import Foundation

struct Config {
    var targetName: String?
    var dumpTree = false
    var json = false
    var promptForTrust = false
    var maxDepth = 6
    var selectIndices: [Int]?
}

struct MenuNode: Encodable {
    let title: String?
    let enabled: Bool
    let separator: Bool
    let indices: [Int]
    let children: [MenuNode]
}

enum ProbeError: Error, CustomStringConvertible {
    case usage
    case accessibilityNotTrusted
    case dockNotRunning
    case itemNotFound(String)
    case actionUnsupported(String)
    case actionFailed(String, AXError)
    case shownMenuUnavailable(String)
    case invalidSelectionPath([Int])
    case selectionUnsupported([Int])
    case selectionFailed([Int], AXError)

    var description: String {
        switch self {
        case .usage:
            return """
            Usage:
              swift scripts/dock_ax_probe.swift --app "Zen"
              swift scripts/dock_ax_probe.swift --app "Zen" --json
              swift scripts/dock_ax_probe.swift --app "Zen" --select-indices 0,3
              swift scripts/dock_ax_probe.swift --app "Visual Studio Code" --dump-tree
            """
        case .accessibilityNotTrusted:
            return """
            Accessibility access is not granted for this process.
            Enable it in System Settings > Privacy & Security > Accessibility, then rerun.
            """
        case .dockNotRunning:
            return "The Dock process is not running."
        case let .itemNotFound(name):
            return "Could not find a Dock item matching \"\(name)\"."
        case let .actionUnsupported(name):
            return "\"\(name)\" does not expose the AXShowMenu action."
        case let .actionFailed(name, error):
            return "AXShowMenu failed for \"\(name)\" with \(error)."
        case let .shownMenuUnavailable(name):
            return "Dock item \"\(name)\" did not expose a shown menu after AXShowMenu."
        case let .invalidSelectionPath(indices):
            return "Invalid menu item path: \(indices)."
        case let .selectionUnsupported(indices):
            return "Menu item at path \(indices) does not expose a selectable action."
        case let .selectionFailed(indices, error):
            return "Selecting menu item at path \(indices) failed with \(error)."
        }
    }
}

func parseConfig() throws -> Config {
    var config = Config()
    var iterator = CommandLine.arguments.dropFirst().makeIterator()

    while let arg = iterator.next() {
        switch arg {
        case "--app":
            config.targetName = iterator.next()
        case "--dump-tree":
            config.dumpTree = true
        case "--json":
            config.json = true
        case "--prompt-trust":
            config.promptForTrust = true
        case "--max-depth":
            guard let value = iterator.next(), let depth = Int(value) else {
                throw ProbeError.usage
            }
            config.maxDepth = depth
        case "--select-indices":
            guard let value = iterator.next() else {
                throw ProbeError.usage
            }
            config.selectIndices = try parseIndexList(value)
        default:
            throw ProbeError.usage
        }
    }

    guard config.targetName != nil else {
        throw ProbeError.usage
    }

    return config
}

func parseIndexList(_ value: String) throws -> [Int] {
    if value.isEmpty {
        return []
    }

    let parts = value.split(separator: ",")
    let indices = try parts.map { part -> Int in
        guard let value = Int(part), value >= 0 else {
            throw ProbeError.usage
        }
        return value
    }

    return indices
}

func ensureAccessibilityTrust(prompt: Bool) throws {
    if prompt {
        let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        _ = AXIsProcessTrustedWithOptions(options)
    }

    guard AXIsProcessTrusted() else {
        throw ProbeError.accessibilityNotTrusted
    }
}

func dockApplicationElement() throws -> AXUIElement {
    guard let dockApp = NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.dock").first else {
        throw ProbeError.dockNotRunning
    }

    return AXUIElementCreateApplication(dockApp.processIdentifier)
}

func attributeValue(_ element: AXUIElement, _ attribute: String) -> CFTypeRef? {
    var value: CFTypeRef?
    let error = AXUIElementCopyAttributeValue(element, attribute as CFString, &value)
    guard error == .success else {
        return nil
    }
    return value
}

func stringAttribute(_ element: AXUIElement, _ attribute: String) -> String? {
    attributeValue(element, attribute) as? String
}

func boolAttribute(_ element: AXUIElement, _ attribute: String) -> Bool? {
    attributeValue(element, attribute) as? Bool
}

func children(of element: AXUIElement) -> [AXUIElement] {
    attributeValue(element, kAXChildrenAttribute as String) as? [AXUIElement] ?? []
}

func actions(of element: AXUIElement) -> [String] {
    var value: CFArray?
    let error = AXUIElementCopyActionNames(element, &value)
    guard error == .success, let names = value as? [String] else {
        return []
    }
    return names
}

func describe(_ element: AXUIElement) -> String {
    let role = stringAttribute(element, kAXRoleAttribute as String) ?? "?"
    let subrole = stringAttribute(element, kAXSubroleAttribute as String)
    let title = stringAttribute(element, kAXTitleAttribute as String)
    let description = stringAttribute(element, kAXDescriptionAttribute as String)

    let details = [
        title.map { "title=\($0)" },
        description.map { "description=\($0)" },
        subrole.map { "subrole=\($0)" },
    ].compactMap { $0 }

    if details.isEmpty {
        return role
    }

    return "\(role) [\(details.joined(separator: ", "))]"
}

func printTree(_ element: AXUIElement, indent: String = "", depth: Int, maxDepth: Int) {
    print("\(indent)\(describe(element))")

    guard depth < maxDepth else {
        return
    }

    for child in children(of: element) {
        printTree(child, indent: indent + "  ", depth: depth + 1, maxDepth: maxDepth)
    }
}

func matchesTarget(_ element: AXUIElement, targetName: String) -> Bool {
    let candidates = [
        stringAttribute(element, kAXTitleAttribute as String),
        stringAttribute(element, kAXDescriptionAttribute as String),
    ].compactMap { $0?.trimmingCharacters(in: .whitespacesAndNewlines) }

    return candidates.contains { candidate in
        candidate.caseInsensitiveCompare(targetName) == .orderedSame
    }
}

func findElement(named targetName: String, in root: AXUIElement, maxDepth: Int) -> AXUIElement? {
    var queue: [(AXUIElement, Int)] = [(root, 0)]

    while !queue.isEmpty {
        let (element, depth) = queue.removeFirst()
        if matchesTarget(element, targetName: targetName) {
            return element
        }

        if depth >= maxDepth {
            continue
        }

        for child in children(of: element) {
            queue.append((child, depth + 1))
        }
    }

    return nil
}

func shownMenuElements(for element: AXUIElement) -> [AXUIElement] {
    guard let raw = attributeValue(element, kAXShownMenuUIElementAttribute as String) else {
        return []
    }

    if CFGetTypeID(raw) == AXUIElementGetTypeID() {
        return [unsafeBitCast(raw, to: AXUIElement.self)]
    }

    if let menus = raw as? [AnyObject] {
        return menus.compactMap { item in
            guard CFGetTypeID(item) == AXUIElementGetTypeID() else {
                return nil
            }

            return unsafeBitCast(item, to: AXUIElement.self)
        }
    }

    return []
}

func menusInSubtree(of root: AXUIElement, maxDepth: Int) -> [AXUIElement] {
    var matches: [AXUIElement] = []
    var queue: [(AXUIElement, Int)] = [(root, 0)]

    while !queue.isEmpty {
        let (element, depth) = queue.removeFirst()
        if stringAttribute(element, kAXRoleAttribute as String) == kAXMenuRole as String {
            matches.append(element)
        }

        if depth >= maxDepth {
            continue
        }

        for child in children(of: element) {
            queue.append((child, depth + 1))
        }
    }

    return matches
}

func resolveShownMenu(for item: AXUIElement, in dock: AXUIElement, name: String) throws -> AXUIElement {
    let deadline = Date().addingTimeInterval(1.5)

    while Date() < deadline {
        if let menu = shownMenuElements(for: item).first {
            return menu
        }

        if let menu = menusInSubtree(of: dock, maxDepth: 8).first(where: {
            !menuItems(in: $0).isEmpty
        }) {
            return menu
        }

        usleep(50_000)
    }

    throw ProbeError.shownMenuUnavailable(name)
}

func performShowMenu(on element: AXUIElement, name: String) throws {
    let actionName = kAXShowMenuAction as String
    guard actions(of: element).contains(actionName) else {
        throw ProbeError.actionUnsupported(name)
    }

    let error = AXUIElementPerformAction(element, actionName as CFString)
    guard error == .success else {
        throw ProbeError.actionFailed(name, error)
    }
}

func menuItems(in menu: AXUIElement) -> [AXUIElement] {
    children(of: menu).filter { child in
        stringAttribute(child, kAXRoleAttribute as String) == kAXMenuItemRole as String
    }
}

func submenu(of menuItem: AXUIElement) -> AXUIElement? {
    children(of: menuItem).first { child in
        stringAttribute(child, kAXRoleAttribute as String) == kAXMenuRole as String
    }
}

func normalizedTitle(for menuItem: AXUIElement) -> String? {
    let raw = stringAttribute(menuItem, kAXTitleAttribute as String)?
        .trimmingCharacters(in: .whitespacesAndNewlines)

    if let raw, !raw.isEmpty {
        return raw
    }

    return nil
}

func buildMenuNodes(in menu: AXUIElement, path: [Int] = []) -> [MenuNode] {
    menuItems(in: menu).enumerated().map { index, menuItem in
        let indices = path + [index]
        let submenuNodes = submenu(of: menuItem).map { buildMenuNodes(in: $0, path: indices) } ?? []
        let title = normalizedTitle(for: menuItem)

        return MenuNode(
            title: title,
            enabled: boolAttribute(menuItem, kAXEnabledAttribute as String) ?? true,
            separator: title == nil && submenuNodes.isEmpty,
            indices: indices,
            children: submenuNodes
        )
    }
}

func printMenuTree(_ nodes: [MenuNode], indent: String = "") {
    for node in nodes {
        if node.separator {
            print("\(indent)- <separator>")
            continue
        }

        let marker = node.children.isEmpty ? "-" : "+"
        print("\(indent)\(marker) \(node.title ?? "<untitled>")")

        if !node.children.isEmpty {
            printMenuTree(node.children, indent: indent + "  ")
        }
    }
}

func printJSON<T: Encodable>(_ value: T) throws {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    let data = try encoder.encode(value)
    guard let json = String(data: data, encoding: .utf8) else {
        throw NSError(domain: "dock-ax-probe", code: 1)
    }
    print(json)
}

func targetMenuItem(in menu: AXUIElement, indices: [Int]) -> AXUIElement? {
    guard let first = indices.first else {
        return nil
    }

    let items = menuItems(in: menu)
    guard first < items.count else {
        return nil
    }

    let item = items[first]
    if indices.count == 1 {
        return item
    }

    guard let submenu = submenu(of: item) else {
        return nil
    }

    return targetMenuItem(in: submenu, indices: Array(indices.dropFirst()))
}

func performSelection(on menuItem: AXUIElement, indices: [Int]) throws {
    let availableActions = actions(of: menuItem)
    let actionOrder = [kAXPickAction as String, kAXPressAction as String]

    guard let action = actionOrder.first(where: { availableActions.contains($0) }) else {
        throw ProbeError.selectionUnsupported(indices)
    }

    let error = AXUIElementPerformAction(menuItem, action as CFString)
    guard error == .success else {
        throw ProbeError.selectionFailed(indices, error)
    }
}

func dismissMenu(_ menu: AXUIElement) {
    let actionName = kAXCancelAction as String
    guard actions(of: menu).contains(actionName) else {
        return
    }

    _ = AXUIElementPerformAction(menu, actionName as CFString)
}

do {
    let config = try parseConfig()
    try ensureAccessibilityTrust(prompt: config.promptForTrust)

    let dock = try dockApplicationElement()

    if config.dumpTree {
        print("Dock accessibility tree:")
        printTree(dock, depth: 0, maxDepth: config.maxDepth)
        print("")
    }

    let targetName = config.targetName!
    guard let item = findElement(named: targetName, in: dock, maxDepth: config.maxDepth) else {
        throw ProbeError.itemNotFound(targetName)
    }

    try performShowMenu(on: item, name: targetName)
    let menu = try resolveShownMenu(for: item, in: dock, name: targetName)

    if let selectIndices = config.selectIndices {
        guard let menuItem = targetMenuItem(in: menu, indices: selectIndices) else {
            throw ProbeError.invalidSelectionPath(selectIndices)
        }

        try performSelection(on: menuItem, indices: selectIndices)
    } else {
        let nodes = buildMenuNodes(in: menu)

        if config.json {
            try printJSON(nodes)
        } else {
            print("Matched Dock item: \(describe(item))")
            print("Actions: \(actions(of: item).joined(separator: ", "))")
            printMenuTree(nodes)
        }

        dismissMenu(menu)
    }
} catch let error as ProbeError {
    fputs("\(error.description)\n", stderr)
    exit(1)
} catch {
    fputs("Unexpected error: \(error)\n", stderr)
    exit(1)
}
