#ifndef KODOMO_H
#define KODOMO_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Error codes
 */
typedef enum StreamError {
  Success = 0,
  InitError = 1,
  NotRunning = 2,
  AlreadyRunning = 3,
  InvalidConfig = 4,
  CaptureError = 5,
  EncodingError = 6,
  NetworkError = 7,
  InputError = 8,
  NullPointer = 9,
} StreamError;

/**
 * Video codec
 */
typedef enum StreamCodec {
  H264 = 0,
  H265 = 1,
  VP9 = 2,
} StreamCodec;

/**
 * Encoder preset
 */
typedef enum StreamPreset {
  UltraFast = 0,
  SuperFast = 1,
  VeryFast = 2,
  Faster = 3,
  Fast = 4,
  Medium = 5,
  Slow = 6,
} StreamPreset;

/**
 * Transport type
 */
typedef enum StreamTransport {
  WebRTC = 0,
  UDP = 1,
} StreamTransport;

typedef struct Option_FrameCallback Option_FrameCallback;

/**
 * Opaque handle to the streaming engine
 */
typedef struct StreamHandle StreamHandle;

/**
 * Configuration struct
 */
typedef struct StreamingConfig {
  unsigned int width;
  unsigned int height;
  unsigned int fps;
  unsigned int bitrate_kbps;
  enum StreamCodec codec;
  enum StreamPreset preset;
  int hw_accel;
  enum StreamTransport transport;
  unsigned int port;
  int enable_keyboard;
  int enable_mouse;
  int enable_gamepad;
} StreamingConfig;

/**
 * Statistics
 */
typedef struct StreamStats {
  uint64_t frames_captured;
  uint64_t frames_encoded;
  uint64_t frames_sent;
  uint64_t frames_dropped;
  uint64_t bytes_sent;
  double average_fps;
  double average_bitrate_kbps;
} StreamStats;

/**
 * Initialize the streaming library (call once at startup)
 */
void stream_init(void);

/**
 * Create a new streaming engine instance
 *
 * # Safety
 * - config must be a valid pointer
 * - out_handle must be a valid pointer to store the result
 * - The returned handle must be freed with stream_destroy()
 */
enum StreamError stream_create(const struct StreamingConfig *config,
                               struct StreamHandle **out_handle);

/**
 * Get default configuration
 */
struct StreamingConfig stream_get_default_config(void);

/**
 * Start the streaming engine
 *
 * # Safety
 * - handle must be a valid pointer obtained from stream_create()
 */
enum StreamError stream_start(struct StreamHandle *handle);

/**
 * Stop the streaming engine
 *
 * # Safety
 * - handle must be a valid pointer
 */
enum StreamError stream_stop(struct StreamHandle *handle);

/**
 * Check if the engine is running
 *
 * # Safety
 * - handle must be a valid pointer
 */
int stream_is_running(const struct StreamHandle *handle);

/**
 * Update configuration (can be called while running for dynamic changes)
 *
 * # Safety
 * - handle must be a valid pointer
 * - config must be a valid pointer
 */
enum StreamError stream_update_config(struct StreamHandle *handle,
                                      const struct StreamingConfig *config);

/**
 * Get statistics
 *
 * # Safety
 * - handle must be a valid pointer
 * - out_stats must be a valid pointer
 */
enum StreamError stream_get_stats(const struct StreamHandle *handle, struct StreamStats *out_stats);

/**
 * Destroy the streaming engine and free resources
 *
 * # Safety
 * - handle must be a valid pointer obtained from stream_create()
 * - handle must not be used after this call
 */
void stream_destroy(struct StreamHandle *handle);

/**
 * Get error message for an error code
 *
 * # Safety
 * - The returned string is static and must not be freed
 */
const char *stream_error_string(enum StreamError error);

/**
 * Get library version
 *
 * # Safety
 * - The returned string is static and must not be freed
 */
const char *stream_version(void);

/**
 * List available video encoders
 *
 * # Safety
 * - out_count must be a valid pointer
 * - Returns array of strings that must be freed with stream_free_string_array()
 */
char **stream_list_encoders(unsigned int *out_count);

/**
 * Free string array returned by stream_list_encoders()
 *
 * # Safety
 * - array must be obtained from stream_list_encoders()
 * - count must match the count returned by stream_list_encoders()
 */
void stream_free_string_array(char **array, unsigned int count);

/**
 * Send keyboard event to the system
 *
 * # Safety
 * - key_code: Virtual key code (Windows VK_* compatible)
 * - is_pressed: 1 for key down, 0 for key up
 */
enum StreamError stream_inject_keyboard(unsigned int key_code, int is_pressed);

/**
 * Send mouse event to the system
 *
 * # Safety
 * - x, y: Mouse coordinates
 * - button: Mouse button (0=none, 1=left, 2=right, 3=middle)
 * - is_pressed: 1 for button down, 0 for button up
 */
enum StreamError stream_inject_mouse(int x, int y, int button, int is_pressed);

/**
 * Set callback for captured frames (useful for debugging/recording)
 *
 * # Safety
 * - handle must be a valid pointer
 * - callback can be null to disable
 */
enum StreamError stream_set_frame_callback(struct StreamHandle *handle,
                                           struct Option_FrameCallback callback,
                                           void *user_data);

#endif  /* KODOMO_H */
