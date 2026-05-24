import VideoToolbox

class Depacketizer {
    private var ptr: OpaquePointer?
    
    init() {
        ptr = kd_depacketizer_create()
    }
    
    deinit {
        kd_depacketizer_destroy(ptr)
    }
    
    func push(_ data: Data) -> [Data] {
        let nalUnits = data.withUnsafeBytes { ptr in
            kd_depacketizer_push(self.ptr, ptr.baseAddress?.assumingMemoryBound(to: UInt8.self), UInt(data.count))
        }
        
        var result: [Data] = []
        for i in 0..<nalUnits.count {
            let ptr = nalUnits.data[Int(i)]!
            let length = nalUnits.lengths[Int(i)]
            result.append(Data(bytes: ptr, count: Int(length)))
        }
        
        var mutableNalUnits = nalUnits
        kd_nal_units_free(&mutableNalUnits)
        
        return result
    }
}
