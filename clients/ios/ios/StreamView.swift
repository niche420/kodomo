import SwiftUI

struct StreamView: View {
    let params: ConnectParams
    @StateObject private var receiver: UDPReceiver
    private var depacketizer: Depacketizer
    private var decoder: VideoDecoder
    let renderer: MetalRenderer
    
    var body: some View {
        MetalView(renderer: self.renderer)
            .onAppear {
                print("Depacketizer created: \(String(describing: depacketizer))")
                decoder.onFrameDecoded = { pixelBuffer in
                    print("Frame decoded: \(CVPixelBufferGetWidth(pixelBuffer))x\(CVPixelBufferGetHeight(pixelBuffer))")
                    renderer.currentPixelBuffer = pixelBuffer
                }
                receiver.onPacketReceived = { data in
                    let nals = depacketizer.push(data)
                    if !nals.isEmpty {
                        decoder.decode(nalUnits: nals)
                    }
                }
                receiver.start()
            }
            .onDisappear {
                receiver.stop()
            }
    }
    
    init(params: ConnectParams) {
        self.params = params
        _receiver = StateObject(wrappedValue: UDPReceiver(port: params.port))
        depacketizer = Depacketizer()
        decoder = VideoDecoder()
        renderer = MetalRenderer()
    }
}

#Preview {
    StreamView(params: ConnectParams(
        ip: "127.0.0.1",
        port: 5000,
        session: "1234567890abcdef",
        game: "Yakuza 3",
    ))
}
