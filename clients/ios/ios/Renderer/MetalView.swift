import SwiftUI
import MetalKit

struct MetalView: UIViewRepresentable {
    let renderer: MetalRenderer
    
    func makeUIView(context: Context) -> MTKView {
        let mtkView = MTKView()
        mtkView.delegate = renderer
        mtkView.preferredFramesPerSecond = 60
        mtkView.enableSetNeedsDisplay = false
        mtkView.isPaused = false
        mtkView.device = renderer.device
        return mtkView
    }
    
    func updateUIView(_ uiView: MTKView, context: Context) {}
}
