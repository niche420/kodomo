import SwiftUI

// MARK: - EditorWidget

struct EditorWidget: Identifiable {
    let id: String
    var kind: WidgetKind
    var label: String
    var x: Float
    var y: Float
    var w: Float
    var h: Float

    enum WidgetKind: String, CaseIterable {
        case button, dpad, joystick, trigger
        var displayName: String {
            switch self {
            case .button:   return "Button"
            case .dpad:     return "D-Pad"
            case .joystick: return "Joystick"
            case .trigger:  return "Trigger"
            }
        }
        var systemImage: String {
            switch self {
            case .button:   return "circle"
            case .dpad:     return "dpad"
            case .joystick: return "circle.dotted"
            case .trigger:  return "rectangle.roundedtop"
            }
        }
    }
}

// MARK: - ProfileEditorView

struct ProfileEditorView: View {
    let server: PairedServer
    let game: ServerAPI.GameEntry
    let profileName: String

    @Environment(\.dismiss) private var dismiss

    @State private var widgets: [EditorWidget] = []
    @State private var slotBindings: [String: PhysicalInput] = [:]
    @State private var joystickModes: [String: JoystickMode] = [:]
    @State private var selectedWidgetID: String? = nil
    @State private var showBindingSheet = false
    @State private var isSaving = false
    @State private var saveError: String? = nil
    @State private var isLoading = true

    private var api: ServerAPI { ServerAPI(server: server) }

    var body: some View {
        Group {
            if isLoading {
                ProgressView("Loading...")
            } else {
                editorLayout
            }
        }
        .navigationTitle(profileName)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { toolbarContent }
        .task { await loadProfile() }
        .sheet(isPresented: $showBindingSheet) {
            if let id = selectedWidgetID,
               let widget = widgets.first(where: { $0.id == id }) {
                BindingSheetView(
                    widget: widget,
                    slotBindings: bindingSubset(for: widget),
                    joystickMode: joystickModes[widget.id] ?? .GamepadStick(x: .LeftX, y: .LeftY),
                    onSave: { newBindings, newMode in
                        applyBindings(newBindings, mode: newMode, to: widget)
                    }
                )
            }
        }
    }

    // MARK: - Layout

    private var editorLayout: some View {
        HStack(spacing: 0) {
            palette
                .frame(width: 72)
                .background(Color(uiColor: .systemGroupedBackground))
            Divider()
            GeometryReader { geo in
                CanvasView(
                    widgets: $widgets,
                    selectedID: selectedWidgetID,
                    canvasSize: geo.size,
                    onTapWidget: { id in
                        selectedWidgetID = id
                        showBindingSheet = true
                    },
                    onTapBackground: {
                        selectedWidgetID = nil
                    }
                )
            }
            .background(Color.black)
        }
        .ignoresSafeArea(edges: .bottom)
    }

    // MARK: - Palette

    private var palette: some View {
        VStack(spacing: 4) {
            Text("ADD")
                .font(.system(size: 9, weight: .semibold))
                .foregroundStyle(.secondary)
                .padding(.top, 12)
            ForEach(EditorWidget.WidgetKind.allCases, id: \.self) { kind in
                Button(action: { addWidget(kind: kind) }) {
                    VStack(spacing: 4) {
                        Image(systemName: kind.systemImage)
                            .font(.system(size: 20))
                        Text(kind.displayName)
                            .font(.system(size: 9))
                    }
                    .frame(width: 56, height: 56)
                    .background(Color(uiColor: .secondarySystemGroupedBackground))
                    .cornerRadius(10)
                }
                .foregroundStyle(.primary)
                .padding(.vertical, 2)
            }
            Spacer()
            if selectedWidgetID != nil {
                Button(role: .destructive, action: {
                    if let id = selectedWidgetID { deleteWidget(id: id) }
                }) {
                    VStack(spacing: 4) {
                        Image(systemName: "trash")
                            .font(.system(size: 18))
                        Text("Delete")
                            .font(.system(size: 9))
                    }
                    .frame(width: 56, height: 56)
                    .background(Color.red.opacity(0.12))
                    .cornerRadius(10)
                }
                .padding(.bottom, 12)
            }
        }
        .padding(.horizontal, 8)
    }

