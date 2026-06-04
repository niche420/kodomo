import SwiftUI

struct StreamView: View {
    let params: ConnectParams
    let profile: GameProfile?
    @StateObject private var receiver: UDPReceiver
    private var depacketizer: Depacketizer
    private var decoder: VideoDecoder
    let renderer: MetalRenderer
    let handshakeClient: HandshakeClient?

    var body: some View {
        ZStack {
            MetalView(renderer: renderer)
            //if let profile { ControlOverlayView(profile: profile) }
        }
        .onAppear {
            decoder.onFrameDecoded = { pixelBuffer in
                renderer.currentPixelBuffer = pixelBuffer
            }
            receiver.onPacketReceived = { data in
                let nals = depacketizer.push(data)
                if !nals.isEmpty {
                    decoder.decode(nalUnits: nals)
                }
            }
            receiver.onReady = {
                handshakeClient?.sendReady()
            }
            receiver.start()
        }
        .onDisappear {
            receiver.stop()
        }
    }

    init(params: ConnectParams, handshakeClient: HandshakeClient?, profile: GameProfile?) {
        self.params = params
        self.handshakeClient = handshakeClient
        self.profile = profile
        _receiver = StateObject(wrappedValue: UDPReceiver(port: params.port))
        depacketizer = Depacketizer()
        decoder = VideoDecoder()
        renderer = MetalRenderer()
    }
}
