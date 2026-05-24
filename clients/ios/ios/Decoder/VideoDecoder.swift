import VideoToolbox

class VideoDecoder {
    var onFrameDecoded: ((CVPixelBuffer) -> Void)?
    var formatDescription: CMVideoFormatDescription?
    var decompressionSession: VTDecompressionSession?
    private var sps: Data?
    private var pps: Data?
    
    func decode(nalUnits: [Data]) {
        for nal in nalUnits {
            guard let firstByte = nal.first else { continue }
            let nalType = firstByte & 0x1F
            switch nalType {
            case 7: // SPS
                sps = nal
                break
            case 8: // PPS
                pps = nal
                guard let sps = sps, let pps = pps else { return }
                sps.withUnsafeBytes { spsPtr in
                    pps.withUnsafeBytes { ppsPtr in
                        let parameterSetPointers = [
                            spsPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                            ppsPtr.baseAddress!.assumingMemoryBound(to: UInt8.self)
                        ]
                        let parameterSetSizes = [sps.count, pps.count]
                        var desc: CMVideoFormatDescription?
                        CMVideoFormatDescriptionCreateFromH264ParameterSets(allocator: kCFAllocatorDefault, parameterSetCount: 2, parameterSetPointers: parameterSetPointers, parameterSetSizes: parameterSetSizes, nalUnitHeaderLength: 4, formatDescriptionOut: &desc)
                        self.formatDescription = desc
                        
                        var decompressionSession: VTDecompressionSession?
                        VTDecompressionSessionCreate(allocator: kCFAllocatorDefault, formatDescription: desc!, decoderSpecification: nil, imageBufferAttributes: nil, decompressionSessionOut: &decompressionSession)
                        self.decompressionSession = decompressionSession
                    }
                }
                break
            case 5: // IDR
                decodeFrame(nal)
                break
            case 1: // P-frame
                decodeFrame(nal)
                break
            default:
                break
            }
        }
    }
    
    private func decodeFrame(_ nal: Data) {
        var nalWithHeader = Data(count: 4 + nal.count)
        let length = UInt32(nal.count).bigEndian
        withUnsafeBytes(of: length) { nalWithHeader.replaceSubrange(0..<4, with: $0) }
        nalWithHeader.replaceSubrange(4..., with: nal)
        
        var blockBuffer: CMBlockBuffer?
        let count = nalWithHeader.count
        nalWithHeader.withUnsafeMutableBytes { ptr in
            CMBlockBufferCreateWithMemoryBlock(
                allocator: kCFAllocatorDefault,
                memoryBlock: ptr.baseAddress,
                blockLength: count,
                blockAllocator: kCFAllocatorNull,
                customBlockSource: nil,
                offsetToData: 0,
                dataLength: count,
                flags: 0,
                blockBufferOut: &blockBuffer
            )
        }
        
        var sampleBuffer: CMSampleBuffer?
        var sampleSize = nalWithHeader.count
        CMSampleBufferCreateReady(
            allocator: kCFAllocatorDefault,
            dataBuffer: blockBuffer,
            formatDescription: formatDescription,
            sampleCount: 1,
            sampleTimingEntryCount: 0,
            sampleTimingArray: nil,
            sampleSizeEntryCount: 1,
            sampleSizeArray: &sampleSize,
            sampleBufferOut: &sampleBuffer
        )
        
        if let sampleBuffer = sampleBuffer, let session = decompressionSession {
            VTDecompressionSessionDecodeFrame(session, sampleBuffer: sampleBuffer, flags: ._1xRealTimePlayback, infoFlagsOut: nil, outputHandler: { status, _, imageBuffer, _, _ in
                if let imageBuffer = imageBuffer {
                    self.onFrameDecoded?(imageBuffer)
                }
            })
        }
    }
}