    // MARK: - Toolbar

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        ToolbarItem(placement: .cancellationAction) {
            Button("Cancel") { dismiss() }
        }
        ToolbarItem(placement: .primaryAction) {
            Button(action: { Task { await save() } }) {
                if isSaving {
                    ProgressView().controlSize(.small)
                } else {
                    Text("Save")
                }
            }
            .disabled(isSaving)
        }
    }

    // MARK: - Widget management

    private func addWidget(kind: EditorWidget.WidgetKind) {
        let id = "\(kind.rawValue)_\(UUID().uuidString.prefix(6).lowercased())"
        let size = defaultSize(kind)
        widgets.append(EditorWidget(
            id: id, kind: kind, label: kind.displayName,
            x: 0.5, y: 0.5, w: size.w, h: size.h
        ))
        selectedWidgetID = id
    }

    private func deleteWidget(id: String) {
        widgets.removeAll { $0.id == id }
        slotBindings = slotBindings.filter { !$0.key.hasPrefix(id) }
        joystickModes.removeValue(forKey: id)
        selectedWidgetID = nil
    }

    private func defaultSize(_ kind: EditorWidget.WidgetKind) -> (w: Float, h: Float) {
        switch kind {
        case .button:   return (0.10, 0.18)
        case .dpad:     return (0.18, 0.32)
        case .joystick: return (0.20, 0.36)
        case .trigger:  return (0.08, 0.22)
        }
    }

    // MARK: - Slot helpers

    private func bindingSubset(for widget: EditorWidget) -> [String: PhysicalInput] {
        slotsFor(widget: widget).reduce(into: [:]) { dict, slot in
            dict[slot] = slotBindings[slot]
        }
    }

    private func slotsFor(widget: EditorWidget) -> [String] {
        switch widget.kind {
        case .button, .trigger:
            return [widget.id]
        case .dpad:
            return ["up", "down", "left", "right"].map { "\(widget.id)_\($0)" }
        case .joystick:
            let mode = joystickModes[widget.id] ?? .GamepadStick(x: .LeftX, y: .LeftY)
            switch mode {
            case .Directional:
                return ["up", "down", "left", "right"].map { "\(widget.id)_\($0)" }
            default:
                return ["\(widget.id)_x", "\(widget.id)_y"]
            }
        }
    }

    private func applyBindings(
        _ newBindings: [String: PhysicalInput],
        mode: JoystickMode?,
        to widget: EditorWidget
    ) {
        for (slot, input) in newBindings {
            slotBindings[slot] = input
        }
        if let mode = mode, widget.kind == .joystick {
            joystickModes[widget.id] = mode
        }
        if (widget.kind == .button || widget.kind == .trigger),
           let input = newBindings[widget.id],
           let idx = widgets.firstIndex(where: { $0.id == widget.id }) {
            widgets[idx].label = shortLabel(for: input)
        }
    }

    private func shortLabel(for input: PhysicalInput) -> String {
        switch input {
        case .Key(let sc):        return keyDisplayName(sc: sc)
        case .MButton(let b):
            let names = ["LMB", "RMB", "MMB"]
            return Int(b) < names.count ? names[Int(b)] : "MB\(b)"
        case .MAxis(let a):       return a == .X ? "Mouse X" : "Mouse Y"
        case .GPadButton(let b):  return b.rawValue
        case .GPadAxis(let a):    return a.rawValue
        case .GPadTrigger(let t): return t.rawValue
        }
    }

    // MARK: - Load / Save

    private func loadProfile() async {
        isLoading = true
        do {
            let profile = try await api.fetchProfile(game: game.title, name: profileName)
            buildEditorState(from: profile)
        } catch {
            // New profile — start blank
        }
        isLoading = false
    }

    private func buildEditorState(from profile: GameProfile) {
        widgets = profile.widgets.compactMap { tw -> EditorWidget? in
            switch tw {
            case .Button(let id, let label, let rect):
                return EditorWidget(id: id, kind: .button, label: label,
                                    x: rect.x, y: rect.y, w: rect.w, h: rect.h)
            case .DPad(let id, let rect):
                return EditorWidget(id: id, kind: .dpad, label: "",
                                    x: rect.x, y: rect.y, w: rect.w, h: rect.h)
            case .Joystick(let id, let rect, let mode):
                joystickModes[id] = mode
                return EditorWidget(id: id, kind: .joystick, label: "",
                                    x: rect.x, y: rect.y, w: rect.w, h: rect.h)
            case .Trigger(let id, let label, let rect):
                return EditorWidget(id: id, kind: .trigger, label: label,
                                    x: rect.x, y: rect.y, w: rect.w, h: rect.h)
            }
        }
        for wb in profile.bindings {
            if let action = profile.actions.first(where: { $0.id == wb.action_id }) {
                slotBindings[wb.widget_slot] = action.input
            }
        }
    }

    private func save() async {
        isSaving = true
        saveError = nil
        let profile = buildProfile()
        do {
            try await api.saveProfile(game: game.title, name: profileName, profile: profile)
            dismiss()
        } catch {
            saveError = "Save failed"
        }
        isSaving = false
    }

    private func buildProfile() -> GameProfile {
        var touchWidgets: [TouchWidget] = []
        var actions: [Action] = []
        var widgetBindings: [WidgetBinding] = []

        for widget in widgets {
            let rect = WidgetRect(x: widget.x, y: widget.y, w: widget.w, h: widget.h)
            let mode = joystickModes[widget.id] ?? .GamepadStick(x: .LeftX, y: .LeftY)
            switch widget.kind {
            case .button:
                touchWidgets.append(.Button(id: widget.id, label: widget.label, rect: rect))
            case .dpad:
                touchWidgets.append(.DPad(id: widget.id, rect: rect))
            case .joystick:
                touchWidgets.append(.Joystick(id: widget.id, rect: rect, mode: mode))
            case .trigger:
                touchWidgets.append(.Trigger(id: widget.id, label: widget.label, rect: rect))
            }
            for slot in slotsFor(widget: widget) {
                guard let input = slotBindings[slot] else { continue }
                actions.append(Action(id: slot, label: slot, input: input))
                widgetBindings.append(WidgetBinding(widget_slot: slot, action_id: slot))
            }
        }
        return GameProfile(game_title: game.title, widgets: touchWidgets,
                           actions: actions, bindings: widgetBindings)
    }
}

