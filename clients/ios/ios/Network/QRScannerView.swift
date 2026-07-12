import SwiftUI
import AVFoundation

struct QRScannerView: UIViewControllerRepresentable {
    var onScanned: (ScanResult) -> Void

    struct ScanResult {
        let ip: String
        let videoPort: UInt16
        let handshakePort: UInt16
        let httpPort: UInt16
        let session: String
    }

    func makeUIViewController(context: Context) -> ScannerViewController {
        let vc = ScannerViewController()
        vc.onScanned = { string in
            guard let result = Self.parse(string) else { return }
            onScanned(result)
        }
        return vc
    }

    func updateUIViewController(_ uiViewController: ScannerViewController, context: Context) {}

    static func parse(_ string: String) -> ScanResult? {
        guard let components = URLComponents(string: string),
              components.scheme == "kodomo",
              let host = components.host,
              let port = components.port,
              let queryItems = components.queryItems else { return nil }

        let session = queryItems.first(where: { $0.name == "session" })?.value ?? ""
        let game = queryItems.first(where: { $0.name == "game" })?.value ?? ""

        guard let handshakeStr = queryItems.first(where: { $0.name == "handshake_port" })?.value,
              let handshakePort = UInt16(handshakeStr),
              let httpStr = queryItems.first(where: { $0.name == "http_port" })?.value,
              let httpPort = UInt16(httpStr) else { return nil }

        return ScanResult(
            ip: host,
            videoPort: UInt16(port),
            handshakePort: handshakePort,
            httpPort: httpPort,
            session: session
        )
    }
}

class ScannerViewController: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    var onScanned: ((String) -> Void)?
    private var captureSession: AVCaptureSession?
    private var previewLayer: AVCaptureVideoPreviewLayer?
    private var didScan = false

    override func viewDidLoad() {
        super.viewDidLoad()
        setupCamera()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        DispatchQueue.global(qos: .userInitiated).async {
            self.captureSession?.startRunning()
        }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        captureSession?.stopRunning()
    }

    private func setupCamera() {
        guard let device = AVCaptureDevice.default(for: .video),
              let input = try? AVCaptureDeviceInput(device: device) else {
            showPermissionError()
            return
        }

        let session = AVCaptureSession()
        session.addInput(input)

        let output = AVCaptureMetadataOutput()
        session.addOutput(output)
        output.setMetadataObjectsDelegate(self, queue: .main)
        output.metadataObjectTypes = [.qr]

        let preview = AVCaptureVideoPreviewLayer(session: session)
        preview.frame = view.layer.bounds
        preview.videoGravity = .resizeAspectFill
        view.layer.addSublayer(preview)
        self.previewLayer = preview
        self.captureSession = session

        DispatchQueue.global(qos: .userInitiated).async {
            session.startRunning()
        }
    }

    func metadataOutput(_ output: AVCaptureMetadataOutput,
                        didOutput metadataObjects: [AVMetadataObject],
                        from connection: AVCaptureConnection) {
        guard !didScan,
              let obj = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
              let string = obj.stringValue else { return }
        didScan = true
        captureSession?.stopRunning()
        onScanned?(string)
    }

    private func showPermissionError() {
        let label = UILabel()
        label.text = "Camera access required.\nEnable in Settings."
        label.numberOfLines = 0
        label.textAlignment = .center
        label.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: view.centerYAnchor),
        ])
    }
}
