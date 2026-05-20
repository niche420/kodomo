# Kodomo RTP/H.264 Reference

## NAL Unit
```
[header: 1 byte][data: N bytes]
```

## NAL Header Byte
```
Bit:  7    6    5    4    3    2    1    0
      F   NRI  NRI   T    T    T    T    T
```
| Field | Bits | Description |
|-------|------|-------------|
| F | 7 | Forbidden zero bit, always 0 |
| NRI | 6-5 | Network reference indicator. 0=droppable, 3=critical |
| T | 4-0 | NAL type. 1=P-frame, 5=IDR, 7=SPS, 8=PPS, 24=STAP-A, 28=FU-A |

---

## RTP Header (12 bytes)
```
Bit:  7    6    5    4    3    2    1    0
      V    V    P    X   CC   CC   CC   CC   byte 0
      M   PT   PT   PT   PT   PT   PT   PT   byte 1
     [sequence number: bytes 2-3            ]
     [timestamp:       bytes 4-7            ]
     [ssrc:            bytes 8-11           ]
```
| Field | Bits | Description |
|-------|------|-------------|
| V | 7-6 of byte 0 | Version, always 2 |
| P | 5 of byte 0 | Padding, always 0 |
| X | 4 of byte 0 | Extension, always 0 |
| CC | 3-0 of byte 0 | CSRC count, always 0 |
| M | 7 of byte 1 | Marker bit, set on last packet of access unit |
| PT | 6-0 of byte 1 | Payload type, 96 for H.264 |
| Sequence number | bytes 2-3 | Increments per packet, wraps at 2¹⁶, big-endian |
| Timestamp | bytes 4-7 | 90kHz clock ticks, same for all packets in a frame, big-endian |
| SSRC | bytes 8-11 | Stream identifier, big-endian |

---

## Single NAL RTP Payload
```
[NAL header][NAL data...]
```
Used when the NAL unit fits within the MTU (1400 bytes).

---

## STAP-A RTP Payload
```
[0x18][size: 2 bytes big-endian][NAL data][size: 2 bytes big-endian][NAL data]...
```
| Field | Description |
|-------|-------------|
| 0x18 | STAP-A type byte, decimal 24 |
| size | Length of following NAL in big-endian |
| NAL data | Raw NAL unit including its header byte |

Used to bundle SPS + PPS into one packet before every IDR frame.

---

## FU-A RTP Payload
```
[FU indicator][FU header][fragment data...]
```

### FU Indicator Byte
```
Bit:  7    6    5    4    3    2    1    0
      F   NRI  NRI   1    1    1    0    0
```
| Field | Bits | Description |
|-------|------|-------------|
| F | 7 | Copied from original NAL header |
| NRI | 6-5 | Copied from original NAL header |
| Type | 4-0 | Always 28 (FU-A) |

### FU Header Byte
```
Bit:  7    6    5    4    3    2    1    0
      S    E    R    T    T    T    T    T
```
| Field | Bits | Description |
|-------|------|-------------|
| S | 7 | Start flag, 1 on first fragment only |
| E | 6 | End flag, 1 on last fragment only |
| R | 5 | Reserved, always 0 |
| T | 4-0 | Original NAL type copied from NAL header |

Used when a NAL unit exceeds the MTU and must be split across multiple packets.