// MARK: - CanvasView

struct CanvasView: View {
    @Binding var widgets: [EditorWidget]
    // Plain value — CanvasView reads it, never writes it. Mutations go via callbacks.
    let selectedID: String?
    let canvasSize: CGSize
    let onTapWidget: (String) -> Void
    let onTapBackground: () -> Void

    var body: some View {
        ZStack {
            Color.clear
                .contentShape(Rectangle())
                .onTapGesture { onTapBackground() }

            Canvas { ctx, size in
                let spacing: CGFloat = 40
                var x: CGFloat = 0
                while x <= size.width {
                    ctx.stroke(
                        Path { p in p.move(to: .init(x: x, y: 0)); p.addLine(to: .init(x: x, y: size.height)) },
                        with: .color(.white.opacity(0.06)), lineWidth: 0.5
                    )
                    x += spacing
                }
                var y: CGFloat = 0
                while y <= size.height {
                    ctx.stroke(
                        Path { p in p.move(to: .init(x: 0, y: y)); p.addLine(to: .init(x: size.width, y: y)) },
                        with: .color(.white.opacity(0.06)), lineWidth: 0.5
                    )
                    y += spacing
                }
            }
            .allowsHitTesting(false)

            // Index-based so we can project $widgets[idx] as a Binding<EditorWidget>
            ForEach(widgets.indices, id: \.self) { idx in
                WidgetHandle(
                    widget: $widgets[idx],
                    isSelected: selectedID == widgets[idx].id,
                    canvasSize: canvasSize,
                    onTap: { onTapWidget(widgets[idx].id) }
                )
            }
        }
    }
}

// MARK: - WidgetHandle

