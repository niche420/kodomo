import SwiftUI

struct StreamView: View {
    let params: ConnectParams
    @StateObject private var receiver: UDPReceiver
    private var depacketizer: Depacketizer
    private var decoder: VideoDecoder
    let renderer: MetalRenderer
    let handshakeClient: HandshakeClient?
    
    var body: some View {
        MetalView(renderer: self.renderer)
            .onAppear {
                print("Depacketizer created: \(String(describing: depacketizer))")
                decoder.onFrameDecoded = { pixelBuffer in
                    print("Frame decoded: \(CVPixelBufferGetWidth(pixelBuffer))x\(CVPixelBufferGetHeight(pixelBuffer))")
                    renderer.currentPixelBuffer = pixelBuffer
                }
                receiver.onPacketReceived = { data in
                    print("Raw packet received: \(data.count) bytes")
                    let nals = depacketizer.push(data)
                    print("Received \(nals.count) NALs")
                    if !nals.isEmpty {
                        decoder.decode(nalUnits: nals)
                    }
                }
                receiver.start()
                handshakeClient?.sendReady()
            }
            .onDisappear {
                receiver.stop()
            }
    }
    
    init(params: ConnectParams, handshakeClient: HandshakeClient?) {
        self.params = params
        self.handshakeClient = handshakeClient
        _receiver = StateObject(wrappedValue: UDPReceiver(port: params.port))
        depacketizer = Depacketizer()
        decoder = VideoDecoder()
        renderer = MetalRenderer()
    }
}
