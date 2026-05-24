#ifndef KD_FFI
#define KD_FFI

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Depacketizer Depacketizer;

typedef struct KdNalUnits {
  const uint8_t **data;
  uintptr_t *lengths;
  uintptr_t count;
} KdNalUnits;

struct Depacketizer *kd_depacketizer_create(void);

void kd_depacketizer_destroy(struct Depacketizer *raw);

struct KdNalUnits kd_depacketizer_push(struct Depacketizer *raw,
                                       const uint8_t *data,
                                       uintptr_t len);

void kd_nal_units_free(struct KdNalUnits *nal_units);

#endif  /* KD_FFI */