struct WidgetHandle: View {
    @Binding var widget: EditorWidget
    let isSelected: Bool
    let canvasSize: CGSize
    let onTap: () -> Void

    @GestureState private var dragOffset: CGSize = .zero

    var body: some View {
        let w = CGFloat(widget.w) * canvasSize.width
        let h = CGFloat(widget.h) * canvasSize.height

        ZStack(alignment: .bottomTrailing) {
            widgetPreview
                .frame(width: w, height: h)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(
                            isSelected ? Color.accentColor : Color.white.opacity(0.3),
                            lineWidth: isSelected ? 2 : 1
                        )
                )
                .gesture(
                    DragGesture()
                        .updating($dragOffset) { val, state, _ in state = val.translation }
                        .onEnded { val in
                            let dx = Float(val.translation.width / canvasSize.width)
                            let dy = Float(val.translation.height / canvasSize.height)
                            widget.x = clampf(widget.x + dx, lo: widget.w / 2, hi: 1 - widget.w / 2)
                            widget.y = clampf(widget.y + dy, lo: widget.h / 2, hi: 1 - widget.h / 2)
                        }
                )
                .onTapGesture { onTap() }

            if isSelected {
                resizeHandle
            }
        }
        .position(
            x: CGFloat(widget.x) * canvasSize.width + dragOffset.width,
            y: CGFloat(widget.y) * canvasSize.height + dragOffset.height
        )
        .animation(.easeInOut(duration: 0.15), value: isSelected)
    }

    private var resizeHandle: some View {
        Image(systemName: "arrow.up.left.and.arrow.down.right")
            .font(.system(size: 10, weight: .bold))
            .foregroundStyle(.white)
            .frame(width: 20, height: 20)
            .background(Color.accentColor)
            .clipShape(RoundedRectangle(cornerRadius: 4))
            .offset(x: 8, y: 8)
            .gesture(
                DragGesture()
                    .onEnded { val in
                        let dw = Float(val.translation.width / canvasSize.width)
                        let dh = Float(val.translation.height / canvasSize.height)
                        widget.w = clampf(widget.w + dw * 2, lo: 0.05, hi: 0.6)
                        widget.h = clampf(widget.h + dh * 2, lo: 0.05, hi: 0.8)
                    }
            )
    }

    @ViewBuilder
    private var widgetPreview: some View {
        switch widget.kind {
        case .button:
            Circle()
                .fill(.white.opacity(0.25))
                .overlay(Text(widget.label).font(.system(size: 11, weight: .semibold)).foregroundStyle(.white))
        case .dpad:
            RoundedRectangle(cornerRadius: 8)
                .fill(.white.opacity(0.15))
                .overlay(Image(systemName: "dpad.fill").foregroundStyle(.white.opacity(0.6)))
        case .joystick:
            Circle()
                .fill(.white.opacity(0.15))
                .overlay(Circle().fill(.white.opacity(0.4)).frame(width: 20, height: 20))
        case .trigger:
            RoundedRectangle(cornerRadius: 8)
                .fill(.white.opacity(0.15))
                .overlay(Text(widget.label).font(.system(size: 11, weight: .semibold)).foregroundStyle(.white))
        }
    }

    private func clampf(_ v: Float, lo: Float, hi: Float) -> Float { max(lo, min(hi, v)) }
}

// MARK: - BindingSheetView

struct BindingSheetView: View {
    let widget: EditorWidget
    let slotBindings: [String: PhysicalInput]
    let joystickMode: JoystickMode
    let onSave: ([String: PhysicalInput], JoystickMode?) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var workingBindings: [String: PhysicalInput] = [:]
    @State private var workingMode: JoystickMode = .GamepadStick(x: .LeftX, y: .LeftY)

