use kd_shared::rtp::packetizer::Depacketizer;
use kd_shared::rtp::RtpPacket;

#[unsafe(no_mangle)]
extern "C" fn kd_depacketizer_create() -> *mut Depacketizer {
    let tizer = Box::new(Depacketizer::new());
    Box::into_raw(tizer)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kd_depacketizer_destroy(raw: *mut Depacketizer) {
    let _ = Box::from_raw(raw);
}

#[repr(C)]
struct KdNalUnits {
    data: *mut *const u8,
    lengths: *mut usize,
    count: usize
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kd_depacketizer_push(raw: *mut Depacketizer, data: *const u8, len: usize) -> KdNalUnits {
    let tizer = &mut *raw;
    let packet_data = std::slice::from_raw_parts(data, len);
    let packet = RtpPacket::decode(packet_data).unwrap();
    let nals = tizer.push(&packet).unwrap().unwrap();
    let count = nals.len();
    let mut nal_ptrs = Vec::with_capacity(count);
    let mut lengths = Vec::with_capacity(count);
    nals.into_iter().for_each(|nal| {
        let length = nal.len();
        lengths.push(length);
        let ptr = Box::leak(nal.into_boxed_slice());
        nal_ptrs.push(ptr.as_ptr());
    });

    KdNalUnits {
        data: Box::leak(nal_ptrs.into_boxed_slice()).as_mut_ptr(),
        lengths: Box::leak(lengths.into_boxed_slice()).as_mut_ptr(),
        count
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kd_nal_units_free(nal_units: *mut KdNalUnits) {
    let units = nal_units.as_mut().unwrap();
    let nal_slices = Box::from_raw(std::ptr::slice_from_raw_parts_mut(units.data, units.count));
    let lengths = Box::from_raw(std::ptr::slice_from_raw_parts_mut(units.lengths, units.count));
    for (ptr, length) in nal_slices.iter().zip(lengths.iter()) {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(*ptr as *mut u8, *length));
    }
}