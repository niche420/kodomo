package com.kodomo

/**
 * JNI interface to native C++ client
 */
class NativeClient {

    companion object {
        init {
            System.loadLibrary("kodomo-android")
        }
    }

    /**
     * Initialize the native client
     * @param serverAddress Server IP or hostname
     * @param width Screen width
     * @param height Screen height
     * @param useTailscale Enable Tailscale hostname resolution
     * @param tailscaleHostname Tailscale hostname (if enabled)
     * @return true if initialized successfully
     */
    external fun nativeInit(
        serverAddress: String,
        width: Int,
        height: Int,
        useTailscale: Boolean,
        tailscaleHostname: String?
    ): Boolean

    /**
     * Connect to the server
     * @return true if connected successfully
     */
    external fun nativeConnect(): Boolean

    /**
     * Update the client (call at ~60Hz from rendering thread)
     * @return true if still running
     */
    external fun nativeUpdate(): Boolean

    /**
     * Disconnect from server
     */
    external fun nativeDisconnect()

    /**
     * Destroy native resources
     */
    external fun nativeDestroy()

    /**
     * Handle SDL event
     * @param eventType SDL event type
     * @param x X coordinate (for mouse events)
     * @param y Y coordinate (for mouse events)
     */
    external fun nativeHandleEvent(eventType: Int, x: Float, y: Float)

    /**
     * Handle touch down event
     */
    external fun nativeTouchDown(x: Float, y: Float, fingerId: Int)

    /**
     * Handle touch up event
     */
    external fun nativeTouchUp(x: Float, y: Float, fingerId: Int)

    /**
     * Handle touch move event
     */
    external fun nativeTouchMove(x: Float, y: Float, fingerId: Int)

    /**
     * Lifecycle: activity paused
     */
    external fun nativeOnPause()

    /**
     * Lifecycle: activity resumed
     */
    external fun nativeOnResume()

    /**
     * Lifecycle: low memory warning
     */
    external fun nativeOnLowMemory()

    /**
     * Get current statistics as JSON string
     * @return JSON string with stats
     */
    external fun nativeGetStats(): String
}