    var body: some View {
        NavigationStack {
            Form {
                if widget.kind == .joystick {
                    joystickModeSection
                }
                bindingSections
            }
            .navigationTitle("Assign Controls")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .primaryAction) {
                    Button("Done") {
                        onSave(workingBindings, widget.kind == .joystick ? workingMode : nil)
                        dismiss()
                    }
                }
            }
            .onAppear {
                workingBindings = slotBindings
                workingMode = joystickMode
            }
        }
        .presentationDetents([.medium, .large])
    }

    // MARK: Joystick mode section

    @ViewBuilder
    private var joystickModeSection: some View {
        Section("Joystick Mode") {
            // Mode picker — backed by a String tag to avoid Binding<JoystickMode> complications
            Picker("Mode", selection: joystickModeTag) {
                Text("Gamepad Stick").tag("gamepad")
                Text("Mouse Look").tag("mouse")
                Text("Directional").tag("directional")
            }
            .pickerStyle(.segmented)
            .listRowBackground(Color.clear)
        }

        // Axis pickers shown only in GamepadStick mode, outside the Section
        // so they appear as separate rows below the mode picker
        if case .GamepadStick(let cx, let cy) = workingMode {
            Section("Axes") {
                axisPicker(label: "X Axis", current: cx) { newVal in
                    workingMode = .GamepadStick(x: newVal, y: cy)
                }
                axisPicker(label: "Y Axis", current: cy) { newVal in
                    workingMode = .GamepadStick(x: cx, y: newVal)
                }
            }
        }
    }

    // Extracted to avoid inline Binding<GamepadAxis> inference issues
    private func axisPicker(
        label: String,
        current: GamepadAxis,
        onSelect: @escaping (GamepadAxis) -> Void
    ) -> some View {
        let axes: [GamepadAxis] = [.LeftX, .LeftY, .RightX, .RightY]
        return Picker(label, selection: Binding(
            get: { current },
            set: { onSelect($0) }
        )) {
            ForEach(axes, id: \.self) { axis in
                Text(axis.rawValue).tag(axis)
            }
        }
    }

    private var joystickModeTag: Binding<String> {
        Binding(
            get: {
                switch workingMode {
                case .GamepadStick: return "gamepad"
                case .MouseLook:    return "mouse"
                case .Directional:  return "directional"
                }
            },
            set: { val in
                switch val {
                case "gamepad": workingMode = .GamepadStick(x: .LeftX, y: .LeftY)
                case "mouse":   workingMode = .MouseLook
                default:        workingMode = .Directional
                }
                workingBindings = workingBindings.filter { !$0.key.hasPrefix(widget.id) }
            }
        )
    }

    // MARK: Slot sections

    private var bindingSections: some View {
        ForEach(currentSlots, id: \.slot) { item in
            Section(item.label) {
                InputPickerRow(
                    current: workingBindings[item.slot],
                    onChange: { workingBindings[item.slot] = $0 }
                )
            }
        }
    }

    private var currentSlots: [(slot: String, label: String)] {
        switch widget.kind {
        case .button:
            return [(widget.id, "Input")]
        case .trigger:
            return [(widget.id, "Trigger Axis")]
        case .dpad:
            return [("up","Up"), ("down","Down"), ("left","Left"), ("right","Right")]
                .map { ("\(widget.id)_\($0.0)", $0.1) }
        case .joystick:
            switch workingMode {
            case .Directional:
                return [("up","Up"), ("down","Down"), ("left","Left"), ("right","Right")]
                    .map { ("\(widget.id)_\($0.0)", $0.1) }
            case .MouseLook:
                return [("\(widget.id)_x", "X (horizontal)"), ("\(widget.id)_y", "Y (vertical)")]
            case .GamepadStick:
                return []
            }
        }
    }
}

// MARK: - InputPickerRow

struct InputPickerRow: View {
    let current: PhysicalInput?
    let onChange: (PhysicalInput) -> Void

    @State private var showPicker = false

    var body: some View {
        Button(action: { showPicker = true }) {
            HStack {
                Text(current.map(inputDisplayName) ?? "Not assigned")
                    .foregroundStyle(current == nil ? .secondary : .primary)
                Spacer()
                Image(systemName: "chevron.right")
                    .foregroundStyle(.secondary)
                    .font(.caption)
            }
        }
        .foregroundStyle(.primary)
        .sheet(isPresented: $showPicker) {
            InputPickerSheet(current: current) { input in
                onChange(input)
                showPicker = false
            }
        }
    }
}

