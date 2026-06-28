import Metal
import MetalKit
import MetalFX
import CoreVideo

class MetalRenderer: NSObject, MTKViewDelegate {

    // ── Metal core ────────────────────────────────────────────────────────────

    let device: MTLDevice
    let commandQueue: MTLCommandQueue
    let textureCache: CVMetalTextureCache
    let pipelineState: MTLRenderPipelineState

    // ── Upscaling ─────────────────────────────────────────────────────────────

    private var intermediateTexture: MTLTexture?
    private var spatialScaler: MTLFXSpatialScaler?
    private var lastInputSize: CGSize = .zero

    // ── Input ─────────────────────────────────────────────────────────────────

    var currentPixelBuffer: CVPixelBuffer?
    var upscalingEnabled: Bool = true

    // ── Init ──────────────────────────────────────────────────────────────────

    override init() {
        guard let device = MTLCreateSystemDefaultDevice() else { fatalError("No Metal device") }
        self.device = device

        guard let queue = device.makeCommandQueue() else { fatalError("No command queue") }
        self.commandQueue = queue

        var cache: CVMetalTextureCache?
        CVMetalTextureCacheCreate(kCFAllocatorDefault, nil, device, nil, &cache)
        guard let cache else { fatalError("No texture cache") }
        self.textureCache = cache

        let library = device.makeDefaultLibrary()!
        let pipelineDescriptor = MTLRenderPipelineDescriptor()
        pipelineDescriptor.vertexFunction   = library.makeFunction(name: "vertexShader")
        pipelineDescriptor.fragmentFunction = library.makeFunction(name: "fragmentShader")
        pipelineDescriptor.colorAttachments[0].pixelFormat = .bgra8Unorm
        guard let pipeline = try? device.makeRenderPipelineState(descriptor: pipelineDescriptor) else {
            fatalError("No pipeline state")
        }
        self.pipelineState = pipeline
    }

    // ── MTKViewDelegate ───────────────────────────────────────────────────────

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
        spatialScaler = nil
    }

    func draw(in view: MTKView) {
        guard let pixelBuffer = currentPixelBuffer,
              let drawable = view.currentDrawable else { return }

        let inputW = CVPixelBufferGetWidth(pixelBuffer)
        let inputH = CVPixelBufferGetHeight(pixelBuffer)
        let outputSize = view.drawableSize

        guard let commandBuffer = commandQueue.makeCommandBuffer() else { return }

        let shouldUpscale = upscalingEnabled && outputSize.width > CGFloat(inputW)

        if shouldUpscale,
           let intermediate = ensureIntermediateTexture(width: inputW, height: inputH),
           let scaler = ensureSpatialScaler(inputW: inputW, inputH: inputH, outputSize: outputSize) {

            renderToTexture(pixelBuffer: pixelBuffer, target: intermediate, commandBuffer: commandBuffer)
            scaler.colorTexture  = intermediate
            scaler.outputTexture = drawable.texture
            scaler.encode(commandBuffer: commandBuffer)

        } else {
            directDraw(pixelBuffer: pixelBuffer, into: drawable, commandBuffer: commandBuffer)
        }

        commandBuffer.present(drawable)
        commandBuffer.commit()
    }

    // ── Private ───────────────────────────────────────────────────────────────

    private func ensureIntermediateTexture(width: Int, height: Int) -> MTLTexture? {
        let size = CGSize(width: width, height: height)
        guard intermediateTexture == nil || lastInputSize != size else { return intermediateTexture }

        let desc = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .bgra8Unorm, width: width, height: height, mipmapped: false)
        desc.usage = [.renderTarget, .shaderRead]
        desc.storageMode = .private
        intermediateTexture = device.makeTexture(descriptor: desc)
        lastInputSize = size
        return intermediateTexture
    }

    @discardableResult
    private func ensureSpatialScaler(inputW: Int, inputH: Int, outputSize: CGSize) -> MTLFXSpatialScaler? {
        if let existing = spatialScaler { return existing }

        let desc = MTLFXSpatialScalerDescriptor()
        desc.inputWidth          = inputW
        desc.inputHeight         = inputH
        desc.outputWidth         = Int(outputSize.width)
        desc.outputHeight        = Int(outputSize.height)
        desc.colorTextureFormat  = .bgra8Unorm
        desc.outputTextureFormat = .bgra8Unorm
        desc.colorProcessingMode = .perceptual

        spatialScaler = desc.makeSpatialScaler(device: device)
        if spatialScaler == nil {
            print("MetalFX: spatial scaler unavailable, falling back to direct draw")
        }
        return spatialScaler
    }

    private func renderToTexture(pixelBuffer: CVPixelBuffer, target: MTLTexture, commandBuffer: MTLCommandBuffer) {
        guard let (yTex, cbcrTex) = makeYCbCrTextures(from: pixelBuffer) else { return }

        let passDesc = MTLRenderPassDescriptor()
        passDesc.colorAttachments[0].texture    = target
        passDesc.colorAttachments[0].loadAction  = .dontCare
        passDesc.colorAttachments[0].storeAction = .store

        guard let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: passDesc) else { return }
        encoder.setRenderPipelineState(pipelineState)
        encoder.setFragmentTexture(yTex,    index: 0)
        encoder.setFragmentTexture(cbcrTex, index: 1)
        encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
        encoder.endEncoding()
    }

    private func directDraw(pixelBuffer: CVPixelBuffer, into drawable: CAMetalDrawable, commandBuffer: MTLCommandBuffer) {
        guard let (yTex, cbcrTex) = makeYCbCrTextures(from: pixelBuffer) else { return }

        let passDesc = MTLRenderPassDescriptor()
        passDesc.colorAttachments[0].texture    = drawable.texture
        passDesc.colorAttachments[0].loadAction  = .dontCare
        passDesc.colorAttachments[0].storeAction = .store

        guard let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: passDesc) else { return }
        encoder.setRenderPipelineState(pipelineState)
        encoder.setFragmentTexture(yTex,    index: 0)
        encoder.setFragmentTexture(cbcrTex, index: 1)
        encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
        encoder.endEncoding()
    }

    private func makeYCbCrTextures(from pixelBuffer: CVPixelBuffer) -> (MTLTexture, MTLTexture)? {
        let w = CVPixelBufferGetWidth(pixelBuffer)
        let h = CVPixelBufferGetHeight(pixelBuffer)

        var yRef: CVMetalTexture?
        CVMetalTextureCacheCreateTextureFromImage(
            kCFAllocatorDefault, textureCache, pixelBuffer, nil, .r8Unorm, w, h, 0, &yRef)
        guard let yRef, let yTex = CVMetalTextureGetTexture(yRef) else { return nil }

        var cbcrRef: CVMetalTexture?
        CVMetalTextureCacheCreateTextureFromImage(
            kCFAllocatorDefault, textureCache, pixelBuffer, nil, .rg8Unorm, w / 2, h / 2, 1, &cbcrRef)
        guard let cbcrRef, let cbcrTex = CVMetalTextureGetTexture(cbcrRef) else { return nil }

        return (yTex, cbcrTex)
    }
}