// MARK: - InputPickerSheet

enum InputCategory: String, CaseIterable {
    case keyboard = "Keys"
    case mouse    = "Mouse"
    case gamepad  = "Gamepad"
}

struct InputPickerSheet: View {
    let current: PhysicalInput?
    let onSelect: (PhysicalInput) -> Void

    @State private var category: InputCategory = .keyboard
    @State private var search = ""

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                Picker("Category", selection: $category) {
                    ForEach(InputCategory.allCases, id: \.self) {
                        Text($0.rawValue).tag($0)
                    }
                }
                .pickerStyle(.segmented)
                .padding()

                List(filteredItems, id: \.label) { item in
                    Button(action: { onSelect(item.input) }) {
                        HStack {
                            Text(item.label)
                            Spacer()
                            if physicalInputsEqual(item.input, current) {
                                Image(systemName: "checkmark")
                                        .foregroundStyle(Color.accentColor)
                            }
                        }
                    }
                    .foregroundStyle(.primary)
                }
                .searchable(text: $search, prompt: "Search")
            }
            .navigationTitle("Choose Input")
            .navigationBarTitleDisplayMode(.inline)
        }
        .presentationDetents([.large])
    }

    struct PickerItem { let label: String; let input: PhysicalInput }

    var allItems: [PickerItem] {
        switch category {
        case .keyboard:
            return allKeyEntries.map { PickerItem(label: $0.name, input: .Key($0.scanCode)) }
        case .mouse:
            return [
                PickerItem(label: "Left Click",   input: .MButton(0)),
                PickerItem(label: "Right Click",  input: .MButton(1)),
                PickerItem(label: "Middle Click", input: .MButton(2)),
                PickerItem(label: "Mouse X",      input: .MAxis(.X)),
                PickerItem(label: "Mouse Y",      input: .MAxis(.Y)),
            ]
        case .gamepad:
            let allButtons: [GamepadButton] = [
                .South, .East, .West, .North,
                .LBumper, .RBumper, .LStick, .RStick,
                .DPadUp, .DPadDown, .DPadLeft, .DPadRight,
                .Start, .Select
            ]
            let allAxes: [GamepadAxis] = [.LeftX, .LeftY, .RightX, .RightY]
            let allTriggers: [GamepadTrigger] = [.Left, .Right]
            let buttons = allButtons.map { PickerItem(label: $0.rawValue, input: .GPadButton($0)) }
            let axes = allAxes.map { PickerItem(label: "\($0.rawValue) (Axis)", input: .GPadAxis($0)) }
            let triggers = allTriggers.map { PickerItem(label: "\($0.rawValue) Trigger", input: .GPadTrigger($0)) }
            return buttons + axes + triggers
        }
    }

    var filteredItems: [PickerItem] {
        guard !search.isEmpty else { return allItems }
        return allItems.filter { $0.label.localizedCaseInsensitiveContains(search) }
    }
}

// MARK: - Helpers

func physicalInputsEqual(_ a: PhysicalInput, _ b: PhysicalInput?) -> Bool {
    guard let b = b else { return false }
    switch (a, b) {
    case (.Key(let x),         .Key(let y)):         return x == y
    case (.MButton(let x),     .MButton(let y)):     return x == y
    case (.MAxis(let x),       .MAxis(let y)):       return x == y
    case (.GPadButton(let x),  .GPadButton(let y)):  return x == y
    case (.GPadAxis(let x),    .GPadAxis(let y)):    return x == y
    case (.GPadTrigger(let x), .GPadTrigger(let y)): return x == y
    default: return false
    }
}

func inputDisplayName(_ input: PhysicalInput) -> String {
    switch input {
    case .Key(let sc):        return keyDisplayName(sc: sc)
    case .MButton(let b):
        let names = ["Left Click", "Right Click", "Middle Click"]
        return Int(b) < names.count ? names[Int(b)] : "Mouse Button \(b)"
    case .MAxis(let a):       return a == .X ? "Mouse X" : "Mouse Y"
    case .GPadButton(let b):  return b.rawValue
    case .GPadAxis(let a):    return "\(a.rawValue) (Axis)"
    case .GPadTrigger(let t): return "\(t.rawValue) Trigger"
    }
}

func keyDisplayName(sc: UInt16) -> String {
    allKeyEntries.first(where: { $0.scanCode == sc })?.name ?? "SC:0x\(String(sc, radix: 16))"
}

struct KeyEntry { let name: String; let scanCode: UInt16 }

// TODO: rename scanCode to usageCode
let allKeyEntries: [KeyEntry] = [
    KeyEntry(name: "A", scanCode: 0x04),
    KeyEntry(name: "B", scanCode: 0x05),
    KeyEntry(name: "C", scanCode: 0x06),
    KeyEntry(name: "D", scanCode: 0x07),
    KeyEntry(name: "E", scanCode: 0x08),
    KeyEntry(name: "F", scanCode: 0x09),
    KeyEntry(name: "G", scanCode: 0x0A),
    KeyEntry(name: "H", scanCode: 0x0B),
    KeyEntry(name: "I", scanCode: 0x0C),
    KeyEntry(name: "J", scanCode: 0x0D),
    KeyEntry(name: "K", scanCode: 0x0E),
    KeyEntry(name: "L", scanCode: 0x0F),
    KeyEntry(name: "M", scanCode: 0x10),
    KeyEntry(name: "N", scanCode: 0x11),
    KeyEntry(name: "O", scanCode: 0x12),
    KeyEntry(name: "P", scanCode: 0x13),
    KeyEntry(name: "Q", scanCode: 0x14),
    KeyEntry(name: "R", scanCode: 0x15),
    KeyEntry(name: "S", scanCode: 0x16),
    KeyEntry(name: "T", scanCode: 0x17),
    KeyEntry(name: "U", scanCode: 0x18),
    KeyEntry(name: "V", scanCode: 0x19),
    KeyEntry(name: "W", scanCode: 0x1A),
    KeyEntry(name: "X", scanCode: 0x1B),
    KeyEntry(name: "Y", scanCode: 0x1C),
    KeyEntry(name: "Z", scanCode: 0x1D),
    KeyEntry(name: "0", scanCode: 0x27),
    KeyEntry(name: "1", scanCode: 0x1E),
    KeyEntry(name: "2", scanCode: 0x1F),
    KeyEntry(name: "3", scanCode: 0x20),
    KeyEntry(name: "4", scanCode: 0x21),
    KeyEntry(name: "5", scanCode: 0x22),
    KeyEntry(name: "6", scanCode: 0x23),
    KeyEntry(name: "7", scanCode: 0x24),
    KeyEntry(name: "8", scanCode: 0x25),
    KeyEntry(name: "9", scanCode: 0x26),
    KeyEntry(name: "F1",  scanCode: 0x3A),
    KeyEntry(name: "F2",  scanCode: 0x3B),
    KeyEntry(name: "F3",  scanCode: 0x3C),
    KeyEntry(name: "F4",  scanCode: 0x3D),
    KeyEntry(name: "F5",  scanCode: 0x3E),
    KeyEntry(name: "F6",  scanCode: 0x3F),
    KeyEntry(name: "F7",  scanCode: 0x40),
    KeyEntry(name: "F8",  scanCode: 0x41),
    KeyEntry(name: "F9",  scanCode: 0x42),
    KeyEntry(name: "F10", scanCode: 0x43),
    KeyEntry(name: "Space",       scanCode: 0x2C),
    KeyEntry(name: "Enter",       scanCode: 0x28),
    KeyEntry(name: "Escape",      scanCode: 0x29),
    KeyEntry(name: "Left Shift",  scanCode: 0xE1),
    KeyEntry(name: "Left Ctrl",   scanCode: 0xE0),
    KeyEntry(name: "Left Alt",    scanCode: 0xE2),
    KeyEntry(name: "Arrow Up",    scanCode: 0x52),
    KeyEntry(name: "Arrow Down",  scanCode: 0x51),
    KeyEntry(name: "Arrow Left",  scanCode: 0x50),
    KeyEntry(name: "Arrow Right", scanCode: 0x4F),
]
